pub mod commands;

use commands::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            current_index: Mutex::new(None),
            current_root: Mutex::new(None),
            current_manifest: Mutex::new(None),
            current_match_result: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_directory,
            commands::get_current_scan,
            commands::filter_entities,
            commands::get_entity_detail,
            commands::get_entity_source,
            commands::search_entities,
            commands::generate_prompt,
            commands::export_entities,
            commands::get_build_status,
            commands::open_in_editor,
            commands::check_editors_available,
            commands::load_manifest,
            commands::match_manifest,
            commands::get_tube_map_data,
            commands::get_subsystem_entities,
            commands::get_subsystem_detail,
            commands::bootstrap_manifest,
            commands::save_manifest,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Error running tauri application: {e}");
        });
}
