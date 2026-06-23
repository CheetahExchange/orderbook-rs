use std::result::Result;

use log::error;
use rdkafka::producer::FutureRecord;
use rdkafka::util::Timeout;

use crate::matching::log::LogTrait;
use crate::utils::error::CustomError;
use crate::utils::kafka::{new_kafka_producer, DefaultProducer};

const TOPIC_BOOK_MESSAGE_PREFIX: &str = "matching_message_";

pub struct KafkaLogStore {
    pub topic: String,
    pub log_producer: DefaultProducer,
}

impl KafkaLogStore {
    pub fn new_kafka_log_producer(
        brokers: &[String],
        product_id: &str,
        message_time_out: u64,
    ) -> Result<KafkaLogStore, CustomError> {
        match new_kafka_producer(brokers, message_time_out) {
            Ok(dp) => Ok(KafkaLogStore {
                topic: [TOPIC_BOOK_MESSAGE_PREFIX, product_id].join(""),
                log_producer: dp,
            }),
            Err(e) => Err(CustomError::new(&e)),
        }
    }

    pub async fn store(&self, logs: &[Box<dyn LogTrait>]) -> Result<(), CustomError> {
        for log in logs {
            let s = serde_json::to_string(log)
                .map_err(|e| CustomError::new(&e))?;

            // send() returns OwnedDeliveryResult = Result<(i32, i64), (KafkaError, OwnedMessage)>
            let delivery_result = self
                .log_producer
                .send(
                    FutureRecord::to(&self.topic).payload(&s).key(""),
                    Timeout::Never,
                )
                .await;

            // Check delivery result: Ok((partition, offset)) or Err((KafkaError, OwnedMessage))
            match delivery_result {
                Ok((_partition, _offset)) => {
                    // Message successfully delivered to Kafka
                }
                Err((kafka_error, _owned_message)) => {
                    error!("Kafka delivery failed for log seq {}: {:?}", log.get_seq(), kafka_error);
                    return Err(CustomError::from_string(format!(
                        "Kafka delivery failed: {}",
                        kafka_error
                    )));
                }
            }
        }
        Ok(())
    }
}
