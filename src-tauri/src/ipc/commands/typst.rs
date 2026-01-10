use super::{Error, Result};
use crate::ipc::commands::project;
use crate::ipc::model::TypstRenderResponse;
use crate::ipc::{
    TypstCompileEvent, TypstDiagnosticSeverity, TypstDocument, TypstSourceDiagnostic,
};
use crate::project::ProjectManager;
use log::debug;
use serde::Serialize;
use serde_repr::Serialize_repr;
use siphasher::sip128::{Hasher128, SipHasher};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tauri::Runtime;
use typst::diag::Severity;
use typst::World;
use typst_ide::{Completion, CompletionKind};

#[derive(Serialize, Debug)]
pub struct TypstJump {
    filepath: String,
    start: Option<(usize, usize)>, // line, column
    end: Option<(usize, usize)>,
}

#[derive(Serialize_repr, Debug)]
#[repr(u8)]
pub enum TypstCompletionKind {
    Syntax = 1,
    Function = 2,
    Parameter = 3,
    Constant = 4,
    Symbol = 5,
    Type = 6,
}

#[derive(Serialize, Debug)]
pub struct TypstCompletion {
    kind: TypstCompletionKind,
    label: String,
    apply: Option<String>,
    detail: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TypstCompleteResponse {
    offset: usize,
    completions: Vec<TypstCompletion>,
}

impl From<Completion> for TypstCompletion {
    fn from(value: Completion) -> Self {
        Self {
            kind: match value.kind {
                CompletionKind::Syntax => TypstCompletionKind::Syntax,
                CompletionKind::Func => TypstCompletionKind::Function,
                CompletionKind::Param => TypstCompletionKind::Parameter,
                CompletionKind::Constant => TypstCompletionKind::Constant,
                CompletionKind::Symbol(_) => TypstCompletionKind::Symbol,
                CompletionKind::Type => TypstCompletionKind::Type,
                _ => TypstCompletionKind::Syntax,
            },
            label: value.label.to_string(),
            apply: value.apply.map(|s| s.to_string()),
            detail: value.detail.map(|s| s.to_string()),
        }
    }
}

#[tauri::command]
pub async fn typst_compile<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    request_id: u64,
) -> Result<()> {
    let project = project(&window, &project_manager)?;

    project.current_compile_request_id.store(request_id, Ordering::SeqCst);

    let mut world = project.world.lock().unwrap();
    let source_id = world
        .slot_update(&path, Some(content.clone()))
        .map_err(Into::<Error>::into)?;

    // Set the processed file as main
    world.set_main_path(typst::syntax::VirtualPath::new(&path));

    if !world.is_main_set() {
        let config = project.config.read().unwrap();
        if config.apply_main(&project, &mut world).is_err() {
            debug!("skipped compilation for {:?} (main not set)", project);
            return Ok(());
        }
    }

    debug!("compiling {:?}: {:?} (request_id: {})", path, project, request_id);
    let now = Instant::now();
    let result = typst::compile::<typst::layout::PagedDocument>(&*world);
    match result.output {
        Ok(doc) => {
            let current_request = project.current_compile_request_id.load(Ordering::SeqCst);
            if current_request != request_id {
                debug!("skipping stale compile result for request {} (current: {})", request_id, current_request);
                return Ok(());
            }

            let elapsed = now.elapsed();
            debug!(
                "compilation succeeded for {:?} in {:?} ms",
                project,
                elapsed.as_millis()
            );

            let pages = doc.pages.len();

            let mut hasher = SipHasher::new();
            for page in &doc.pages {
                page.frame.hash(&mut hasher);
            }
            let hash = hex::encode(hasher.finish128().as_bytes());

            let first_page = &doc.pages[0];
            let width = first_page.frame.width();
            let height = first_page.frame.height();

            project.cache.write().unwrap().document = Some(doc);

            let _ = window.emit(
                "typst_compile",
                TypstCompileEvent {
                    document: Some(TypstDocument {
                        pages,
                        hash,
                        width: width.to_pt(),
                        height: height.to_pt(),
                    }),
                    diagnostics: None,
                },
            );
        }
        Err(diagnostics) => {
            let current_request = project.current_compile_request_id.load(Ordering::SeqCst);
            if current_request != request_id {
                debug!("skipping stale compile error for request {} (current: {})", request_id, current_request);
                return Ok(());
            }

            debug!(
                "compilation failed with {:?} diagnostics",
                diagnostics.len()
            );

            let source = world.source(source_id);
            let diagnostics: Vec<TypstSourceDiagnostic> = match source {
                Ok(source) => diagnostics
                    .iter()
                    .filter(|d| d.span.id() == Some(source_id))
                    .filter_map(|d| {
                        let span = source.find(d.span)?;
                        let range = span.range();
                        let start = content[..range.start].chars().count();
                        let size = content[range.start..range.end].chars().count();

                        let message = d.message.to_string();
                        Some(TypstSourceDiagnostic {
                            range: start..start + size,
                            severity: match d.severity {
                                Severity::Error => TypstDiagnosticSeverity::Error,
                                Severity::Warning => TypstDiagnosticSeverity::Warning,
                            },
                            message,
                            hints: d.hints.iter().map(|hint| hint.to_string()).collect(),
                        })
                    })
                    .collect(),
                Err(_) => vec![],
            };

            let _ = window.emit(
                "typst_compile",
                TypstCompileEvent {
                    document: None,
                    diagnostics: Some(diagnostics),
                },
            );
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn typst_render<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    page: usize,
    scale: f32,
    nonce: u32,
) -> Result<TypstRenderResponse> {
    debug!("rendering page {} @{}x", page, scale);
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    if let Some(p) = cache.document.as_ref().and_then(|doc| doc.pages.get(page)) {
        let now = Instant::now();
        
        let svg = typst_svg::svg(p);
        let elapsed = now.elapsed();
        debug!(
            "SVG rendering complete for page {} in {} ms",
            page,
            elapsed.as_millis()
        );
        
        let width = (p.frame.width().to_pt() * scale as f64) as u32;
        let height = (p.frame.height().to_pt() * scale as f64) as u32;
        
        return Ok(TypstRenderResponse {
            image: svg,
            width,
            height,
            nonce,
        });
    }

    Err(Error::Unknown)
}

#[tauri::command]
pub async fn typst_autocomplete<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    offset: usize,
    explicit: bool,
) -> Result<TypstCompleteResponse> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap();

    let offset = content
        .char_indices()
        .nth(offset)
        .map(|a| a.0)
        .unwrap_or(content.len());

    let source_id = world
        .slot_update(&*path, Some(content.clone()))
        .map_err(Into::<Error>::into)?;

    let source = world.source(source_id).map_err(Into::<Error>::into)?;

    let (completed_offset, completions) =
        typst_ide::autocomplete(&*world, None, &source, offset, explicit)
            .ok_or_else(|| Error::Unknown)?;

    let completed_char_offset = content[..completed_offset].chars().count();
    Ok(TypstCompleteResponse {
        offset: completed_char_offset,
        completions: completions.into_iter().map(TypstCompletion::from).collect(),
    })
}

#[tauri::command]
pub async fn typst_jump<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    page: usize,
    x: f64,
    y: f64,
) -> Result<Option<TypstJump>> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap();
    let cache = project.cache.read().unwrap();

    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;
    let page_doc = doc.pages.get(page).ok_or(Error::Unknown)?;

    let point = typst::layout::Point::new(
        typst::layout::Abs::pt(x),
        typst::layout::Abs::pt(y)
    );

    let jump = typst_ide::jump_from_click(&*world, doc, &page_doc.frame, point);

    let (source_id, offset) = match jump {
        Some(typst_ide::Jump::File(id, offset)) => (id, offset),
        _ => return Ok(None),
    };

    let source = world.source(source_id).map_err(Into::<Error>::into)?;
    
    // In Typst 0.14, Source::lines() returns a Lines struct which has conversion methods.
    let lines = source.lines();
    let line = lines.byte_to_line(offset).ok_or(Error::Unknown)?;
    let column = lines.byte_to_column(offset).ok_or(Error::Unknown)?;

    // Get relative path from project root
    let path = source.id().vpath().as_rootless_path().to_string_lossy().to_string();
    let filepath = if path.starts_with("/") { path } else { format!("/{}", path) };

    Ok(Some(TypstJump {
        filepath,
        start: Some((line + 1, column + 1)),
        end: Some((line + 1, column + 1)),
    }))
}

#[derive(Serialize, Debug)]
pub struct TypstDocumentPosition {
    page: usize,
    x: f64,
    y: f64,
}

#[tauri::command]
pub async fn typst_jump_from_cursor<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    offset: usize,
) -> Result<Option<TypstDocumentPosition>> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap();
    let cache = project.cache.read().unwrap();

    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;

    let byte_offset = content
        .char_indices()
        .nth(offset)
        .map(|a| a.0)
        .unwrap_or(content.len());

    let source_id = world
        .slot_update(&*path, Some(content.clone()))
        .map_err(Into::<Error>::into)?;

    let source = world.source(source_id).map_err(Into::<Error>::into)?;

    let positions = typst_ide::jump_from_cursor(doc, &source, byte_offset);

    if let Some(position) = positions.first() {
        Ok(Some(TypstDocumentPosition {
            page: position.page.get().saturating_sub(1),
            x: position.point.x.to_pt(),
            y: position.point.y.to_pt(),
        }))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Debug)]
pub struct InstalledPackage {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

fn get_package_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::cache_dir().map(|p| p.join("typst").join("packages"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CACHE_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|p| p.join(".cache")))
            .map(|p| p.join("typst").join("packages"))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::cache_dir().map(|p| p.join("typst").join("packages"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[tauri::command]
pub async fn typst_list_packages() -> Result<Vec<InstalledPackage>> {
    let cache_dir = get_package_cache_dir().ok_or(Error::Unknown)?;
    let mut packages = Vec::new();

    if !cache_dir.exists() {
        return Ok(packages);
    }

    for namespace_entry in std::fs::read_dir(&cache_dir).map_err(Into::<Error>::into)? {
        let namespace_entry = namespace_entry.map_err(Into::<Error>::into)?;
        let namespace_path = namespace_entry.path();
        if !namespace_path.is_dir() {
            continue;
        }
        let namespace = namespace_entry.file_name().to_string_lossy().to_string();

        for package_entry in std::fs::read_dir(&namespace_path).map_err(Into::<Error>::into)? {
            let package_entry = package_entry.map_err(Into::<Error>::into)?;
            let package_path = package_entry.path();
            if !package_path.is_dir() {
                continue;
            }
            let package_name = package_entry.file_name().to_string_lossy().to_string();

            for version_entry in std::fs::read_dir(&package_path).map_err(Into::<Error>::into)? {
                let version_entry = version_entry.map_err(Into::<Error>::into)?;
                let version_path = version_entry.path();
                if !version_path.is_dir() {
                    continue;
                }
                let version = version_entry.file_name().to_string_lossy().to_string();

                packages.push(InstalledPackage {
                    namespace: namespace.clone(),
                    name: package_name.clone(),
                    version,
                });
            }
        }
    }

    packages.sort_by(|a, b| {
        a.namespace.cmp(&b.namespace)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| b.version.cmp(&a.version))
    });

    Ok(packages)
}

#[tauri::command]
pub async fn typst_delete_package(
    namespace: String,
    name: String,
    version: String,
) -> Result<()> {
    let cache_dir = get_package_cache_dir().ok_or(Error::Unknown)?;
    let package_version_path = cache_dir.join(&namespace).join(&name).join(&version);

    if !package_version_path.exists() {
        return Err(Error::Unknown);
    }

    std::fs::remove_dir_all(&package_version_path).map_err(Into::<Error>::into)?;

    let package_path = cache_dir.join(&namespace).join(&name);
    if package_path.read_dir().map_err(Into::<Error>::into)?.next().is_none() {
        let _ = std::fs::remove_dir(&package_path);
    }

    let namespace_path = cache_dir.join(&namespace);
    if namespace_path.read_dir().map_err(Into::<Error>::into)?.next().is_none() {
        let _ = std::fs::remove_dir(&namespace_path);
    }

    debug!("Deleted package @{}/{}:{}", namespace, name, version);
    Ok(())
}

#[tauri::command]
pub async fn typst_install_package(spec: String) -> Result<()> {
    use std::process::Command;

    let output = Command::new("typst")
        .args(["init", &format!("@{}", spec.trim_start_matches('@')), "/dev/null"])
        .output()
        .map_err(Into::<Error>::into)?;

    if !output.status.success() {
        let output = Command::new("typst")
            .args(["compile", "--help"])
            .output();
        
        if output.is_err() {
            debug!("typst CLI not found, cannot install packages");
            return Err(Error::Unknown);
        }
        return Err(Error::Unknown);
    }

    debug!("Installed package {}", spec);
    Ok(())
}

#[tauri::command]
pub async fn export_pdf<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: String,
) -> Result<()> {
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;

    let options = typst_pdf::PdfOptions::default();
    let pdf = typst_pdf::pdf(doc, &options).map_err(|_| Error::Unknown)?;
    
    let mut path_buf = PathBuf::from(&path);
    if path_buf.extension().is_none() {
        path_buf.set_extension("pdf");
    }
    
    std::fs::write(&path_buf, pdf).map_err(Into::<Error>::into)?;
    debug!("Exported PDF to {:?}", path_buf);
    
    Ok(())
}
