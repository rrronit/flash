use std::time::{SystemTime, UNIX_EPOCH};

use crate::{client::redis::RedisClient, core::job::Job};
use serde_json;

/// Creates a new job and stores it in Redis.
pub async fn create_job(redis: &RedisClient, job: Job) -> Result<String, String> {
    let job_id = job.id.to_string();


    redis
        .store_job(&job_id, &job, None)
        .await
        .map_err(|e| e.to_string())?;

    redis
        .enqueue_job("jobs", &job)
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

/// Retrieves a job from Redis by its ID.
pub async fn check_job(redis: &RedisClient, job_id: &str) -> Result<Job, String> {
    let data=redis
        .get_job(job_id)
        .await
        .map_err(|e| e.to_string());
    match data {
        Ok(Some(job)) => Ok(job),
        Ok(None) => Err("Job not found".to_string()),
        Err(e) => Err(e),
    }
}
