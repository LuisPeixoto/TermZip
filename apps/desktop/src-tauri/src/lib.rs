//! TermZip desktop shell: a Tauri application that owns shell sessions.

mod commands;
mod flow;
mod sessions;

use sessions::Sessions;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(Sessions::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::app_info,
            commands::detect_shells,
            commands::create_session,
            commands::destroy_session,
            commands::write_session,
            commands::resize_session,
            commands::ack_session,
            commands::load_workspace,
            commands::save_workspace,
            commands::list_blocks,
        ])
        .build(tauri::generate_context!())
        .expect("could not initialize the TermZip application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<Sessions>().destroy_all();
            }
        });
}
