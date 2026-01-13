use crate::compiler::cancellation::CancellableWorld;
use crate::ipc::events::{emit_event, BackendEvent};
use crate::ipc::{TypstCompileEvent, TypstDiagnosticSeverity, TypstDocument, TypstSourceDiagnostic};
use crate::project::ProjectManager;
use log::{debug, error};
#[allow(unused_imports)]
use serde::Serialize;
use siphasher::sip128::{Hasher128, SipHasher};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use tauri::{Manager, Runtime};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use typst::diag::Severity;
use typst::World;

#[derive(Clone, Debug)]
pub struct CompileRequest {
    pub path: PathBuf,
    pub content: String,
    pub main_path: Option<PathBuf>,
    pub request_id: u64,
    pub window_label: String,
}

pub struct Compiler<R: Runtime> {
    tx: watch::Sender<Option<CompileRequest>>,
    _handle: JoinHandle<()>,
    _marker: std::marker::PhantomData<R>,
}

unsafe impl<R: Runtime> Send for Compiler<R> {}
unsafe impl<R: Runtime> Sync for Compiler<R> {}

impl<R: Runtime> Compiler<R> {
    pub fn new(project_manager: Arc<ProjectManager<R>>, app: tauri::AppHandle<R>) -> Self {
        let (tx, mut rx) = watch::channel::<Option<CompileRequest>>(None);

        let handle = tokio::spawn(async move {
            let mut current_cancel_token: Option<Arc<AtomicBool>> = None;
            let mut _current_job: Option<JoinHandle<()>> = None;

            while rx.changed().await.is_ok() {
                if let Some(token) = &current_cancel_token {
                    token.store(true, Ordering::Relaxed);
                }

                let request = {
                    let borrow = rx.borrow_and_update();
                    borrow.clone()
                };

                if let Some(req) = request {
                    let token = Arc::new(AtomicBool::new(false));
                    current_cancel_token = Some(token.clone());
                    
                    let pm = project_manager.clone();
                    let window = app.get_webview_window(&req.window_label);

                    if let Some(window) = window {
                        let inner_token = token.clone();
                        _current_job = Some(tokio::task::spawn_blocking(move || {
                             compile_job(pm, window, req, inner_token);
                        }));
                    }
                }
            }
        });

        Self {
            tx,
            _handle: handle,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn update(&self, req: CompileRequest) {
        let _ = self.tx.send(Some(req));
    }
}

fn compile_job<R: Runtime>(
    project_manager: Arc<ProjectManager<R>>,
    window: tauri::WebviewWindow<R>,
    req: CompileRequest,
    token: Arc<AtomicBool>,
) {
    if token.load(Ordering::Relaxed) { return; }

    let project_opt = project_manager.get_project(&window);
    if project_opt.is_none() {
        return;
    }
    let project = project_opt.unwrap();

    if token.load(Ordering::Relaxed) { return; }
    let mut world_guard = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });

    if token.load(Ordering::Relaxed) { return; }

    let update_res = world_guard.slot_update(&req.path, Some(req.content.clone()));
    if let Err(e) = update_res {
        error!("Failed to update slot: {:?}", e);
        return;
    }

    let main_to_set = req.main_path.as_ref().unwrap_or(&req.path);
    world_guard.set_main_path(typst::syntax::VirtualPath::new(main_to_set));
    
    if !world_guard.is_main_set() {
        let config = project.config.read().unwrap();
        if config.apply_main(&project, &mut world_guard).is_err() {
            return;
        }
    }

    let cancellable_world = CancellableWorld::new(&world_guard, token.clone());

    let result = typst::compile::<typst::layout::PagedDocument>(&cancellable_world);
    
    drop(world_guard);

    let old_id = project.current_compile_request_id.fetch_max(req.request_id, Ordering::SeqCst);
    if req.request_id < old_id {
        return;
    }

    match result.output {
        Ok(doc) => {
             let pages = doc.pages.len();
             let mut hasher = SipHasher::new();
             for page in &doc.pages {
                 page.frame.hash(&mut hasher);
             }
             let hash = hex::encode(hasher.finish128().as_bytes());

             let first_page = &doc.pages[0];
             let width = first_page.frame.width();
             let height = first_page.frame.height();
             
             let max_prerender = std::cmp::min(pages, 10);
             let page_svgs: Vec<String> = (0..max_prerender)
                 .map(|i| {
                     let page = &doc.pages[i];
                     let mut renderer = project.renderer.lock().unwrap_or_else(|e| e.into_inner());
                     let (svg, _) = renderer.render_page(i, page);
                     svg
                 })
                 .collect();

             project.cache.write().unwrap().document = Some(doc);
            
             emit_event(&window, BackendEvent::Compile(TypstCompileEvent {
                 document: Some(TypstDocument {
                     pages,
                     hash,
                     width: width.to_pt(),
                     height: height.to_pt(),
                     page_svgs,
                 }),
                 diagnostics: None,
             }));
        }
        Err(diagnostics) => {
            let world_guard = project.world.lock().unwrap_or_else(|e| {
                log::warn!("Project world mutex poisoned, recovering: {}", e);
                e.into_inner()
            });
            
            let vpath = typst::syntax::VirtualPath::new(&req.path);
            let id = typst::syntax::FileId::new(None, vpath);
            
            let source_res = world_guard.source(id);
            let mapped_diagnostics = if let Ok(source) = source_res {
                diagnostics.iter()
                    .filter(|d| d.span.id() == Some(id))
                    .filter_map(|d| {
                         let span = source.find(d.span)?;
                         let range = span.range();
                         let start = req.content[..range.start].chars().count();
                         let size = req.content[range.start..range.end].chars().count();
                         
                         Some(TypstSourceDiagnostic {
                             range: start..start + size,
                             severity: match d.severity {
                                 Severity::Error => TypstDiagnosticSeverity::Error,
                                 Severity::Warning => TypstDiagnosticSeverity::Warning,
                             },
                             message: d.message.to_string(),
                             hints: d.hints.iter().map(|h| h.to_string()).collect(),
                         })
                    })
                    .collect()
            } else {
                vec![]
            };

            emit_event(&window, BackendEvent::Compile(TypstCompileEvent {
                document: None,
                diagnostics: Some(mapped_diagnostics),
            }));
        }
    }
}
