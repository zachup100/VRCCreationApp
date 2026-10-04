mod commands;
mod prereq;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !prereq::ensure() {
        return;
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::unity::list_unity_editors,
            commands::unity::open_unity_download_page,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
