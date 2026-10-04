mod applog;
mod args;
mod commands;
mod download;
mod manifest;
mod prefs;
mod presets;
mod prereq;
mod store;
mod vpm;

use std::sync::Mutex;
use tauri::{Emitter, Manager};

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
            app.manage(applog::AppLog::default());
            app.manage(Mutex::new(prefs::Prefs::load(folder.clone())));
            app.manage(Mutex::new(presets::PresetStore::load(folder.clone())));
            app.manage(vpm::VpmContext::new(folder.join("vpm"))?);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let busy = window
                    .try_state::<vpm::VpmContext>()
                    .map(|vpm| vpm.try_busy().is_err())
                    .unwrap_or(false);
                if busy {
                    api.prevent_close();
                    let _ = window.emit("close-blocked", ());
                }
            }
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
            commands::projects::refresh_project_info,
            commands::projects::set_project_launch_args,
            commands::backups::list_backups,
            commands::backups::delete_backup,
            commands::backups::restore_backup,
            commands::migrate::backup_project,
            commands::migrate::migrate_project,
            commands::migrate::open_backup_folder,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::packages::project_packages,
            commands::about::get_licenses,
            applog::get_logs,
            applog::clear_logs,
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::delete_preset,
            commands::presets::available_packages,
            commands::presets::set_packages_hidden,
            commands::presets::show_all_packages,
            commands::duplicate::duplicate_project,
            commands::create::list_templates,
            commands::create::create_project,
            commands::changes::project_plan_changes,
            commands::changes::project_apply_changes,
            commands::changes::project_cleanup_manifest,
            commands::repositories::repositories_info,
            commands::repositories::repository_preview,
            commands::repositories::repository_add,
            commands::repositories::repository_remove,
            commands::repositories::repository_headers,
            commands::repositories::repository_update,
            commands::repositories::repository_set_hidden,
            commands::repositories::import_packages_from_vcc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
