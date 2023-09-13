#![warn(rust_2018_idioms)]

use mini_redis::client::{Client, connect};
use mini_redis::Result;

pub async fn new_redis_client(ip: &str, port: u16) -> Result<Client> {
    let client = connect(&format!("{}:{}", ip, port)).await?;
    Ok(client)
}