use axum::{
    extract::{Path, Json},
    http::StatusCode,
    routing::{get, post}, Router,
};
use serde_json::json;

use crate::utils::utils::{check_job, create_job};

pub fn server() -> Router {
    Router::new()
        .route("/create", post(handle_create))
        .route("/check/:job_id", get(handle_check))
}


async fn handle_create(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("Creating job");
    let code = body["code"].as_str().ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let language = body["language"].as_str().ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let input = body["input"].as_str().unwrap_or_default();
    let expected = body["expected"].as_str().unwrap_or_default();
    let time_limit = body["time_limit"].as_u64().unwrap_or(1);
    let memory_limit = body["memory_limit"].as_u64().unwrap_or(16);
    let stack_limit = body["stack_limit"].as_u64().unwrap_or(16);

    // Call the utility function to create the job
    let job=create_job(
        code,
        language,
        input,
        expected,
        time_limit,
        memory_limit,
        stack_limit,
    );

    Ok(Json(json!({ "status": "created", "id": job })))
}

async fn handle_check(Path(job_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("Checking job {}", job_id);
    match check_job(&job_id) {
        Ok(job) => Ok(Json(json!(job))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
