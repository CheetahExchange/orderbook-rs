use crate::utils::kafka::DefaultConsumer;
use crate::utils::kafka::new_kafka_consumer;
use rdkafka::error::{KafkaError};
use std::borrow::Borrow;
use rdkafka::consumer::Consumer;
use rdkafka::topic_partition_list::Offset::OffsetTail;
use rdkafka::Offset;
use rdkafka::util::Timeout;
use std::time::Duration;

const TOPIC_ORDER_PREFIX: &str = "matching_order_";


pub struct KafkaOrderReader {
    pub topic: String,
    pub order_consumer: DefaultConsumer,
}

impl KafkaOrderReader {
    pub fn new_kafka_order_consumer(&mut self, brokers: &[&str], product_id: &str, time_out: u64) -> Option<KafkaError> {
        self.topic = String::from(&[TOPIC_ORDER_PREFIX, product_id].join(""));
        return match new_kafka_consumer(
            brokers,
            self.topic.as_str(),
            time_out,
        ) {
            Ok(dc) => {
                self.order_consumer = dc;
                None
            }
            Err(e) => {
                Some(e)
            }
        };
    }

    pub fn set_offset(&mut self, offset: i64, time_out: u64) -> Option<KafkaError> {
        let offset =
            if offset == -1 as i64 {
                Offset::End
            } else {
                Offset::Offset(offset)
            };

        return match self.order_consumer.seek(
            self.topic.as_str(),
            0,
            offset,
            Timeout::After(Duration::from_secs(time_out)),
        ) {
            Ok(_) => {
                None
            }
            Err(e) => {
                Some(e)
            }
        };
    }
}