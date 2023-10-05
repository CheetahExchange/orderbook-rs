use std::result::Result;
use std::time::Duration;

use rdkafka::consumer::Consumer;
use rdkafka::error::RDKafkaErrorCode;
use rdkafka::{Message, Offset};
use tokio::time::timeout;

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
        brokers: &Vec<String>,
        group_id: &str,
        product_id: &str,
        session_time_out: u64,
    ) -> Result<KafkaOrderReader, CustomError> {
        let topic = String::from(&[TOPIC_ORDER_PREFIX, product_id].join(""));
        return match new_kafka_consumer(brokers, group_id, topic.as_str(), session_time_out) {
            Ok(dc) => Ok(KafkaOrderReader {
                topic,
                order_consumer: dc,
            }),
            Err(e) => Err(CustomError::new(&e)),
        };
    }

    pub async fn set_offset(&mut self, offset: Offset) -> Result<(), CustomError> {
        loop {
            return match self.order_consumer.assignment() {
                Ok(mut tpl) => {
                    if tpl.count() == 0 {
                        match timeout(Duration::from_secs(1), self.order_consumer.recv()).await {
                            Ok(r) => match r {
                                Ok(_) => continue,
                                Err(e) => {
                                    // if topic not exist, continue
                                    match e.rdkafka_error_code().unwrap() {
                                        RDKafkaErrorCode::UnknownTopicOrPartition => continue,
                                        _ => Err(CustomError::new(&e)),
                                    }
                                }
                            },
                            Err(_) => continue,
                        }
                    } else {
                        match tpl.set_all_offsets(offset) {
                            Ok(()) => match self.order_consumer.assign(&tpl) {
                                Ok(()) => Ok(()),
                                Err(e) => Err(CustomError::new(&e)),
                            },
                            Err(e) => Err(CustomError::new(&e)),
                        }
                    }
                }
                Err(e) => Err(CustomError::new(&e)),
            };
        }
    }

    pub async fn fetch_message(&mut self) -> Result<(i64, Option<Vec<u8>>), CustomError> {
        return match self.order_consumer.recv().await {
            // kafka consume err
            Err(e) => Err(CustomError::new(&e)),
            Ok(message) => match message.payload() {
                // payload is none
                None => Ok((0, None)),
                Some(payload) => Ok((message.offset(), Some(payload.to_vec()))),
            },
        };
    }

    pub async fn fetch_order(&mut self) -> Result<(i64, Option<Order>), CustomError> {
        let (offset, payload) = self.fetch_message().await?;

        return match payload {
            None => Ok((0, None)),
            Some(v) => match serde_json::from_slice(&v) {
                Ok(order) => Ok((offset, Some(order))),
                // json serde err
                Err(e) => Err(CustomError::new(&e)),
            },
        };
    }
}
