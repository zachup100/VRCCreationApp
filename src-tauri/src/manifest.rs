use serde_json::Value;

pub fn remove_dependency_entries(text: &str, names: &[String]) -> Result<String, String> {
    let mut document: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("vpm-manifest.json is not valid JSON: {error}"))?;
    let dependencies = document
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "vpm-manifest.json has no dependencies list".to_string())?;
    for name in names {
        dependencies.remove(name);
    }
    serde_json::to_string_pretty(&document).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn removes_only_the_named_dependencies_and_keeps_order() {
        let text = r#"{"dependencies":{"z":{"version":"1"},"m":{"version":"2"},"a":{"version":"3"}},"locked":{"m":{"version":"2"}}}"#;
        let output = remove_dependency_entries(text, &names(&["m"])).unwrap();
        let value: Value = serde_json::from_str(&output).unwrap();
        let keys: Vec<&String> = value["dependencies"].as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["z", "a"]);
        assert!(value["locked"].get("m").is_some());
    }

    #[test]
    fn unknown_names_are_ignored() {
        let text = r#"{"dependencies":{"a":{"version":"1"}}}"#;
        let output = remove_dependency_entries(text, &names(&["nope"])).unwrap();
        let value: Value = serde_json::from_str(&output).unwrap();
        assert!(value["dependencies"].get("a").is_some());
    }

    #[test]
    fn a_byte_order_mark_is_accepted() {
        let text = "\u{feff}{\"dependencies\":{\"a\":{}}}";
        assert!(remove_dependency_entries(text, &names(&["a"])).is_ok());
    }

    #[test]
    fn invalid_json_and_missing_dependencies_are_errors() {
        assert!(remove_dependency_entries("not json", &names(&["a"])).is_err());
        assert!(remove_dependency_entries(r#"{"locked":{}}"#, &names(&["a"])).is_err());
    }
}