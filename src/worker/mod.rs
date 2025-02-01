use crate::{client::redis::RedisClient, core::job::Job, vendors::isolate::IsolateExecutor};
use std::sync::Arc;
use tokio::task;

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

    pub async fn start(&self, n: i8) {
        (0..n).for_each(|i| {
            let isolate_executor = self.isolate_executor.clone();
            let redis = self.redis.clone();
            task::spawn(async move {
                loop {
                    if let Ok(Some(mut job)) = redis.get_job_from_queue::<Job>("jobs").await {
                        let isolate_executor = isolate_executor.clone();
                        task::spawn(async move {
                            let result = isolate_executor.execute(&mut job).await;
                            if let Err(e) = result {
                                eprintln!("Job processing failed: {}", e);
                            }
                        });
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            });
        });
    }
}
