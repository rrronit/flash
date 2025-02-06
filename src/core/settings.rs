use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSettings {
    pub cpu_time_limit: f64,
    pub wall_time_limit: f64,
    pub memory_limit: u64,
    pub stack_limit: u64,
    pub max_processes: u32,
    pub max_file_size: u64,
    pub enable_network: bool,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            cpu_time_limit: 2.0,
            wall_time_limit: 5.0,
            memory_limit: 128_000,
            stack_limit: 64_000,
            max_processes: 60,
            max_file_size: 4096,
            enable_network: false,
        }
    }
}