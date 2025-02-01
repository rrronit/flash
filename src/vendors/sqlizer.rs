use std::{error::Error, fs, process::Command};
use std::fmt;
use std::path::PathBuf;

use crate::core::pool::Code;

use super::isolate::Metadata;

#[derive(Debug)]
pub enum SqlizerError {
    IoError(std::io::Error),
    DockerError(String),
    OutputMismatch,
}

impl fmt::Display for SqlizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlizerError::IoError(e) => write!(f, "IO error: {}", e),
            SqlizerError::DockerError(e) => write!(f, "Docker error: {}", e),
            SqlizerError::OutputMismatch => write!(f, "Query output did not match expected output"),
        }
    }
}


pub struct Sqlizer {
    box_id: u128,
    sql_dir: PathBuf,
}

impl fmt::Display for Sqlizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sqlizer #{}", self.box_id)
    }
}

impl Sqlizer {
    pub fn new(code: Code) -> Result<Self, SqlizerError> {
        let box_id = code.id % 1000;
        let sql_dir = PathBuf::from(format!("/tmp/sqlizer/{}", box_id));

        fs::create_dir_all(&sql_dir)
            .map_err(SqlizerError::IoError)?;

        let paths = [
            ("query.sql", &code.source_code),
            ("correct_query.sql", &code.correct_query_code),
            ("table.sql", &code.stdin),
        ];

        // Initialize files
        for (filename, content) in paths.iter() {
            fs::write(sql_dir.join(filename), content)
                .map_err(SqlizerError::IoError)?;
        }

        // Copy script file
        let script = fs::read_to_string("./sql_script.sh")
            .map_err(SqlizerError::IoError)?;
        fs::write(sql_dir.join("sql_script.sh"), script)
            .map_err(SqlizerError::IoError)?;

        Ok(Sqlizer {
            box_id,
            sql_dir,
        })
    }

    pub fn run(&self) -> Result<Metadata, SqlizerError> {
        let output = Command::new("docker")
            .arg("run")
            .arg("-v")
            .arg(format!("{}:/sqlizer", self.sql_dir.display()))
            .arg("-v")
            .arg("/data:/data")
            .arg("sqlizer")
            .output()
            .map_err(|e| SqlizerError::DockerError(e.to_string()))?;

        if !output.status.success() {
            return Err(SqlizerError::DockerError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        let read_file = |filename: &str| -> Result<String, SqlizerError> {
            fs::read_to_string(self.sql_dir.join(filename))
                .map_err(SqlizerError::IoError)
        };

        let user_output = read_file("user_output.sql")?;
        let expected_output = read_file("expected_output.sql")?;
        let error_output = read_file("error_output.sql")?;
        let metadata_output = read_file("metadata.json")?;

        let result = user_output == expected_output;

        let mut metadata = Metadata {
            time: 0.0,
            wall_time: 0.0,
            memory: 0,
            std_out: user_output,
            std_err: error_output,
            exit_code: if result { 0 } else { 1 },
            exit_signal: 0,
            message: if result { "Correct" } else { "Incorrect" }.to_string(),
            status: if result { "Correct" } else { "Incorrect" }.to_string(),
        };

        // Parse metadata output
        for line in metadata_output.lines() {
            let mut parts = line.split(':');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                match key.trim() {
                    "User time (seconds)" => metadata.time = value.trim().parse().unwrap_or(0.0),
                    "System time (seconds)" => metadata.wall_time = value.trim().parse().unwrap_or(0.0),
                    "Maximum resident set size (kbytes)" => metadata.memory = value.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        Ok(metadata)
    }
}