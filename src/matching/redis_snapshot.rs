use bytes::Bytes;
use mini_redis::client::Client;
use serde_json;
use std::result::Result;

use crate::matching::engine::Snapshot;
use crate::utils::error::CustomError;
use crate::utils::redis::new_redis_client;

const TOPIC_SNAPSHOT_PREFIX: &str = "matching_snapshot_";

pub struct RedisSnapshotStore {
    pub product_id: String,
    pub redis_client: Client,
}

impl RedisSnapshotStore {
    pub async fn new_redis_snapshot_store(
        product_id: &str,
        ip: &str,
        port: u16,
    ) -> Result<RedisSnapshotStore, CustomError> {
        return match new_redis_client(ip, port).await {
            Ok(rc) => Ok(RedisSnapshotStore {
                product_id: product_id.to_string(),
                redis_client: rc,
            }),
            Err(e) => Err(CustomError::new(e.as_ref())),
        };
    }

    pub async fn store(&mut self, snapshot: &Snapshot) -> Option<CustomError> {
        return match serde_json::to_string(snapshot) {
            Ok(s) => {
                match self
                    .redis_client
                    .set(
                        &*format!("{}{}", TOPIC_SNAPSHOT_PREFIX, self.product_id),
                        Bytes::from(s.clone()),
                    )
                    .await
                {
                    Ok(_) => None,
                    Err(e) => Some(CustomError::new(e.as_ref())),
                }
            }
            Err(e) => Some(CustomError::new(&e)),
        };
    }

    pub async fn get_latest(&mut self) -> Result<Option<Snapshot>, CustomError> {
        return match self
            .redis_client
            .get(&*format!("{}{}", TOPIC_SNAPSHOT_PREFIX, self.product_id))
            .await
        {
            Ok(s) => match s {
                Some(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => match serde_json::from_str(&s) {
                        Ok(snapshot) => Ok(Some(snapshot)),
                        Err(e) => Err(CustomError::new(&e)),
                    },
                    Err(e) => Err(CustomError::new(&e)),
                },
                None => Ok(None),
            },
            Err(e) => Err(CustomError::new(e.as_ref())),
        };
    }
}
