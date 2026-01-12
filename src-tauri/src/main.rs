#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod compiler;
mod engine;
mod ipc;
mod menu;
mod project;

use crate::compiler::Compiler;


use crate::project::ProjectManager;
use env_logger::Env;
use log::info;
use std::sync::Arc;
use tauri::Manager;
use tauri::Wry;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    info!("initializing typstudio");

    let project_manager = Arc::new(ProjectManager::<Wry>::new());
    if let Ok(watcher) = ProjectManager::init_watcher(project_manager.clone()) {
        project_manager.set_watcher(watcher);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        // .menu(menu::build_menu(&[]))
        // .on_menu_event(handle_menu_event)
        .manage(project_manager.clone())
        .setup(move |app| {
            let handle = app.handle();
            let menu = menu::build_menu(handle, &[], false)?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                menu::handle_menu_event(app, event);
            });

            let compiler = Arc::new(Compiler::new(project_manager, app.handle().clone()));
            app.manage(compiler);

            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            } else {
                println!("Error: Could not find window labeled 'main'");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::fs_list_dir,
            ipc::commands::fs_read_file_binary,
            ipc::commands::fs_read_file_text,
            ipc::commands::fs_create_file,
            ipc::commands::fs_write_file_binary,
            ipc::commands::fs_write_file_text,
            ipc::commands::fs_delete_file,
            ipc::commands::fs_rename_file,
            ipc::commands::fs_reveal_path,
            ipc::commands::fs_search_files,
            ipc::commands::git_read_original_file,
            ipc::commands::typst_compile,
            ipc::commands::typst_render,
            ipc::commands::typst_autocomplete,
            ipc::commands::typst_jump,
            ipc::commands::typst_jump_from_cursor,
            ipc::commands::typst_list_packages,
            ipc::commands::typst_delete_package,
            ipc::commands::typst_install_package,
            ipc::commands::typst_get_document_sources,
            ipc::commands::clipboard_paste,
            ipc::commands::open_project,
            ipc::commands::create_playground,
            ipc::commands::export_pdf,
            ipc::commands::export_svg,
            ipc::commands::export_png,
            ipc::commands::update_menu_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
