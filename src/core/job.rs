use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{sync::atomic::{AtomicUsize, Ordering}, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;

use super::{ExecutionSettings, Language};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u128,
    pub source_code: String,
    pub language: Language,
    pub stdin: String,
    pub expected_output: String,
    pub settings: ExecutionSettings,
    pub status: JobStatus,
    pub created_at: i64, // Unix timestamp in seconds
    pub started_at: Option<i64>, // Unix timestamp in seconds
    pub finished_at: Option<i64>, 
    pub output: JobOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub compile_output: Option<String>,
    pub time: Option<f64>,
    pub memory: Option<u64>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed(String),
}

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Invalid job configuration")]
    ConfigurationError,
    #[error("Execution timeout")]
    TimeoutError,
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
    #[error("Compilation failed: {0}")]
    CompilationError(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

impl Job {
    pub fn new(source_code: String, language: Language) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let id = if id > 999 { 1 } else { id }; // Reset to 1 if exceeds 999

        Self {
            id: id as u128,
            source_code,
            language,
            ..Default::default()
        }
    }

    pub fn with_stdin(mut self, stdin: String) -> Self {
        self.stdin = stdin;
        self
    }

    pub fn with_expected_output(mut self, expected_output: String) -> Self {
        self.expected_output = expected_output;
        self
    }

    pub fn set_limits(
        mut self,
        cpu_time_limit: f64,
        memory_limit: u64,
        stack_limit: u64,
        max_processes: u32,
    ) -> Self {
        self.settings.cpu_time_limit = cpu_time_limit;
        self.settings.memory_limit = memory_limit;
        self.settings.stack_limit = stack_limit;
        self.settings.max_processes = max_processes;
        self
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: rand::random(),
            source_code: String::new(),
            language: Language::default(),
            stdin: String::new(),
            expected_output: String::new(),
            settings: ExecutionSettings::default(),
            status: JobStatus::Queued,
            created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH
            )
            .unwrap()
            .as_secs() as i64,
            started_at: None,
            finished_at: None,
            output: JobOutput::default(),
        }
    }
}
