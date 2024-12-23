use redis::{self, Client};
use std::sync::Arc;

pub fn redis_client()-> Arc<Client>{
    let client = redis::Client::open("redis://redis:6379").unwrap();
    let client = Arc::new(client);
    client
}