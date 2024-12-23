
use rand::Rng;
use redis::Commands;

use crate::client::redis::redis_client;

pub fn create_job(
    code: &str,
    language: &str,
    input: &str,
    expected: &str,
    time_limit: u64,
    memory_limit: u64,
    stack_limit: u64,
) -> String {
    let job_id = rand::thread_rng().gen_range(1..1000).to_string();

    let client = redis_client();
    let mut conn = client.get_connection().unwrap();

    let job = serde_json::json!({
        "_id": job_id,
        "_code": code,
        "_language": language,
        "_input": input,
        "_status": "pending",
        "_expected": expected,
        "_result": "",
        "_error": "",
        "_time": time_limit,
        "_memory": memory_limit,
        "_stack": stack_limit,
        "_created_at": std::time::Instant::now().elapsed().as_nanos(),
    });

    let job = serde_json::to_string(&job).unwrap();
    let _: () = conn.set(job_id.clone(), &job).unwrap();
    let _: () = conn.rpush("jobs", job).unwrap();

    job_id
}

pub fn check_job(job_id: &str) -> Result<String, String> {
    let client = redis_client();
    let mut conn = client.get_connection().unwrap();

    let job: Result<String, redis::RedisError> = conn.get(job_id);

    match job {
        Ok(job) => Ok(job),
        Err(_) => Err("Job not found".to_string()),
    }
}
