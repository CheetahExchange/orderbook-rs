#![feature(future_join)]
#![feature(let_chains)]

use crate::config::read_config;
use crate::matching::engine::Engine;
use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::redis_snapshot::RedisSnapshotStore;

mod config;
mod matching;
mod models;
mod utils;

#[tokio::main]
async fn main() {
    let config = read_config().await;

    let mut snapshot_store = RedisSnapshotStore::new_redis_snapshot_store(
        &config.product.id,
        &config.redis.ip,
        config.redis.port,
    )
        .await
        .unwrap_or_else(|e| panic!("{}", e));

    let mut order_reader = KafkaOrderReader::new_kafka_order_consumer(
        &config.kafka.brokers,
        &format!("order-reader-{}-group", config.product.id),
        &config.product.id,
        config.kafka.session_timeout,
    )
        .unwrap_or_else(|e| panic!("{}", e));

    let mut log_store = KafkaLogStore::new_kafka_log_producer(
        &config.kafka.brokers,
        &config.product.id,
        config.kafka.message_timeout,
    )
        .unwrap_or_else(|e| panic!("{}", e));

    let mut engine = Engine::new(&config.product, &mut snapshot_store).await;

    engine
        .start(&mut snapshot_store, &mut order_reader, &mut log_store)
        .await;
}
