mod commands;
mod platform;
mod secrets;

use std::sync::Arc;

use mail_engine::Engine;
use mail_protocols::NetConnector;
use mail_store::Store;
use tauri::Manager;

fn build_engine(app: &tauri::App) -> Result<Engine, Box<dyn std::error::Error>> {
    let directory = app.path().app_data_dir()?;
    std::fs::create_dir_all(&directory)?;
    let store = Store::open(&directory.join("mail.db"))?;

    Ok(Engine::new(
        store,
        Arc::new(NetConnector),
        Arc::new(secrets::KeychainSecrets),
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::prepare_environment();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.manage(Arc::new(build_engine(app)?));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_account_settings,
            commands::validate_account_settings,
            commands::default_port,
            commands::list_accounts,
            commands::add_account,
            commands::remove_account,
            commands::list_folders,
            commands::refresh_folders,
            commands::list_messages,
            commands::sync_folder,
            commands::open_message,
            commands::set_seen,
            commands::set_flagged,
            commands::delete_messages,
            commands::move_messages,
            commands::reply_draft,
            commands::send_message,
            commands::save_attachment,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("failed to start the application: {error}");
        std::process::exit(1);
    }
}
