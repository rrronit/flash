use redis::{AsyncCommands, RedisResult};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::core::job;

#[derive(Clone)]
pub struct RedisClient {
    client: redis::Client,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    pub async fn get_connection(&self) -> RedisResult<redis::aio::Connection> {
        self.client.get_async_connection().await
    }

    pub async fn store_job<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> RedisResult<()> {
        let mut conn = self.get_connection().await?;
        let serialized = serde_json::to_string(value).unwrap(); // Serialize to JSON string

        if let Some(ttl) = ttl {
            conn.set_ex(key, serialized, ttl.as_secs()).await
        } else {
            conn.set(key, serialized).await
        }
    }

    pub async fn get_job<T: DeserializeOwned>(&self, key: &str) -> RedisResult<Option<T>> {
        let mut conn = self.get_connection().await?;
        let data: Option<String> = conn.get(key).await?;
        
        data.map(|d| serde_json::from_str(&d))
            .transpose()
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "deserialization failed", e.to_string())))
    }

    pub async fn enqueue_job<T: Serialize>(&self, queue: &str, value: &T) -> RedisResult<()> {
        let mut conn = self.get_connection().await?;
        let serialized = serde_json::to_string(value).unwrap();
        conn.rpush(queue, serialized).await
    }

    pub async fn get_job_from_queue<T: DeserializeOwned>(&self, queue: &str) -> RedisResult<Option<T>> {
        let mut conn = self.get_connection().await?;
        let job_data: Option<String> = conn.lpop(queue, None).await?;

        if let Some(data) = job_data {
            let job: T = serde_json::from_str(&data)
                .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "deserialization failed", e.to_string())))?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }
}