//! TermZip desktop shell: a Tauri application that owns shell sessions.

mod commands;
mod flow;
mod links;
mod sessions;
mod themes;

use sessions::Sessions;
use tauri::Manager;

/// The default macOS menu without "Close Window". A menu accelerator is handled by the system before
/// the page sees the key, so ⌘W would close the whole window; the app uses ⌘W to close a pane or a
/// tab. The window still closes with the red button.
#[cfg(target_os = "macos")]
fn app_menu(handle: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItemKind};

    let menu = Menu::default(handle)?;
    for item in menu.items()? {
        let MenuItemKind::Submenu(submenu) = item else {
            continue;
        };
        for entry in submenu.items()? {
            if let MenuItemKind::Predefined(predefined) = &entry
                && predefined.text()?.eq_ignore_ascii_case("close window")
            {
                submenu.remove(&entry)?;
            }
        }
    }
    Ok(menu)
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.menu(app_menu);

    builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
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
            commands::open_link,
            commands::load_workspace,
            commands::save_workspace,
            commands::list_blocks,
            themes::list_themes,
            themes::import_theme,
            themes::export_theme,
            themes::delete_theme,
        ])
        .build(tauri::generate_context!())
        .expect("could not initialize the TermZip application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<Sessions>().destroy_all();
            }
        });
}
