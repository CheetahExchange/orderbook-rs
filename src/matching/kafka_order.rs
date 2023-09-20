use rdkafka::consumer::Consumer;
use rdkafka::util::Timeout;
use rdkafka::{Message, Offset};
use std::result::Result;
use std::time::Duration;

use crate::models::models::Order;
use crate::utils::error::CustomError;
use crate::utils::kafka::new_kafka_consumer;
use crate::utils::kafka::DefaultConsumer;

const TOPIC_ORDER_PREFIX: &str = "matching_order_";

pub struct KafkaOrderReader {
    pub topic: String,
    pub order_consumer: DefaultConsumer,
}

impl KafkaOrderReader {
    pub fn new_kafka_order_consumer(
        brokers: &[&str],
        product_id: &str,
        session_time_out: u64,
    ) -> Result<KafkaOrderReader, CustomError> {
        let topic = String::from(&[TOPIC_ORDER_PREFIX, product_id].join(""));
        return match new_kafka_consumer(brokers, topic.as_str(), session_time_out) {
            Ok(dc) => Ok(KafkaOrderReader {
                topic,
                order_consumer: dc,
            }),
            Err(e) => Err(CustomError::new(&e)),
        };
    }

    pub fn set_offset(&mut self, offset: Offset, time_out: u64) -> Option<CustomError> {
        return match self.order_consumer.seek(
            self.topic.as_str(),
            0,
            offset,
            Timeout::After(Duration::from_secs(time_out)),
        ) {
            Ok(_) => None,
            Err(e) => Some(CustomError::new(&e)),
        };
    }

    pub async fn fetch_message(&mut self) -> (i64, Option<Vec<u8>>, Option<CustomError>) {
        return match self.order_consumer.recv().await {
            Err(e) => (0, None, Some(CustomError::new(&e))),
            Ok(message) => match message.payload() {
                None => (0, None, None),
                Some(payload) => (message.offset(), Some(payload.to_vec()), None),
            },
        };
    }

    pub async fn fetch_order(&mut self) -> (i64, Option<Order>, Option<CustomError>) {
        let (offset, payload, err) = self.fetch_message().await;
        match err {
            Some(e) => {
                return (0, None, Some(e));
            }
            _ => {}
        }
        return match payload {
            None => (0, None, None),
            Some(v) => match serde_json::from_slice(&v) {
                Err(e) => (0, None, Some(CustomError::new(&e))),
                Ok(order) => (offset, order, None),
            },
        };
    }
}
