use bincode;
use deadpool_redis::{redis, Config, Connection, Pool, Runtime};
use redis::{AsyncCommands, RedisError, RedisResult};
use serde::{de::DeserializeOwned, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct RedisClient {
    pool: Pool,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| RedisError::from((redis::ErrorKind::IoError, "Pool creation error check the url")))?;
        Ok(Self { pool })
    }

    async fn get_conn(&self) -> RedisResult<Connection> {
        self.pool
            .get()
            .await
            .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "getting connection error")))
    }

    pub async fn store_job<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        let serialized = bincode::serialize(value).unwrap();


        if let Some(ttl) = ttl {
            conn.set_ex(key, serialized, ttl.as_secs() as usize).await
        } else {
            conn.set(key, serialized).await
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_job<T: DeserializeOwned>(&self, key: &str) -> RedisResult<Option<T>> {
        let mut conn = self.get_conn().await?;
        let data: Option<Vec<u8>> = conn.get(key).await?;

        data.map(|d| bincode::deserialize(&d))
            .transpose()
            .map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "deserialization failed",
                    e.to_string(),
                ))
            })
    }

    pub async fn enqueue_job<T: Serialize>(&self, queue: &str, value: &T) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        let serialized = bincode::serialize(value).unwrap(); // Serialize using bincode
        conn.rpush(queue, serialized).await
    }

    pub async fn get_job_from_queue<T: DeserializeOwned>(
        &self,
        queue: &str,
    ) -> RedisResult<Option<T>> {
        let mut conn = self.get_conn().await?;

        // Use BRPOP with 1-second timeout to block until job arrives
        let result: Option<(String, Vec<u8>)> = conn.brpop(queue, 10).await?;

        match result {
            Some((_list_name, data)) => {
                // Deserialize the binary data
                let job = bincode::deserialize(&data).map_err(|e| {
                    redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "deserialization failed",
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

    pub async fn create_job<T: Serialize>(
        &self,
        key: &str,
        queue: &str,
        value: &T,
    ) -> RedisResult<()> {
        let mut conn = self.get_conn().await?;
        let serialized = bincode::serialize(value).unwrap(); // Serialize using bincode

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
