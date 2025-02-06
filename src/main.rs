mod client;
mod core;
mod utils;
mod vendors;
mod worker;

use crate::client::redis::RedisClient;
use crate::core::server::server;
use crate::worker::Worker;
use tokio;

#[tokio::main(flavor = "multi_thread", worker_threads = 20)] 
async fn main() {


    // Initialize Redis client
    let redis_client = RedisClient::new("redis://127.0.0.1/")
        .expect("Failed to connect to Redis");

    // Start the worker
    let worker_redis = redis_client.clone();
    tokio::spawn(async move {
        let worker = Worker::new(worker_redis);
        worker.start(20).await;
    });
    

    // Start the server
    let app = server(redis_client);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();

    println!("Server running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}