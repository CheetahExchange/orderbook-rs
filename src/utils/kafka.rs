use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::DefaultConsumerContext;
use rdkafka::error::KafkaResult;
use rdkafka::producer::DefaultProducerContext;
use rdkafka::producer::FutureProducer;

pub type DefaultConsumer = StreamConsumer<DefaultConsumerContext>;
pub type DefaultProducer = FutureProducer<DefaultProducerContext>;

pub fn new_kafka_producer(
    brokers: &Vec<String>,
    message_time_out: u64,
) -> KafkaResult<DefaultProducer> {
    let producer: DefaultProducer = ClientConfig::new()
        .set("bootstrap.servers", brokers.join(","))
        .set(
            "message.timeout.ms",
            &format!("{}", message_time_out * 1000),
        )
        .create_with_context(DefaultProducerContext)?;

    Ok(producer)
}

pub fn new_kafka_consumer(
    brokers: &Vec<String>,
    group_id: &str,
    topic: &str,
    session_time_out: u64,
) -> KafkaResult<DefaultConsumer> {
    let consumer: DefaultConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers.join(","))
        .set("group.id", group_id)
        .set("enable.partition.eof", "false")
        .set(
            "session.timeout.ms",
            &format!("{}", session_time_out * 1000),
        )
        .set("enable.auto.commit", "true")
        .create_with_context(DefaultConsumerContext)?;

    consumer.subscribe(&[topic])?;

    Ok(consumer)
}
