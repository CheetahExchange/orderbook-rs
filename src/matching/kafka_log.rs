use crate::utils::kafka::{DefaultProducer, new_kafka_producer};
use rdkafka::error::KafkaError;

const TOPIC_BOOK_MESSAGE_PREFIX: &str = "matching_order_";


pub struct KafkaLogStore {
    pub topic: String,
    pub log_producer: DefaultProducer,
}

impl KafkaLogStore {
    pub fn new_kafka_log_producer(&mut self, brokers: &[&str], product_id: &str, time_out: u64) -> Option<KafkaError> {
        self.topic = String::from(&[TOPIC_BOOK_MESSAGE_PREFIX, product_id].join(""));
        return match new_kafka_producer(
            brokers,
            time_out,
        ) {
            Ok(dp) => {
                self.log_producer = dp;
                None
            }
            Err(e) => {
                Some(e)
            }
        };
    }
}