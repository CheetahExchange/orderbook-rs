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
    pub snapshot_key: String,
    pub redis_client: Client,
}

impl RedisSnapshotStore {
    pub async fn new_redis_snapshot_store(
        product_id: &str,
        ip: &str,
        port: u16,
    ) -> Result<RedisSnapshotStore, CustomError> {
        return match new_redis_client(ip, port).await {
            Ok(c) => Ok(RedisSnapshotStore {
                product_id: product_id.to_string(),
                snapshot_key: String::from(&[TOPIC_SNAPSHOT_PREFIX, product_id].join("")),
                redis_client: c,
            }),
            // redis connect err
            Err(e) => Err(CustomError::new(e.as_ref())),
        };
    }

    pub async fn store(&mut self, snapshot: &Snapshot) -> Result<(), CustomError> {
        return match serde_json::to_string(snapshot) {
            Ok(s) => {
                match self
                    .redis_client
                    .set(self.snapshot_key.as_str(), Bytes::from(s))
                    .await
                {
                    Ok(_) => Ok(()),
                    // redis set err
                    Err(e) => Err(CustomError::new(e.as_ref())),
                }
            }
            // json serde err
            Err(e) => Err(CustomError::new(&e)),
        };
    }

    pub async fn get_latest(&mut self) -> Result<Option<Snapshot>, CustomError> {
        return match self.redis_client.get(self.snapshot_key.as_str()).await {
            Ok(obs) => match obs {
                Some(bs) => match String::from_utf8(bs.to_vec()) {
                    Ok(s) => match serde_json::from_str(&s) {
                        Ok(snapshot) => Ok(Some(snapshot)),
                        // json obj from str err
                        Err(e) => Err(CustomError::new(&e)),
                    },
                    //bytes to utf8 str err
                    Err(e) => Err(CustomError::new(&e)),
                },
                // redis get none
                None => Ok(None),
            },
            // redis get err
            Err(e) => Err(CustomError::new(e.as_ref())),
        };
    }
}
