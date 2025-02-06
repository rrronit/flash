use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{ExecutionSettings, Language};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub source_code: String,
    pub language: Language,
    pub stdin: String,
    pub expected_output: String,
    pub settings: ExecutionSettings,
    pub status: JobStatus,
    pub created_at: i64,         
    pub started_at: Option<i64>, 
    pub finished_at: Option<i64>,
    pub output: JobOutput,
    pub number_of_runs: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub compile_output: Option<String>,
    pub time: Option<f64>,
    pub memory: Option<u64>,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Processing,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    CompilationError,
    RuntimeError(String),
    InternalError,
    ExecFormatError,
}

impl JobStatus {
    pub fn id(&self) -> i32 {
        match self {
            JobStatus::Queued => 1,
            JobStatus::Processing => 2,
            JobStatus::Accepted => 3,
            JobStatus::WrongAnswer => 4,
            JobStatus::TimeLimitExceeded => 5,
            JobStatus::CompilationError => 6,
            JobStatus::RuntimeError(e) => match e.as_str() {
                "SIGSEGV" => 7,
                "SIGXFSZ" => 8,
                "SIGFPE" => 9,
                "SIGABRT" => 10,
                "NZEC" => 11,
                _ => 12,
            },
            JobStatus::InternalError => 13,
            JobStatus::ExecFormatError => 14,
        }
    }
}

impl Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "In Queue"),
            JobStatus::Processing => write!(f, "Processing"),
            JobStatus::Accepted => write!(f, "Accepted"),
            JobStatus::WrongAnswer => write!(f, "Wrong Answer"),
            JobStatus::TimeLimitExceeded => write!(f, "Time Limit Exceeded"),
            JobStatus::CompilationError => write!(f, "Compilation Error"),
            JobStatus::RuntimeError(e) => write!(f, "Runtime Error: ({})", e),
            JobStatus::InternalError => write!(f, "Internal Error"),
            JobStatus::ExecFormatError => write!(f, "Exec Format Error"),
        }
    }
}

impl Job {
    pub fn new(source_code: String, language: Language) -> Self {
        Self {
            id: Uuid::new_v4().as_u128() as u64,
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
            id: 0,
            source_code: String::new(),
            language: Language::default(),
            stdin: String::new(),
            expected_output: String::new(),
            settings: ExecutionSettings::default(),
            status: JobStatus::Queued,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            started_at: None,
            finished_at: None,
            output: JobOutput::default(),
            number_of_runs: 5,
        }
    }
}
