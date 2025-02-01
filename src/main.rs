mod client;
mod core;
mod utils;
mod vendors;
mod worker;

use crate::client::redis::RedisClient;
use crate::core::server::server;
use crate::worker::Worker;
use tokio;

#[tokio::main]
async fn main() {
    // Initialize Redis client
    let redis_client = RedisClient::new("redis://127.0.0.1/")
        .expect("Failed to connect to Redis");

    // Start the worker
    let worker = Worker::new(redis_client.clone());
    worker.start(20).await;
    

    // Start the server
    let app = server(redis_client);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();

    println!("Server running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}