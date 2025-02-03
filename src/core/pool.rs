use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u128,
    pub source_code: String,
    pub language: Language,
    pub stdin: String,
    pub expected_output: String,
    pub settings: ExecutionSettings,
    pub status: JobStatus,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub output: JobOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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


impl Job {
    pub fn new(source_code: String, language: Language) -> Self {
        Self {
            id: rand::random(),
            source_code,
            language,
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), JobError> {
        if self.source_code.is_empty() {
            return Err(JobError::ConfigurationError);
        }
        Ok(())
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
            created_at: SystemTime::now(),
            started_at: None,
            finished_at: None,
            output: JobOutput::default(),
        }
    }
}