use crate::{client::redis::RedisClient, core::job::Job, vendors::isolate::IsolateExecutor};
use futures::StreamExt;
use std::sync::Arc;

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
                // Process job here
                if let Some(mut job) = job {
                    if let Err(e) = executor.execute(&mut job).await {
                        panic!("Job execution failed: {:?}", e);
                    }
                }
            });
        }
    }
}
