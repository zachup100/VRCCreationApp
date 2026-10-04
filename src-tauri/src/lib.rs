mod prereq;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !prereq::ensure() {
        return;
    }
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running application");
}
