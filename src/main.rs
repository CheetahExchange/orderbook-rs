#![feature(future_join)]
#![feature(let_chains)]

use std::io::Write;
use std::str::FromStr;

use env_logger::Builder;
use log::LevelFilter;

use crate::config::read_config;
use crate::matching::engine::Engine;
use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::redis_snapshot::RedisSnapshotStore;

mod config;
mod matching;
mod models;
mod utils;

fn init_log(level: &str) {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}][{}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.file().unwrap(),
                record.line().unwrap(),
                record.args()
            )
        })
        .filter(None, LevelFilter::from_str(level).unwrap())
        .init();
}

#[tokio::main]
async fn main() {
    let config = read_config().await;

    init_log(&config.log.level);

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
