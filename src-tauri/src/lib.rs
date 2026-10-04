mod commands;
mod prefs;
mod prereq;
mod store;
mod vpm;

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
            app.manage(Mutex::new(store::ProjectStore::load(folder.clone())));
            app.manage(Mutex::new(prefs::Prefs::load(folder.clone())));
            app.manage(vpm::VpmContext::new(folder.join("vpm"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::unity::list_unity_editors,
            commands::unity::open_external,
            commands::projects::list_projects,
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::set_project_favorite,
            commands::projects::import_from_vcc,
            commands::projects::open_project,
            commands::projects::open_project_folder,
            commands::packages::project_packages,
            commands::repositories::repositories_info,
            commands::repositories::repository_preview,
            commands::repositories::repository_add,
            commands::repositories::repository_remove,
            commands::repositories::repository_set_hidden,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
