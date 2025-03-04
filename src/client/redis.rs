use bincode;
use deadpool_redis::{redis, Config, Connection, Pool, Runtime};
use futures::TryFutureExt;
use redis::{AsyncCommands, RedisError, RedisResult};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing;

#[derive(Clone)]
pub struct RedisClient {
    pool: Pool,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| {
                tracing::error!("Failed to create Redis pool: {:?}", e);
                RedisError::from((redis::ErrorKind::IoError, "Pool creation error", format!("Error: {}", e)))
            })?;
        
        // Test connection to confirm Redis is available
        let client = Self { pool };
        let _ = client.test_connection().map_err(|e| {
            tracing::error!("Redis connection test failed: {:?}", e);
            e
        });
        
        Ok(client)
}
    
    async fn test_connection(&self) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        redis::cmd("PING").query_async(&mut conn).await.map_err(|e| {
            tracing::error!("Redis PING failed: {:?}", e);
            e
        })
    }

    #[tracing::instrument(skip(self), level = "debug")]
    async fn get_conn(&self) -> RedisResult<Connection> {
        self.pool
            .get()
            .await
            .map_err(|e| {
                tracing::error!("Failed to get Redis connection: {:?}", e);
                RedisError::from((redis::ErrorKind::IoError, "Error getting connection", format!("Error: {}", e)))
            })
    }

    #[tracing::instrument(skip(self, value), level = "debug")]
    pub async fn store_job<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        let serialized = bincode::serialize(value).map_err(|e| {
            tracing::error!("Failed to serialize job: {:?}", e);
            RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", format!("Error: {}", e)))
        })?;

        if let Some(ttl) = ttl {
            conn.set_ex(key, serialized, ttl.as_secs() as usize).await
        } else {
            conn.set(key, serialized).await
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_job<T: DeserializeOwned>(&self, key: &str) -> RedisResult<Option<T>> {
        let mut conn = self.get_conn().await?;
        let data: Option<Vec<u8>> = conn.get(key).await?;

        data.map(|d| bincode::deserialize(&d))
            .transpose()
            .map_err(|e| {
                tracing::error!("Failed to deserialize job: {:?}", e);
                redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Deserialization failed",
                    e.to_string(),
                ))
            })
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_job_from_queue<T: DeserializeOwned>(
        &self,
        queue: &str,
    ) -> RedisResult<Option<T>> {
        let mut conn = self.get_conn().await?;

        // Use BRPOP with 1-second timeout to block until job arrives
        let result: Option<(String, Vec<u8>)> = conn.brpop(queue, 1).await?;

        match result {
            Some((_list_name, data)) => {
                // Deserialize the binary data
                let job = bincode::deserialize(&data).map_err(|e| {
                    tracing::error!("Failed to deserialize queue job: {:?}", e);
                    redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Deserialization failed",
                        e.to_string(),
                    ))
                })?;
                Ok(Some(job))
            }
            None => {
                Ok(None)
            }
        }
    }

    #[tracing::instrument(skip(self, value), level = "debug")]
    pub async fn create_job<T: Serialize>(
        &self,
        key: &str,
        queue: &str,
        value: &T,
    ) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        let serialized = bincode::serialize(value).map_err(|e| {
            tracing::error!("Failed to serialize job for queue: {:?}", e);
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        // Store the job in Redis and enqueue it
        redis::pipe()
            .atomic()
            .set(key, &serialized)
            .ignore()
            .rpush(queue, &serialized)
            .ignore()
            .query_async(&mut conn)
            .await
    }
}