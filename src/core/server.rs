use crate::{
    client::redis::RedisClient,
    core::{job::Job, language::Language, settings::ExecutionSettings}, utils::utils::{check_job, create_job}, vendors::debugger,
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::sync::Arc;

pub fn server(redis_client: RedisClient) -> Router {
    Router::new()
        .route("/create", post(handle_create))
        .route("/check/:job_id", get(handle_check))
        .route("/debug", post(handle_debug))
        .with_state(Arc::new(redis_client))
}

#[derive(serde::Deserialize)]
struct CreateJobRequest {
    code: String,
    language: String,
    input: String,
    expected: String,
    time_limit: Option<f64>,
    memory_limit: Option<u64>,
    stack_limit: Option<u64>,
}

async fn handle_create(
    State(redis): State<Arc<RedisClient>>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("Creating job for language: {}", payload.language);

    let language = match payload.language.as_str() {
        "python" => Language {
            name: "python".to_string(),
            source_file: "main.py".to_string(),
            compile_cmd: None,
            run_cmd: "/usr/bin/python3 main.py".to_string(),
            is_compiled: false,
        },
        "cpp" => Language {
            name: "cpp".to_string(),
            source_file: "main.cpp".to_string(),
            compile_cmd: Some("/usr/bin/g++ main.cpp -o main".to_string()),
            run_cmd: "./main".to_string(),
            is_compiled: true,
        },
        "javascript" => Language {
            name: "javascript".to_string(),
            source_file: "main.js".to_string(),
            compile_cmd: None,
            run_cmd: "node".to_string(),
            is_compiled: false,
        },
        "sql" => Language {
            name: "sql".to_string(),
            source_file: "main.sql".to_string(),
            compile_cmd: None,
            run_cmd: "sqlite3".to_string(),
            is_compiled: false,
        },
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let settings = ExecutionSettings {
        cpu_time_limit: payload.time_limit.unwrap_or(2.0),
        memory_limit: payload.memory_limit.unwrap_or(128_000),
        stack_limit: payload.stack_limit.unwrap_or(64_000),
        ..Default::default()
    };

    let job = Job::new(payload.code, language)
        .with_stdin(payload.input)
        .with_expected_output(payload.expected)
        .set_limits(
            settings.cpu_time_limit,
            settings.memory_limit,
            settings.stack_limit,
            60,
        );

    let job_id = create_job(&redis, job)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "status": "created", "id": job_id })))
}

async fn handle_check(
    State(redis): State<Arc<RedisClient>>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("Checking job {}", job_id);

    let job = check_job(&redis, &job_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(json!(job)))
}

async fn handle_debug(Json(body): Json<debugger::DebugRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let response = debugger::debug(axum::Json(body)).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!(*response)))
}
