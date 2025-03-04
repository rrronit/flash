use num_cpus;

use flash::client::redis::RedisClient;
use flash::core::server::server;
use flash::worker::Worker;
use tokio;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();
    let cpu_count: usize = num_cpus::get();
    println!("Number of CPU cores: {}", cpu_count);
    // Initialize Redis client
    let redis_client = RedisClient::new("redis://127.0.0.1/").expect("Failed to connect to Redis");

    // Start the worker
    let worker_redis = redis_client.clone();
    tokio::spawn(async move {
        let worker = Worker::new(worker_redis);
        worker.start(cpu_count * 2).await;
    });

    // Start the server
    let app = server(redis_client);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();

    println!("Server running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
