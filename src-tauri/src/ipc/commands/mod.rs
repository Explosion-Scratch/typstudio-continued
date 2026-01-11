mod clipboard;
mod fs;
mod typst;
mod playground;

pub use self::typst::*;
pub use clipboard::*;
pub use fs::*;
pub use playground::*;

use crate::project::{Project, ProjectManager};
use ::typst::diag::FileError;
use serde::{Serialize, Serializer};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::{Runtime, State, Window};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown error")]
    Unknown,
    #[error("unknown project")]
    UnknownProject,
    #[error("io error occurred")]
    IO(#[from] io::Error),
    #[error("typst file error occurred")]
    TypstFile(#[from] FileError),
    #[error("open error occurred")]
    Open(#[from] opener::OpenError),
    #[error("the provided path does not belong to the project")]
    UnrelatedPath,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn project<R: Runtime>(
    window: &Window<R>,
    project_manager: &State<Arc<ProjectManager<R>>>,
) -> Result<Arc<Project>> {
    project_manager
        .get_project(window)
        .ok_or(Error::UnknownProject)
}

pub fn project_path<R: Runtime, P: AsRef<Path>>(
    window: &Window<R>,
    project_manager: &State<Arc<ProjectManager<R>>>,
    path: P,
) -> Result<(Arc<Project>, PathBuf)> {
    let project = project_manager
        .get_project(window)
        .ok_or(Error::UnknownProject)?;
    let root_len = project.root.as_os_str().len();
    let mut out = project.root.to_path_buf();
    for component in path.as_ref().components() {
        match component {
            Component::Prefix(_) => {}
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
                if out.as_os_str().len() < root_len {
                    return Err(Error::UnrelatedPath);
                }
            }
            Component::Normal(_) => out.push(component),
        }
    }
    Ok((project, out))
}

#[tauri::command]
pub async fn open_project<R: Runtime>(
    window: Window<R>,
    project_manager: State<'_, Arc<ProjectManager<R>>>,
    path: String,
) -> Result<()> {
    use crate::ipc::LoadingProgressEvent;
    
    let _ = window.emit("loading_progress", LoadingProgressEvent {
        stage: "Initializing".to_string(),
        progress: 10,
        message: Some("Opening project...".to_string()),
    });
    
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    
    let path = PathBuf::from(&path);
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    
    let _ = window.emit("loading_progress", LoadingProgressEvent {
        stage: "Loading fonts".to_string(),
        progress: 30,
        message: Some("Loading fonts...".to_string()),
    });
    
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    
    let project = Arc::new(Project::load_from_path(path));
    
    let _ = window.emit("loading_progress", LoadingProgressEvent {
        stage: "Finalizing".to_string(),
        progress: 80,
        message: Some("Finalizing...".to_string()),
    });
    
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    
    project_manager.set_project(&window, Some(project));
    
    let _ = window.emit("loading_progress", LoadingProgressEvent {
        stage: "Ready".to_string(),
        progress: 100,
        message: Some("Ready".to_string()),
    });
    
    Ok(())
}

