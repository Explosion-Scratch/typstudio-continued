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
    text: Option<String>,
    offset: Option<usize>,
    node_kind: Option<String>,
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
    
    println!("Backend handling compile request_id: {}", request_id);

    project.current_compile_request_id.store(request_id, Ordering::SeqCst);

    println!("Backend waiting for lock request_id: {}", request_id);
    let mut world = project.world.lock().unwrap();
    println!("Backend acquired lock request_id: {}", request_id);

    // Check if a newer request has come in while we were waiting for the lock
    let current_request = project.current_compile_request_id.load(Ordering::SeqCst);
    if current_request != request_id {
        println!("Backend aborting stale compile request {} (current: {})", request_id, current_request);
        debug!("aborting stale compile request {} (current: {})", request_id, current_request);
        return Ok(());
    }

    let source_id = world
        .slot_update(&path, Some(content.clone()))
        .map_err(Into::<Error>::into)?;

    // Set the processed file as main
    world.set_main_path(typst::syntax::VirtualPath::new(&path));

    if !world.is_main_set() {
        let config = project.config.read().unwrap();
        if config.apply_main(&project, &mut world).is_err() {
            debug!("skipped compilation for {:?} (main not set)", project);
            println!("Backend skipped compilation (main not set) request_id: {}", request_id);
            return Ok(());
        }
    }

    debug!("compiling {:?}: {:?} (request_id: {})", path, project, request_id);
    let now = Instant::now();
    println!("Backend calling typst::compile request_id: {}", request_id);
    let result = typst::compile::<typst::layout::PagedDocument>(&*world);
    match result.output {
        Ok(doc) => {
            let current_request = project.current_compile_request_id.load(Ordering::SeqCst);
            if current_request != request_id {
                debug!("skipping stale compile result for request {} (current: {})", request_id, current_request);
                println!("Backend skipping stale compile result for request {} (current: {})", request_id, current_request);
                return Ok(());
            }

            let elapsed = now.elapsed();
            debug!(
                "compilation succeeded for {:?} in {:?} ms",
                project,
                elapsed.as_millis()
            );
            println!("Backend compilation succeeded in {:?} ms request_id: {}", elapsed.as_millis(), request_id);

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

            println!("Backend emitting success event request_id: {}", request_id);
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
                println!("Backend skipping stale compile error for request {} (current: {})", request_id, current_request);
                return Ok(());
            }

            debug!(
                "compilation failed with {:?} diagnostics",
                diagnostics.len()
            );
            println!("Backend compilation failed with {} diagnostics request_id: {}", diagnostics.len(), request_id);

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
    println!("Backend rendering page {} @{}x", page, scale);
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    if let Some(p) = cache.document.as_ref().and_then(|doc| doc.pages.get(page)) {
        let now = Instant::now();
        
        // Use typst_svg::svg instead of render (based on previous context)
        // Check previously viewed file... yes it was typst_svg::svg(p)
        let svg = typst_svg::svg(p);
        let elapsed = now.elapsed();
        debug!(
            "SVG rendering complete for page {} in {} ms",
            page,
            elapsed.as_millis()
        );
        println!("Backend SVG rendering complete for page {} in {} ms", page, elapsed.as_millis());
        
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

fn find_precise_position(
    frame: &typst::layout::Frame,
    target_span: typst::syntax::Span,
    target_offset: u16,
) -> Option<typst::layout::Point> {
    use typst::layout::FrameItem;
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Text(text) => {
                let mut x = pos.x;
                for glyph in &text.glyphs {
                    if glyph.span.0 == target_span {
                        if glyph.span.1 >= target_offset {
                            return Some(typst::layout::Point::new(x, pos.y));
                        }
                    }
                    x += glyph.x_advance.at(text.size);
                }
            }
            FrameItem::Group(group) => {
                let local_pos = find_precise_position(&group.frame, target_span, target_offset);
                if let Some(mut point) = local_pos {
                    point = point.transform(group.transform);
                    point.x += pos.x;
                    point.y += pos.y;
                    return Some(point);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_precise_jump(
    frame: &typst::layout::Frame,
    click: typst::layout::Point,
) -> Option<(typst::syntax::Span, u16)> {
    use typst::layout::FrameItem;
    for (pos, item) in frame.items().rev() {
        let rel_click = click - *pos;
        match item {
            FrameItem::Text(text) => {
                let height = text.size;
                // Rough bounding box check for text line
                // Typst y is baseline, so we check around the baseline
                if rel_click.y >= -height && rel_click.y <= height * 0.5 {
                    let mut current_x = typst::layout::Abs::zero();
                    for glyph in &text.glyphs {
                        let width = glyph.x_advance.at(text.size);
                        if rel_click.x >= current_x && rel_click.x <= current_x + width {
                            return Some(glyph.span);
                        }
                        current_x += width;
                    }
                }
            }
            FrameItem::Group(group) => {
                // Invert transform if possible, but for simple jumps we just handle translation
                // since most Typst groups are just translated.
                if let Some(res) = find_precise_jump(&group.frame, rel_click) {
                    return Some(res);
                }
            }
            _ => {}
        }
    }
    None
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

    // Try precise jump first
    let (span, span_offset) = find_precise_jump(&page_doc.frame, point)
        .or_else(|| {
            // Fallback to standard IDE jump if precise fails
            let jump = typst_ide::jump_from_click(&*world, doc, &page_doc.frame, point);
            match jump {
                Some(typst_ide::Jump::File(id, offset)) => {
                    let source = world.source(id).ok()?;
                    let node = typst::syntax::LinkedNode::new(source.root())
                        .leaf_at(offset as usize, typst::syntax::Side::Before)?;
                    Some((node.span(), 0))
                }
                _ => None,
            }
        })
        .ok_or(Error::Unknown)?;

    let source_id = span.id().ok_or(Error::Unknown)?;
    let source = world.source(source_id).map_err(Into::<Error>::into)?;
    
    let range = source.find(span).ok_or(Error::Unknown)?.range();
    let offset = range.start + span_offset as usize;

    let lines = source.lines();
    let line = lines.byte_to_line(offset).ok_or(Error::Unknown)?;
    let column = lines.byte_to_column(offset).ok_or(Error::Unknown)?;

    let path = source.id().vpath().as_rootless_path().to_string_lossy().to_string();
    let filepath = if path.starts_with("/") { path } else { format!("/{}", path) };

    // Get a snippet of text around the offset
    let text = source.text();
    let snippet_start = offset.saturating_sub(50);
    let snippet_end = (offset + 50).min(text.len());
    
    let mut actual_start = snippet_start;
    while actual_start > 0 && !text.is_char_boundary(actual_start) {
        actual_start -= 1;
    }
    let mut actual_end = snippet_end;
    while actual_end < text.len() && !text.is_char_boundary(actual_end) {
        actual_end += 1;
    }
    
    let snippet = text[actual_start..actual_end].to_string();

    // Get syntax node info
    let node_kind = typst::syntax::LinkedNode::new(source.root())
        .leaf_at(offset, typst::syntax::Side::Before)
        .map(|n| format!("{:?}", n.kind()));

    Ok(Some(TypstJump {
        filepath,
        start: Some((line + 1, column + 1)),
        end: Some((line + 1, column + 1)),
        text: Some(snippet),
        offset: Some(offset),
        node_kind,
    }))
}

#[derive(Serialize, Debug)]
pub struct TypstDocumentPosition {
    page: usize,
    x: f64,
    y: f64,
    text: Option<String>,
    node_kind: Option<String>,
}

#[tauri::command]
pub async fn typst_jump_from_cursor<R: Runtime>(
    window: tauri::Window<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    byte_offset: usize,
) -> Result<Option<TypstDocumentPosition>> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap();
    let cache = project.cache.read().unwrap();

    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;

    let source_id = world
        .slot_update(&*path, Some(content.clone()))
        .map_err(Into::<Error>::into)?;

    let source = world.source(source_id).map_err(Into::<Error>::into)?;

    let node = typst::syntax::LinkedNode::new(source.root())
        .leaf_at(byte_offset, typst::syntax::Side::Before)
        .ok_or(Error::Unknown)?;
        
    let target_span = node.span();
    let target_offset = (byte_offset - node.offset()).min(u16::MAX as usize) as u16;

    // Search for precise position in pages
    let mut result_pos = None;
    for (i, page) in doc.pages.iter().enumerate() {
        if let Some(point) = find_precise_position(&page.frame, target_span, target_offset) {
            result_pos = Some(TypstDocumentPosition {
                page: i,
                x: point.x.to_pt(),
                y: point.y.to_pt(),
                text: None, // Will fill below
                node_kind: Some(format!("{:?}", node.kind())),
            });
            break;
        }
    }

    // Fallback to standard IDE jump if precise search failed
    let result_pos = if let Some(pos) = result_pos {
        Some(pos)
    } else {
        let positions = typst_ide::jump_from_cursor(doc, &source, byte_offset);
        positions.first().map(|position| TypstDocumentPosition {
            page: position.page.get().saturating_sub(1),
            x: position.point.x.to_pt(),
            y: position.point.y.to_pt(),
            text: None,
            node_kind: Some(format!("{:?}", node.kind())),
        })
    };

    if let Some(mut pos) = result_pos {
        // Get a snippet of text around the offset
        let text = source.text();
        let snippet_start = byte_offset.saturating_sub(50);
        let snippet_end = (byte_offset + 50).min(text.len());
        
        let mut actual_start = snippet_start;
        while actual_start > 0 && !text.is_char_boundary(actual_start) {
            actual_start -= 1;
        }
        let mut actual_end = snippet_end;
        while actual_end < text.len() && !text.is_char_boundary(actual_end) {
            actual_end += 1;
        }
        
        let snippet = text[actual_start..actual_end].to_string();
        pos.text = Some(snippet);
        Ok(Some(pos))
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
