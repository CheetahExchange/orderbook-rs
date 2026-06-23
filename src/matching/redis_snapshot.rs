use redis::Client;
use redis::AsyncCommands;
use std::result::Result;

use crate::matching::engine::Snapshot;
use crate::utils::error::CustomError;
use crate::utils::redis::new_redis_client;

const TOPIC_SNAPSHOT_PREFIX: &str = "matching_snapshot_";

pub struct RedisSnapshotStore {
    pub product_id: String,
    pub snapshot_key: String,
    pub redis_client: Client,
}

impl RedisSnapshotStore {
    pub async fn new_redis_snapshot_store(
        product_id: &str,
        ip: &str,
        port: u16,
    ) -> Result<RedisSnapshotStore, CustomError> {
        match new_redis_client(ip, port).await {
            Ok(c) => Ok(RedisSnapshotStore {
                product_id: product_id.to_string(),
                snapshot_key: [TOPIC_SNAPSHOT_PREFIX, product_id].join(""),
                redis_client: c,
            }),
            Err(e) => Err(CustomError::from_string(format!("{}", e))),
        }
    }

    pub async fn store(&mut self, snapshot: &Snapshot) -> Result<(), CustomError> {
        let s = serde_json::to_string(snapshot)
            .map_err(|e| CustomError::new(&e))?;

        let mut conn = self.redis_client.get_multiplexed_async_connection().await
            .map_err(|e| CustomError::from_string(format!("{}", e)))?;

        conn.set::<_, _, ()>(&self.snapshot_key, &s).await
            .map_err(|e| CustomError::from_string(format!("{}", e)))?;

        Ok(())
    }

    pub async fn get_latest(&mut self) -> Result<Option<Snapshot>, CustomError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await
            .map_err(|e| CustomError::from_string(format!("{}", e)))?;

        let result: Option<String> = conn.get(&self.snapshot_key).await
            .map_err(|e| CustomError::from_string(format!("{}", e)))?;

        match result {
            Some(s) => {
                let snapshot: Snapshot = serde_json::from_str(&s)
                    .map_err(|e| CustomError::new(&e))?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }
}
