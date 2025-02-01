use std::{collections::HashMap, fs::File, io::Write, process::Stdio};
use tokio::process::Command;

use axum::{http::StatusCode, Json};
use tempfile::NamedTempFile;


#[derive(Debug, serde::Serialize, serde::Deserialize,)]
struct ExecutionOutput {
    stdout: String,
    stderr: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DebugRequest {
    code: String,
    language: String,
    input: String,
}

#[derive(serde::Serialize, Debug, serde::Deserialize)]
struct DebugStep {
    step: usize,
    line: u32,
    variables: HashMap<String, String>,
    stdout: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DebugResponse {
    steps: Vec<DebugStep>,
}

#[derive(Debug)]
enum ExecutionError {
    DebugParseError,
    ExecutionFailed
}

pub async fn debug(
    Json(payload): Json<DebugRequest>,
) -> Result<Json<DebugResponse>, StatusCode> {
    // Validate language
    if payload.language != "python" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Execute code with debug tracing
    let steps = execute_with_debug(&payload.code, &payload.input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DebugResponse { steps }))
}

async fn execute_with_debug(code: &str, input: &str) -> Result<Vec<DebugStep>, ExecutionError> {
 tokio::fs::write("debugger/temp.py", code)
     .await
     .map_err(|_| ExecutionError::DebugParseError)?;

 let file_path = "debugger/temp.py";
    

 // Call the Python script
 let output = Command::new("python3")
     .arg("./debugger/debug.py")
     .arg(file_path)
     .stdout(Stdio::piped())
     .stderr(Stdio::piped())
     .output()
     .await
     .map_err(|_| ExecutionError::ExecutionFailed)?;

println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
println!("stderr: {}", String::from_utf8_lossy(&output.stderr));


 // Parse the output
 let stdout = String::from_utf8(output.stdout).map_err(|_| ExecutionError::ExecutionFailed)?;
 let steps: Vec<DebugStep> = serde_json::from_str(&stdout)
     .map_err(|_| ExecutionError::DebugParseError)?;

 Ok(steps)
}


async fn run_python_code(code: &str, input: &str) -> Result<ExecutionOutput, ExecutionError> {
    // Create a temporary file to store the Python code
    let wrapper_path="debugger/debug.py";
    let code_path="debugger/temp.py";
    let input_path="debugger/temp_input.txt";
    tokio::fs::write(code_path, code)
        .await
        .map_err(|_| ExecutionError::DebugParseError)?;
    tokio::fs::write(input_path, input)
        .await
        .map_err(|_| ExecutionError::DebugParseError)?;


    let input_file=File::open(input_path).unwrap();

    let child = Command::new("python3")
        .arg(code_path)
        .stdin(input_file)
        .output()
        .await
        .map_err(|_| ExecutionError::DebugParseError)?;

    println!("stdout: {}", String::from_utf8_lossy(&child.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&child.stderr));

    Ok(ExecutionOutput {
        stdout: String::from_utf8_lossy(&child.stdout).to_string(),
        stderr: String::from_utf8_lossy(&child.stderr).to_string(),
    })
}
