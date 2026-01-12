use super::{Error, Result};
use crate::ipc::commands::project_path;
use crate::project::ProjectManager;
use enumset::EnumSetType;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Runtime, State, WebviewWindow};
use ignore::WalkBuilder;

#[derive(Serialize, Debug)]
pub struct FileItem {
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: FileType,
}

#[derive(EnumSetType, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    File,
    Directory,
}

/// Reads raw bytes from a specified path.
/// Note that this command is slow compared to the text API due to Wry's
/// messaging system in v1. See: https://github.com/tauri-apps/tauri/issues/1817
#[tauri::command]
pub async fn fs_read_file_binary<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<Vec<u8>> {
    let (_, path) = project_path(&window, &project_manager, path)?;
    fs::read(path).map_err(Into::into)
}

#[tauri::command]
pub async fn fs_read_file_text<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<String> {
    let (_, path) = project_path(&window, &project_manager, path)?;
    fs::read_to_string(path).map_err(Into::into)
}

#[tauri::command]
pub async fn fs_create_file<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<()> {
    let (_, path) = project_path(&window, &project_manager, path)?;

    // Not sure if there's a scenario where this condition is not met
    // unless the project is located at `/`
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(Into::<Error>::into)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&*path)
        .map_err(Into::<Error>::into)?;
    Ok(())
}

#[tauri::command]
pub async fn fs_write_file_binary<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: Vec<u8>,
) -> Result<()> {
    let (_, path) = project_path(&window, &project_manager, path)?;
    fs::write(path, content).map_err(Into::into)
}

#[tauri::command]
pub async fn fs_write_file_text<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
    content: String,
) -> Result<()> {
    let (project, absolute_path) = project_path(&window, &project_manager, &path)?;
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(Into::<Error>::into)?;
    }
    let _ = File::create(&absolute_path)
        .map(|mut f| f.write_all(content.as_bytes()))
        .map_err(Into::<Error>::into)?;

    let world = project.world.lock().unwrap_or_else(|e| {
        log::warn!("Project world mutex poisoned, recovering: {}", e);
        e.into_inner()
    });
    let _ = world
        .slot_update(&path, Some(content))
        .map_err(Into::<Error>::into)?;

    Ok(())
}

#[tauri::command]
pub async fn fs_list_dir<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<Vec<FileItem>> {
    let (_, path) = project_path(&window, &project_manager, path)?;
    let list = fs::read_dir(path).map_err(Into::<Error>::into)?;

    let mut files: Vec<FileItem> = vec![];
    list.into_iter().for_each(|entry| {
        if let Ok(entry) = entry {
            if let (Ok(file_type), Ok(name)) = (entry.file_type(), entry.file_name().into_string())
            {
                // File should only be directory or file.
                // Symlinks should be resolved in project_path.
                let t = if file_type.is_dir() {
                    FileType::Directory
                } else {
                    FileType::File
                };
                files.push(FileItem { name, file_type: t });
            }
        }
    });

    files.sort_by(|a, b| {
        if a.file_type == FileType::Directory && b.file_type == FileType::File {
            Ordering::Less
        } else if a.file_type == FileType::File && b.file_type == FileType::Directory {
            Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(files)
}

#[tauri::command]
pub async fn fs_delete_file<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<()> {
    let (_, abs_path) = project_path(&window, &project_manager, path)?;
    if abs_path.is_dir() {
        fs::remove_dir_all(&abs_path).map_err(Into::<Error>::into)?;
    } else {
        fs::remove_file(&abs_path).map_err(Into::<Error>::into)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn fs_rename_file<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    old_path: PathBuf,
    new_path: PathBuf,
) -> Result<()> {
    let (_, old_abs) = project_path(&window, &project_manager, &old_path)?;
    let (_, new_abs) = project_path(&window, &project_manager, &new_path)?;
    fs::rename(&old_abs, &new_abs).map_err(Into::<Error>::into)?;
    Ok(())
}
#[tauri::command]
pub async fn fs_reveal_path<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: PathBuf,
) -> Result<()> {
    let abs_path = if path.is_absolute() {
        path
    } else {
        let (_, abs) = project_path(&window, &project_manager, path)?;
        abs
    };
    
    opener::reveal(abs_path).map_err(Into::<Error>::into)?;

    Ok(())
}

#[tauri::command]
pub async fn fs_search_files<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
) -> Result<Vec<String>> {
    let project = super::project(&window, &project_manager)?;
    let root = project.root.clone();

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .git_ignore(true)
        .require_git(false)
        .filter_entry(|entry| {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let nomedia = entry.path().join(".nomedia");
                if nomedia.exists() {
                    return false;
                }
            }
            true
        })
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        
        if path.is_dir() {
            continue;
        }

        if let Ok(relative_path) = path.strip_prefix(&root) {
            if let Some(path_str) = relative_path.to_str() {
                if !path_str.is_empty() {
                    files.push(path_str.to_string());
                }
            }
        }
    }

    Ok(files)
}
