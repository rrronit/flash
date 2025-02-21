use serde_json::Value;
use std::process::Stdio;
use axum::{http::StatusCode, Json};
use tokio::process::Command;
use tokio::fs;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct DebugRequest {
    pub code: String,
    pub language: String,
    pub input: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct DebugStep {
    line: u32,
    code: String,
    locals: Value, // Use serde_json::Value to handle arbitrary JSON
    stdout: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct DebugResponse {
    steps: Vec<DebugStep>,
}

#[derive(Debug)]
enum ExecutionError {
    DebugParseError,
    ExecutionFailed,
}

pub async fn debug(payload: DebugRequest) -> Result<Json<DebugResponse>, StatusCode> {
    let steps = execute_with_debug(&payload.code, &payload.input)
        .await
        .map_err(|e| {
            eprintln!("Debug error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(steps))
}

async fn execute_with_debug(code: &str, _input: &str) -> Result<DebugResponse, ExecutionError> {
    // Write the code to a temporary file
    fs::write("debugger/temp.py", code)
        .await
        .map_err(|e| {
            eprintln!("Failed to write temp file: {:?}", e);
            ExecutionError::DebugParseError
        })?;

    let file_path = "debugger/temp.py";

    // Execute the Python debugger
    let output = Command::new("python3")
        .arg("./debugger/debug.py")
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            eprintln!("Failed to execute python: {:?}", e);
            ExecutionError::ExecutionFailed
        })?;

    // Debug logging
    eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Parse the debug steps directly from stdout
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| {
            eprintln!("UTF-8 conversion error: {:?}", e);
            ExecutionError::DebugParseError
        })?;

    let steps: DebugResponse = serde_json::from_str(&stdout)
        .map_err(|e| {
            eprintln!("JSON parse error: {:?}", e);
            ExecutionError::DebugParseError
        })?;

    Ok(steps)
}