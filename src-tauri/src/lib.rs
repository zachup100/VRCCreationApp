mod commands;
mod prereq;
mod store;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !prereq::ensure() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let folder = app.path().app_data_dir()?;
            app.manage(Mutex::new(store::ProjectStore::load(folder)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::unity::list_unity_editors,
            commands::unity::open_unity_download_page,
            commands::projects::list_projects,
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::set_project_favorite,
            commands::projects::import_from_vcc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
