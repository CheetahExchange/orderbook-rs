use redis::Client;

pub async fn new_redis_client(ip: &str, port: u16) -> redis::RedisResult<Client> {
    let client = Client::open(format!("redis://{ip}:{port}"))?;
    // Verify connection
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(client)
}
