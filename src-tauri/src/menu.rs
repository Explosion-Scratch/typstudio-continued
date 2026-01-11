use crate::project::{Project, ProjectManager};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{Menu, MenuBuilder, SubmenuBuilder, MenuEvent};
use tauri::{AppHandle, Manager, Runtime, State, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub struct RecentProject {
    pub name: String,
    pub path: String,
}

pub fn build_menu<R: Runtime>(handle: &AppHandle<R>, recent_projects: &[RecentProject], is_project_open: bool) -> tauri::Result<Menu<R>> {
    use tauri::menu::{MenuItemBuilder, CheckMenuItemBuilder};

    let app_menu = SubmenuBuilder::new(handle, "Typstudio")
        .about(Some(tauri::menu::AboutMetadata::default()))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let mut recent_menu_builder = SubmenuBuilder::new(handle, "Open Recent");
    if recent_projects.is_empty() {
        recent_menu_builder = recent_menu_builder.item(
            &MenuItemBuilder::with_id("file_recent_none", "No Recent Projects")
                .enabled(false)
                .build(handle)?
        );
    } else {
        for (i, project) in recent_projects.iter().enumerate() {
            let id = format!("file_recent_item_{}", i);
            recent_menu_builder = recent_menu_builder.text(id, &project.name);
        }
    }
    
    let recent_sub = recent_menu_builder
        .separator()
        .text("file_clear_recent", "Clear Recent")
        .build()?;

    let export_menu = SubmenuBuilder::new(handle, "Export")
        .item(&MenuItemBuilder::with_id("file_export_pdf", "Export as PDF...").enabled(is_project_open).build(handle)?)
        .item(&MenuItemBuilder::with_id("file_export_svg", "Export as SVG (Zip)...").enabled(is_project_open).build(handle)?)
        .item(&MenuItemBuilder::with_id("file_export_png", "Export as PNG (Zip)...").enabled(is_project_open).build(handle)?)
        .build()?;

    let file_menu = SubmenuBuilder::new(handle, "File")
        .text("file_new_project", "New Project")
        .text("file_open_project", "Open Project...")
        .item(&recent_sub)
        .separator()
        .item(&MenuItemBuilder::with_id("file_save", "Save").enabled(is_project_open).build(handle)?)
        .item(&MenuItemBuilder::with_id("file_save_all", "Save All").enabled(is_project_open).build(handle)?)
        .separator()
        .item(&export_menu)
        .separator()
        .item(&MenuItemBuilder::with_id("file_close_project", "Close Project").enabled(is_project_open).build(handle)?)
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(handle, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(handle, "View")
        .item(&MenuItemBuilder::with_id("view_toggle_sidebar", "Toggle Sidebar").accelerator("CmdOrCtrl+B").enabled(is_project_open).build(handle)?)
        .item(&MenuItemBuilder::with_id("view_toggle_preview", "Toggle Preview").accelerator("CmdOrCtrl+Enter").enabled(is_project_open).build(handle)?)
        .separator()
        .fullscreen()
        .build()?;

    let packages_menu = SubmenuBuilder::new(handle, "Packages")
        .item(&MenuItemBuilder::with_id("packages_install", "Install Package...").enabled(is_project_open).build(handle)?)
        .build()?;

    let help_menu = SubmenuBuilder::new(handle, "Help")
        .text("help_documentation", "Typst Documentation")
        .text("help_typstudio", "Typstudio Help")
        .build()?;

    let menu = MenuBuilder::new(handle)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &packages_menu, &help_menu])
        .build()?;

    Ok(menu)
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id.as_ref();
    
    // Logic similar to before, but getting window might be different.
    // In v2, MenuEvent doesn't carry the window directly unless it's a WindowMenu?
    // But we are setting app-wide menu.
    // We can get the main window from app handle.
    
    let window = app.get_webview_window("main");
    if window.is_none() { return; }
    let window = window.unwrap();

    match id {
        "file_new_project" => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                // ... same logic
                 use crate::ipc::commands::create_playground;
                 // Assuming create_playground is available
                 match create_playground().await {
                    Ok(path) => {
                         let path = fs::canonicalize(&path).unwrap_or(PathBuf::from(path));
                          let project = Arc::new(Project::load_from_path(path, None));
                         let project_manager: State<'_, Arc<ProjectManager<R>>> = app_handle.state();
                         if let Some(w) = app_handle.get_webview_window("main") {
                             project_manager.set_project(&w, Some(project));
                         }
                    }
                    Err(e) => log::error!("Failed: {:?}", e),
                 }
            });
        }
        "file_open_project" => {
             // FileDialogBuilder from plugin-dialog
             let window_clone = window.clone();
             app.dialog().file().set_title("Open Project").pick_folder(move |path| {
                 if let Some(path) = path {
                      if let Ok(path) = path.into_path() {
                          let path = fs::canonicalize(&path).unwrap_or(path);
                          let project_manager: State<'_, Arc<ProjectManager<R>>> = window_clone.state();
                           let project = Arc::new(Project::load_from_path(path, None));
                          project_manager.set_project(&window_clone, Some(project));
                      }
                 }
             });
        }
        "file_save" => { let _ = window.emit("menu_save", ()); }
        "file_save_all" => { let _ = window.emit("menu_save_all", ()); }
        "file_export_pdf" => { let _ = window.emit("menu_export_pdf", ()); }
        "file_export_svg" => { let _ = window.emit("menu_export_svg", ()); }
        "file_export_png" => { let _ = window.emit("menu_export_png", ()); }
        "file_close_project" => {
             let project_manager: State<'_, Arc<ProjectManager<R>>> = window.state();
             project_manager.set_project(&window, None);
        }
        // ... exports ...
        // file_recent_item_ ...
        id if id.starts_with("file_recent_item_") => {
             if let Ok(index) = id.strip_prefix("file_recent_item_").unwrap_or("0").parse::<usize>() {
                  let _ = window.emit("menu_open_recent", index);
             }
        }
        "file_clear_recent" => { let _ = window.emit("menu_clear_recent", ()); }
        "view_toggle_sidebar" => { let _ = window.emit("toggle_sidebar", ()); }
        "view_toggle_preview" => { let _ = window.emit("toggle_preview", ()); }
        "packages_install" => { let _ = window.emit("show_install_package", ()); }
        "help_documentation" => {
             let _ = app.opener().open_url("https://typst.app/docs/", None::<&str>);
        }
        "help_typstudio" => {
             let _ = app.opener().open_url("https://github.com/Cubxity/typstudio", None::<&str>);
        }
        _ => {}
    }
}
