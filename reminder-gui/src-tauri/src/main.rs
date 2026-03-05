#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_reminders,
            commands::add_reminder,
            commands::delete_reminder,
            commands::pause_reminder,
            commands::resume_reminder,
            commands::get_tags,
            commands::test_trigger,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}