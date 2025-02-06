use crate::{client::redis::RedisClient, core::job::Job, vendors::isolate::IsolateExecutor};
use futures::StreamExt;
use std::{process::Command, sync::Arc};

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
        let mut jobs_stream =
            futures::stream::repeat_with(|| self.redis.get_job_from_queue::<Job>("jobs"))
                .buffer_unordered(concurrency);

        while let Some(Ok(job)) = jobs_stream.next().await {
            let executor = self.isolate_executor.clone();

            tokio::spawn(async move {
                let max_retries = 3;
                let mut retry_count = 0;
                if let Some(mut job) = job {
                    loop {
                        let result = executor.execute(&mut job).await;
                        match result {
                            Ok(_) => {
                                println!("Job {} executed successfully", job.id);
                                break;
                            }
                            Err(e) => {
                                println!("Job {} failed: {:?}", job.id, e);
                                retry_count += 1;
                                if retry_count >= max_retries {
                                    println!("Job {} failed after {} retries", job.id, max_retries);
                                    break;
                                }
                            }
                        }
                        let box_id = job.id % 2147483647;
                        let _ = Command::new("isolate")
                            .args(&["-b", &box_id.to_string(), "--cleanup"])
                            .output()
                            .unwrap();
                    }
                }
            });
        }
    }
}
