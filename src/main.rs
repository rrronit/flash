use flash::core::{_thread_pool, server::server};

#[tokio::main]
async fn main() {
    let app = server();
    _thread_pool(2);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
