const LICENSES: &str = include_str!("../../../THIRD-PARTY-LICENSES.txt");

#[tauri::command]
pub fn get_licenses() -> &'static str {
    LICENSES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_license_file_names_the_main_dependencies() {
        assert!(LICENSES.starts_with("THIRD-PARTY LICENSES"));
        for name in ["tauri ", "vrc-get-vpm ", "react ", "reqwest "] {
            assert!(LICENSES.contains(name), "{name} is missing from the license file");
        }
        assert!(LICENSES.contains("MIT License"));
        assert!(LICENSES.contains("Apache License"));
    }
}
