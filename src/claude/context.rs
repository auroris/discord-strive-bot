use std::fs;

use super::history::context_dir;
use super::types::ContextConfig;

pub fn load_context_file_blocks() -> (Vec<serde_json::Value>, Vec<String>) {
    let config_path = context_dir();
    let mut errors = Vec::new();

    let config_str = match fs::read_to_string(format!("{}/context.json", config_path)) {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!(
                "[DEBUG] Context file error: Failed to read config at '{}': {}",
                config_path, e
            ));
            return (Vec::new(), errors);
        }
    };

    let config: ContextConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!(
                "[DEBUG] Context file error: Failed to parse config: {}",
                e
            ));
            return (Vec::new(), errors);
        }
    };

    let blocks = config
        .files
        .iter()
        .map(|f| {
            let mut block = serde_json::json!({
                "type": "document",
                "source": {
                    "type": "file",
                    "file_id": f.id,
                },
            });
            if let Some(title) = &f.title {
                block["title"] = serde_json::Value::String(title.clone());
            }
            block
        })
        .collect();

    (blocks, errors)
}

pub fn load_system_extension() -> (String, Vec<String>) {
    let dir = context_dir();
    let path = format!("{}/system.md", dir);
    let mut errors = Vec::new();

    match fs::read_to_string(&path) {
        Ok(content) => (content, errors),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (String::new(), errors),
        Err(e) => {
            errors.push(format!(
                "[DEBUG] System file error: Failed to read '{}': {}",
                path, e
            ));
            (String::new(), errors)
        }
    }
}
