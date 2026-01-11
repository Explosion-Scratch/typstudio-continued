use super::{Error, Result};
use crate::compiler::{CompileRequest, Compiler};
use crate::ipc::commands::project;
use crate::ipc::model::TypstRenderResponse;
use crate::project::ProjectManager;
use log::debug;
use serde::Serialize;
use serde_repr::Serialize_repr;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Runtime;
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

// REFACTORED COMMAND
#[tauri::command]
pub async fn typst_compile<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    compiler: tauri::State<'_, Arc<Compiler<R>>>,
    path: PathBuf,
    content: String,
    request_id: u64,
) -> Result<()> {
    compiler.update(CompileRequest {
        path,
        content,
        request_id,
        window_label: window.label().to_string(),
    });
    
    Ok(())
}

#[tauri::command]
pub async fn typst_render<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    page: usize,
    scale: f32,
    nonce: u32,
) -> Result<TypstRenderResponse> {
    debug!("rendering page {} @{}x", page, scale); // Keep logging but minimal
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    if let Some(p) = cache.document.as_ref().and_then(|doc| doc.pages.get(page)) {
        // Use typst_svg::svg
        let svg = typst_svg::svg(p);
        
        // Calculate dimensions
        let width = (p.frame.width().to_pt() * scale as f64) as u32;
        let height = (p.frame.height().to_pt() * scale as f64) as u32;
        
        return Ok(TypstRenderResponse {
            image: svg, // This is a String (SVG source)
            width,
            height,
            nonce,
        });
    }

    Err(Error::Unknown)
}

#[tauri::command]
pub async fn typst_autocomplete<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    offset: usize,
    explicit: bool,
) -> Result<TypstCompleteResponse> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });

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

// ... helper functions for jump ...
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
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    page: usize,
    x: f64,
    y: f64,
) -> Result<Option<TypstJump>> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });
    let cache = project.cache.read().unwrap();

    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;
    let page_doc = doc.pages.get(page).ok_or(Error::Unknown)?;

    let point = typst::layout::Point::new(
        typst::layout::Abs::pt(x),
        typst::layout::Abs::pt(y)
    );

    // Try precise jump first
    let (span, span_offset) = match find_precise_jump(&page_doc.frame, point)
        .or_else(|| {
            // Fallback
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
    {
        Some(res) => res,
        None => return Ok(None),
    };

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
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
    byte_offset: usize,
) -> Result<Option<TypstDocumentPosition>> {
    let project = project(&window, &project_manager)?;
    let world = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });
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

    let mut result_pos = None;
    for (i, page) in doc.pages.iter().enumerate() {
        if let Some(point) = find_precise_position(&page.frame, target_span, target_offset) {
            result_pos = Some(TypstDocumentPosition {
                page: i,
                x: point.x.to_pt(),
                y: point.y.to_pt(),
                text: None,
                node_kind: Some(format!("{:?}", node.kind())),
            });
            break;
        }
    }

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
    window: tauri::WebviewWindow<R>,
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


#[tauri::command]
pub async fn export_svg<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: String,
) -> Result<()> {
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;

    let mut path_buf = PathBuf::from(&path);
    if path_buf.extension().is_none() {
        path_buf.set_extension("zip");
    }

    let file = std::fs::File::create(&path_buf).map_err(Into::<Error>::into)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    for (i, page) in doc.pages.iter().enumerate() {
        let svg = typst_svg::svg(page);
        let filename = format!("page_{:02}.svg", i + 1);
        zip.start_file(filename, options).map_err(|e| Error::IO(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        use std::io::Write;
        zip.write_all(svg.as_bytes()).map_err(Into::<Error>::into)?;
    }

    zip.finish().map_err(|e| Error::IO(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    debug!("Exported SVG zip to {:?}", path_buf);
    Ok(())
}

#[tauri::command]
pub async fn export_png<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    project_manager: tauri::State<'_, Arc<ProjectManager<R>>>,
    path: String,
) -> Result<()> {
    let project = project_manager
        .get_project(&window)
        .ok_or(Error::UnknownProject)?;

    let cache = project.cache.read().unwrap();
    let doc = cache.document.as_ref().ok_or(Error::Unknown)?;

    let mut path_buf = PathBuf::from(&path);
    if path_buf.extension().is_none() {
        path_buf.set_extension("zip");
    }

    let file = std::fs::File::create(&path_buf).map_err(Into::<Error>::into)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    let ppi = 144.0;
    let scale = ppi / 72.0;

    for (i, page) in doc.pages.iter().enumerate() {
        let pixmap = typst_render::render(page, scale);
        let filename = format!("page_{:02}.png", i + 1);
        zip.start_file(filename, options).map_err(|e| Error::IO(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        
        // Encoding directly
        // create a temporary encoder to write to buffer? 
        // pixmap.save_png is convenient but writes to path.
        // We can encode using the png crate or just use an intermediate buffer if pixmap exposes raw bytes.
        // pixmap.encode_png() returns Result<Vec<u8>> in recent versions? 
        // checking typst_render usage... it returns a Pixmap.
        // Pixmap has encode_png().
        if let Ok(data) = pixmap.encode_png() {
             use std::io::Write;
             zip.write_all(&data).map_err(Into::<Error>::into)?;
        } else {
             return Err(Error::Unknown);
        }
    }
    
    zip.finish().map_err(|e| Error::IO(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    debug!("Exported PNG zip to {:?}", path_buf);
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct RecentProjectInfo {
    path: String,
    name: String,
    #[serde(default)]
    _last_opened: Option<u64>,
}

#[tauri::command]
pub async fn update_menu_state<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    projects: Vec<RecentProjectInfo>,
    is_project_open: bool,
) -> Result<()> {
    use tauri::Manager;
    use crate::menu::{build_menu, RecentProject};
    
    let recent_projects: Vec<RecentProject> = projects.into_iter().map(|p| RecentProject {
        name: p.name,
        path: p.path,
    }).collect();
    
    log::info!("Updating menu state: is_project_open={}, recent_projects_count={}", is_project_open, recent_projects.len());
    
    match build_menu(window.app_handle(), &recent_projects, is_project_open) {
        Ok(menu) => {
            if let Err(e) = window.app_handle().set_menu(menu) {
                log::error!("Failed to set app menu: {}", e);
                return Err(Error::Unknown);
            }
            log::info!("Menu updated successfully");
        }
        Err(e) => {
            log::error!("Failed to build menu: {}", e);
            return Err(Error::Unknown);
        }
    }
    Ok(())
}
