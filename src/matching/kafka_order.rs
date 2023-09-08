use crate::utils::kafka::DefaultConsumer;
use crate::utils::kafka::new_kafka_reader;
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
    pub order_reader: DefaultConsumer,
}

impl KafkaOrderReader {
    pub fn new_kafka_order_reader(&mut self, brokers: &Vec<String>, product_id: &String, time_out: u64) -> Option<KafkaError> {
        self.topic = vec![TOPIC_ORDER_PREFIX.to_owned(), product_id.clone()].join("");
        return match new_kafka_reader(
            brokers,
            &self.topic,
            time_out,
        ) {
            Ok(dcc) => {
                self.order_reader = dcc;
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

        return match self.order_reader.seek(
            &self.topic,
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