use crate::utils::kafka::{new_kafka_producer, DefaultProducer};

use crate::utils::error::CustomError;
use std::result::Result;

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
}
