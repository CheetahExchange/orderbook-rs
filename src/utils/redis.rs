use mini_redis::client::{connect, Client};
use mini_redis::Result;

pub async fn new_redis_client(ip: &str, port: u16) -> Result<Client> {
    let client = connect(&format!("{}:{}", ip, port)).await?;
    Ok(client)
}
