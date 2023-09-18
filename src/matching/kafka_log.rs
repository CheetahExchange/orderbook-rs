use crate::utils::kafka::{new_kafka_producer, DefaultProducer};
use std::ops::Deref;

use crate::matching::log::Log;
use crate::utils::error::CustomError;
use rdkafka::producer::FutureRecord;
use std::result::Result;
use std::time::Duration;

const TOPIC_BOOK_MESSAGE_PREFIX: &str = "matching_order_";

pub struct KafkaLogStore {
    pub topic: String,
    pub log_producer: DefaultProducer,
}

impl KafkaLogStore {
    pub fn new_kafka_log_producer(
        brokers: &[&str],
        product_id: &str,
        time_out: u64,
    ) -> Result<KafkaLogStore, CustomError> {
        return match new_kafka_producer(brokers, time_out) {
            Ok(dp) => Ok(KafkaLogStore {
                topic: String::from(&[TOPIC_BOOK_MESSAGE_PREFIX, product_id].join("")),
                log_producer: dp,
            }),
            Err(e) => Err(CustomError::new(&e)),
        };
    }

    pub async fn store(&self, logs: &Vec<Box<dyn Log>>) -> Option<CustomError> {
        for log in logs {
            match serde_json::to_string(log) {
                Ok(s) => {
                    _ = self
                        .log_producer
                        .send(
                            FutureRecord::to(&self.topic).payload(&s).key(""),
                            Duration::from_secs(5),
                        )
                        .await;
                }
                Err(e) => {
                    return Some(CustomError::new(&e));
                }
            }
        }
        None
    }
}
