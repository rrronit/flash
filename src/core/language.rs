use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub source_file: String,
    pub compile_cmd: Option<String>,
    pub run_cmd: String,
    pub is_compiled: bool,
}

impl Default for Language {
    fn default() -> Self {
        Self {
            name: "python".to_string(),
            source_file: "main.py".to_string(),
            compile_cmd: None,
            run_cmd: "/usr/bin/python3 main.py".to_string(),
            is_compiled: false,
        }
    }
}