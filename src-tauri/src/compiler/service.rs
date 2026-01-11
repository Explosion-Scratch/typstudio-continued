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
            // storing the handle just to keep it alive or await if needed, 
            // but we mostly rely on the token for cancellation.
            let mut _current_job: Option<JoinHandle<()>> = None;

            while rx.changed().await.is_ok() {
                // 1. Signal cancellation to the running job
                if let Some(token) = &current_cancel_token {
                    token.store(true, Ordering::Relaxed);
                }

                // 2. Grab the latest request
                let request = {
                    let borrow = rx.borrow_and_update();
                    borrow.clone()
                };

                if let Some(req) = request {
                    let token = Arc::new(AtomicBool::new(false));
                    current_cancel_token = Some(token.clone());
                    
                    let pm = project_manager.clone();
                    // We need a window handle to emit events. 
                    // We can resolve it from the app handle using the label.
                    let window = app.get_window(&req.window_label);

                    if let Some(window) = window {
                        let inner_token = token.clone();
                        _current_job = Some(tokio::task::spawn_blocking(move || {
                             compile_job(pm, window, req, inner_token);
                        }));
                    } else {
                        debug!("Could not find window for compilation request: {}", req.window_label);
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
    window: tauri::Window<R>,
    req: CompileRequest,
    token: Arc<AtomicBool>,
) {
    if token.load(Ordering::Relaxed) { return; }

    let project_opt = project_manager.get_project(&window);
    if project_opt.is_none() {
        return;
    }
    let project = project_opt.unwrap();

    // Acquire lock on world
    if token.load(Ordering::Relaxed) { return; }
    let mut world_guard = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });

    if token.load(Ordering::Relaxed) { return; }

    // Update source in world
    let update_res = world_guard.slot_update(&req.path, Some(req.content.clone()));
    if let Err(e) = update_res {
        error!("Failed to update slot: {:?}", e);
        return;
    }

    world_guard.set_main_path(typst::syntax::VirtualPath::new(&req.path));
    
    // Ensure main is ready
    if !world_guard.is_main_set() {
        let config = project.config.read().unwrap();
        if config.apply_main(&project, &mut world_guard).is_err() {
            debug!("skipped compilation for (main not set)");
            return;
        }
    }

    // Wrap in CancellableWorld
    let cancellable_world = CancellableWorld::new(&world_guard, token.clone());

    // Compile
    let result = typst::compile::<typst::layout::PagedDocument>(&cancellable_world);
    
    // Release lock early if possible, though we might need it for introspection 
    // if we were to access source maps securely, but usually introspection relies on the document + source text which we have.
    drop(world_guard);

    if token.load(Ordering::Relaxed) {
        debug!("Compilation aborted after typst::compile (request_id: {})", req.request_id);
        return;
    }

    match result.output {
        Ok(doc) => {
             // Success Introspection
             let pages = doc.pages.len();
             let mut hasher = SipHasher::new();
             for page in &doc.pages {
                 page.frame.hash(&mut hasher);
             }
             let hash = hex::encode(hasher.finish128().as_bytes());

             let first_page = &doc.pages[0];
             let width = first_page.frame.width();
             let height = first_page.frame.height();

             // Update Cache
             project.cache.write().unwrap().document = Some(doc);
            
             emit_event(&window, BackendEvent::Compile(TypstCompileEvent {
                 document: Some(TypstDocument {
                     pages,
                     hash,
                     width: width.to_pt(),
                     height: height.to_pt(),
                 }),
                 diagnostics: None,
             }));
             debug!("Compilation success emitted (request_id: {})", req.request_id);
        }
        Err(diagnostics) => {
            // Error Introspection
            // We need to map diagnostics to source locations.
            // We need the world again to look up sources? 
            // Yes, standard way: world.source(id). But we dropped the lock.
            // We can re-acquire it. It's safe.
            
            let world_guard = project.world.lock().unwrap_or_else(|e| {
                log::warn!("Project world mutex poisoned, recovering: {}", e);
                e.into_inner()
            });
            
            // We need to resolve the source ID for the main file or where error happened?
            // Usually we filter by the source we just edited or return all?
            // The original code filtered by `source_id` of the edited file. Let's try to do that.
            // We know `req.path` corresponds to a source.
            
            // We can get the source from the world for the current path
            // But `slot_update` gave us a `source_id` earlier. We didn't keep it.
            // Let's resolve it again or assume we want diagnostics for the file we edited.
            
            // Actually, we should probably output all diagnostics or just the ones for this file. 
            // The original code did:
            // let source = world.source(source_id);
            // ... filter(|d| d.span.id() == Some(source_id))
            
            // Let's re-resolve ID.
            let vpath = typst::syntax::VirtualPath::new(&req.path);
            let id = typst::syntax::FileId::new(None, vpath); // Assuming package is None for now for simple project files
            
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
             debug!("Compilation diagnostics emitted (request_id: {})", req.request_id);
        }
    }
}
