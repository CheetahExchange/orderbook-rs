use rdkafka::config::ClientConfig;
use rdkafka::consumer::DefaultConsumerContext;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use std::borrow::Borrow;
use rdkafka::consumer::{Consumer};
use rdkafka::error::KafkaResult;

pub type DefaultConsumer = StreamConsumer<DefaultConsumerContext>;

pub fn new_kafka_consumer(brokers: &[&str], topic: &str, time_out: u64) -> KafkaResult<DefaultConsumer> {
    let consumer: DefaultConsumer =
        ClientConfig::new()
            .set("bootstrap.servers", brokers.join(","))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", &format!("{}", time_out * 1000))
            .set("enable.auto.commit", "true")
            .create_with_context(DefaultConsumerContext)?;

    consumer.subscribe(&[topic])?;

    Ok(consumer)
}