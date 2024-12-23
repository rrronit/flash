use std::process::Output;

use redis::Commands;

use crate::{client::redis::redis_client, vendors::Isolate};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Code {
    pub _id: String,
    pub _code: String,
    pub _language: String,
    pub _status: String,
    pub _input: String,
    pub _expected: String,
    pub _result: String,
    pub _error: String,
    pub _time: i32,
    pub _memory: i32,
    pub _stack: i32,
    pub _created_at: u128,
}

pub fn _thread_pool(_pool_count: i32) {
    let mut handles = Vec::new();

    for _ in 0.._pool_count {
        let client = redis_client();
        let handle = std::thread::spawn(move || {
            let mut conn = client.get_connection().unwrap();
            loop {
                let job: redis::RedisResult<Option<String>> = conn.lpop("jobs", None);
                match job {
                    Ok(job) => match job {
                        Some(job) => {
                            //parse code from job
                            let work: Code = serde_json::from_str(&job).unwrap();
                            // let work_id = work._id;
                            let result = process_job(work);
                            // if result.status.success() {
                            //     let output = String::from_utf8_lossy(&result.stdout).to_string();
                            //     let old_data: String = conn.get(work_id.to_string()).unwrap();
                            //     let mut old_work: Code = serde_json::from_str(&old_data).unwrap();
                            //     old_work._result = output;
                            //     old_work._status = "completed".to_string();
                            //     let new_data = serde_json::to_string(&old_work).unwrap();
                            //     let _: () = conn
                            //         .set_ex(work_id.to_string(), new_data, 10 * 60 * 60)
                            //         .unwrap();
                            // } else {
                            //     let output = String::from_utf8_lossy(&result.stderr).to_string();
                            //     let old_data: String = conn.get(work_id.to_string()).unwrap();
                            //     let mut old_work: Code = serde_json::from_str(&old_data).unwrap();
                            //     old_work._error = output;
                            //     old_work._status = "failed".to_string();
                            //     let new_data = serde_json::to_string(&old_work).unwrap();
                            //     let _: () = conn
                            //         .set_ex(work_id.to_string(), new_data, 10 * 60 * 60)
                            //         .unwrap();
                            // }
                            println!("{:?}", result);
                        }
                        None => {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    },
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            }
        });
        handles.push(handle);
    }
}

fn process_job(job: Code) -> Output {
    let mut isolate = Isolate::new(job);

    let output = isolate.compile();
    // let output = isolate.run();
    output.unwrap()
}
