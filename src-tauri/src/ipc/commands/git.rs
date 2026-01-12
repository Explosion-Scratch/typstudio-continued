use super::{Result, Error, project_path};
use git2::Repository;
use tauri::{Runtime, State, WebviewWindow};
use crate::project::ProjectManager;
use std::sync::Arc;

#[tauri::command]
pub async fn git_read_original_file<R: Runtime>(
    window: WebviewWindow<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: String,
) -> Result<String> {
    let (project, full_path) = project_path(&window, &project_manager, &path)?;
    
    let repo = match Repository::discover(&project.root) {
        Ok(r) => r,
        Err(_) => return Ok(String::new()), 
    };

    let relative_path = match full_path.strip_prefix(repo.workdir().unwrap_or(&project.root)) {
        Ok(p) => p,
        Err(_) => return Ok(String::new()),
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(String::new()), 
    };
    let tree = head.peel_to_tree().map_err(|_| Error::Unknown)?;
    
    // Find the entry in the tree
    let entry = match tree.get_path(relative_path) {
        Ok(e) => e,
        Err(_) => return Ok(String::new()), // File not in HEAD (e.g. new file)
    };

    let object = entry.to_object(&repo).map_err(|_| Error::Unknown)?;
    let blob = object.as_blob().ok_or(Error::Unknown)?;
    
    let content = std::str::from_utf8(blob.content()).unwrap_or("").to_string();
    
    Ok(content)
}
