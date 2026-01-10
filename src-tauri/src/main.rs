#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod engine;
mod ipc;
mod menu;
mod project;

use crate::menu::handle_menu_event;
use crate::project::ProjectManager;
use env_logger::Env;
use log::info;
use std::sync::Arc;
use tauri::{AboutMetadata, CustomMenuItem, Menu, MenuItem, Submenu, Wry};

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    info!("initializing typstudio");

    let project_manager = Arc::new(ProjectManager::<Wry>::new());
    if let Ok(watcher) = ProjectManager::init_watcher(project_manager.clone()) {
        project_manager.set_watcher(watcher);
    }

    tauri::Builder::default()
        .menu(build_menu())
        .on_menu_event(handle_menu_event)
        .manage(project_manager)
        .invoke_handler(tauri::generate_handler![
            ipc::commands::fs_list_dir,
            ipc::commands::fs_read_file_binary,
            ipc::commands::fs_read_file_text,
            ipc::commands::fs_create_file,
            ipc::commands::fs_write_file_binary,
            ipc::commands::fs_write_file_text,
            ipc::commands::typst_compile,
            ipc::commands::typst_render,
            ipc::commands::typst_autocomplete,
            ipc::commands::clipboard_paste,
            ipc::commands::open_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_menu() -> Menu {
    let application_menu = Submenu::new(
        "Typstudio",
        Menu::new()
            .add_native_item(MenuItem::About(
                String::from("Typstudio"),
                AboutMetadata::new(),
            ))
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Services)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Hide)
            .add_native_item(MenuItem::HideOthers)
            .add_native_item(MenuItem::ShowAll)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Quit),
    );

    let file_menu = Menu::new()
        .add_item(
            CustomMenuItem::new("file_new_project", "New Project").accelerator("CmdOrCtrl+N"),
        )
        .add_item(
            CustomMenuItem::new("file_open_project", "Open Project...").accelerator("CmdOrCtrl+O"),
        )
        .add_native_item(MenuItem::Separator)
        .add_item(CustomMenuItem::new("file_save", "Save").accelerator("CmdOrCtrl+S"))
        .add_item(CustomMenuItem::new("file_save_all", "Save All").accelerator("CmdOrCtrl+Shift+S"))
        .add_native_item(MenuItem::Separator)
        .add_submenu(Submenu::new(
            "Export",
            Menu::new().add_item(
                CustomMenuItem::new("file_export_pdf", "Export as PDF...").accelerator("CmdOrCtrl+E"),
            ),
        ))
        .add_native_item(MenuItem::Separator)
        .add_item(CustomMenuItem::new("file_close_project", "Close Project"));

    #[cfg(not(target_os = "macos"))]
    let file_menu = file_menu.add_native_item(MenuItem::Quit);

    let file_submenu = Submenu::new("File", file_menu);

    let edit_submenu = Submenu::new(
        "Edit",
        Menu::new()
            .add_native_item(MenuItem::Undo)
            .add_native_item(MenuItem::Redo)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Cut)
            .add_native_item(MenuItem::Copy)
            .add_native_item(MenuItem::Paste)
            .add_native_item(MenuItem::SelectAll),
    );

    let view_submenu = Submenu::new(
        "View",
        Menu::new()
            .add_item(
                CustomMenuItem::new("view_toggle_sidebar", "Toggle Sidebar")
                    .accelerator("CmdOrCtrl+B"),
            )
            .add_item(
                CustomMenuItem::new("view_toggle_preview", "Toggle Preview")
                    .accelerator("CmdOrCtrl+Shift+P"),
            )
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::EnterFullScreen),
    );

    let packages_submenu = Submenu::new(
        "Packages",
        Menu::new().add_item(
            CustomMenuItem::new("packages_install", "Install Package...")
                .accelerator("CmdOrCtrl+Shift+I"),
        ),
    );

    let help_submenu = Submenu::new(
        "Help",
        Menu::new()
            .add_item(CustomMenuItem::new("help_documentation", "Typst Documentation"))
            .add_item(CustomMenuItem::new("help_typstudio", "Typstudio Help")),
    );

    let mut menu = Menu::new();
    #[cfg(target_os = "macos")]
    {
        menu = menu.add_submenu(application_menu)
    }
    menu.add_submenu(file_submenu)
        .add_submenu(edit_submenu)
        .add_submenu(view_submenu)
        .add_submenu(packages_submenu)
        .add_submenu(help_submenu)
}
