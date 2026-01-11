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

pub fn build_menu<R: Runtime>(handle: &AppHandle<R>, recent_projects: &[RecentProject]) -> tauri::Result<Menu<R>> {
    let app_menu = SubmenuBuilder::new(handle, "Typstudio")
        .about(Some(tauri::menu::AboutMetadata::default())) // About is now a method on SubmenuBuilder or a predefined item
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let mut recent_menu = SubmenuBuilder::new(handle, "Open Recent");
    if recent_projects.is_empty() {
        recent_menu = recent_menu.text("file_recent_none", "No Recent Projects"); // .disabled() needs check
        // TextMenuItemBuilder might be needed for disabled
    } else {
        for (i, project) in recent_projects.iter().enumerate() {
            let id = format!("file_recent_item_{}", i);
             recent_menu = recent_menu.text(id, &project.name);
        }
    }
    // Note: submenu building logic is slightly different, usually we define items.
    // Let's use MenuBuilder directly for recent_menu if it's a submenu? 
    // Submenu is a Menu.
    
    // Actually SubmenuBuilder::new returns a builder.
    // For disabled items:
    // .item(&MenuItemBuilder::with_id("id", "text").enabled(false).build(handle)?)
    
    // Let's simplified handling for now.
    let recent_sub = recent_menu
        .separator()
        .text("file_clear_recent", "Clear Recent")
        .build()?;

    let file_menu = SubmenuBuilder::new(handle, "File")
        .text("file_new_project", "New Project") // accelerator needs separate builder
        .text("file_open_project", "Open Project...")
        .item(&recent_sub)
        .separator()
        .text("file_save", "Save")
        .text("file_save_all", "Save All")
        .separator()
        .item(
            &SubmenuBuilder::new(handle, "Export")
                .text("file_export_pdf", "Export as PDF...")
                .text("file_export_svg", "Export as SVG (Zip)...")
                .text("file_export_png", "Export as PNG (Zip)...")
                .build()?
        )
        .separator()
        .text("file_close_project", "Close Project")
        .quit() // if safe
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
        .text("view_toggle_sidebar", "Toggle Sidebar")
        .text("view_toggle_preview", "Toggle Preview")
        .separator()
        .fullscreen()
        .build()?;

    let packages_menu = SubmenuBuilder::new(handle, "Packages")
        .text("packages_install", "Install Package...")
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
                         let project = Arc::new(Project::load_from_path(path));
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
                          let project = Arc::new(Project::load_from_path(path));
                          project_manager.set_project(&window_clone, Some(project));
                      }
                 }
             });
        }
        "file_save" => { let _ = window.emit("menu_save", ()); }
        "file_save_all" => { let _ = window.emit("menu_save_all", ()); }
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
