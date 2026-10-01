mod commands;
mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::prepare_environment();

    let result = tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_account_settings,
            commands::validate_account_settings,
            commands::default_port,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("failed to start the application: {error}");
        std::process::exit(1);
    }
}
