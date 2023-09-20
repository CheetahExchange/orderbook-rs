use crate::matching::log::LogTrait;
use crate::utils::error::CustomError;
use rdkafka::producer::FutureRecord;
use rdkafka::util::Timeout;
use std::result::Result;
use std::time::Duration;

use crate::utils::kafka::{new_kafka_producer, DefaultProducer};

const TOPIC_BOOK_MESSAGE_PREFIX: &str = "matching_order_";

pub struct KafkaLogStore {
    pub topic: String,
    pub log_producer: DefaultProducer,
}

impl KafkaLogStore {
    pub fn new_kafka_log_producer(
        brokers: &[&str],
        product_id: &str,
        message_time_out: u64,
    ) -> Result<KafkaLogStore, CustomError> {
        return match new_kafka_producer(brokers, message_time_out) {
            Ok(dp) => Ok(KafkaLogStore {
                topic: String::from(&[TOPIC_BOOK_MESSAGE_PREFIX, product_id].join("")),
                log_producer: dp,
            }),
            Err(e) => Err(CustomError::new(&e)),
        };
    }

    pub async fn store(&self, logs: &Vec<Box<dyn LogTrait>>) -> Result<(), CustomError> {
        for log in logs {
            match serde_json::to_string(log) {
                Ok(s) => {
                    _ = self
                        .log_producer
                        .send(
                            FutureRecord::to(&self.topic).payload(&s).key(""),
                            Timeout::Never,
                        )
                        .await;
                }
                // json serde err
                Err(e) => return Err(CustomError::new(&e)),
            }
        }
        Ok(())
    }
}
