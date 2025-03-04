use crate::{client::redis::RedisClient, core::job::Job, vendors::isolate::IsolateExecutor};
use tokio::task;
use std::{process::Command, sync::Arc, time::Duration};

pub struct Worker {
    redis: Arc<RedisClient>,
    isolate_executor: IsolateExecutor,
}

impl Worker {
    pub fn new(redis: RedisClient) -> Self {
        Self {
            redis: Arc::new(redis.clone()),
            isolate_executor: IsolateExecutor::new(redis),
        }
    }

    pub async fn start(&self, concurrency: usize) {
        let mut handles = Vec::with_capacity(concurrency);
        
        for _ in 0..concurrency {
            let redis = Arc::clone(&self.redis);
            let executor = self.isolate_executor.clone();
            
            let handle = task::spawn(async move {
                loop {
                    match redis.get_job_from_queue::<Job>("jobs").await {
                        Ok(Some(mut job)) => {
                            
                            let max_retries = 3;
                            let mut retry_count = 0;
                            
                            loop {
                                let result = executor.execute(&mut job).await;

                                match result {
                                    Ok(_) => {
                                        cleanup_job(job.id).await;
                                        break;
                                    }
                                    Err(e) => {
                                        println!("Job {} failed: {:?}", job.id, e);
                                        retry_count += 1;
                                        cleanup_job(job.id).await;
                                        if retry_count >= max_retries {
                                            println!("Job {} failed after {} retries", job.id, max_retries);
                                            break;
                                        }
                                    }
                                }
                            }
                        },
                        Ok(None) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        },
                        Err(e) => {
                            eprintln!("Error fetching job from queue: {:?}", e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all worker tasks
        for handle in handles {
            if let Err(e) = handle.await {
                eprintln!("Worker task failed: {:?}", e);
            }
        }
    }
}

async fn cleanup_job(job_id: u64) {
    let box_id = job_id % 2147483647;
    println!("cleaning {}", box_id);
    if let Err(e) = Command::new("isolate")
        .args(&["--cg", "-b", &box_id.to_string(), "--cleanup"])
        .output()
    {
        eprintln!("Failed to cleanup isolate box {}: {:?}", box_id, e);
    }
}