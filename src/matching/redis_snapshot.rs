#![warn(rust_2018_idioms)]

use mini_redis::client::{Client, connect};
use mini_redis::Result;

use crate::utils::redis::new_redis_client;

const TOPIC_SNAPSHOT_PREFIX: &str = "matching_snapshot_";


pub struct RedisSnapshotStore {
    pub product_id: String,
    pub redis_client: Client,
}


impl RedisSnapshotStore {
    pub async fn new_redis_snapshot_store(product_id: &str, ip: &str, port: u16) -> Result<RedisSnapshotStore> {
        return match new_redis_client(ip, port).await {
            Ok(rc) =>
                Ok(RedisSnapshotStore {
                    product_id: product_id.to_string(),
                    redis_client: rc,
                }),
            Err(e) => Err(e),
        };
    }
}