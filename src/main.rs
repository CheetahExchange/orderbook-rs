#![feature(future_join)]

use crate::matching::engine::Engine;
use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::Product;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

mod matching;
mod models;
mod utils;

#[tokio::main]
async fn main() {
    let product = Product {
        id: "BTC-USD".to_string(),
        created_at: Default::default(),
        updated_at: Default::default(),
        base_currency: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        base_min_size: Decimal::from_f64(0.0000100000000000).unwrap(),
        base_max_size: Decimal::from_f64(10000000.0000000000000000).unwrap(),
        quote_min_size: Default::default(),
        quote_max_size: Default::default(),
        base_scale: 6,
        quote_scale: 2,
        quote_increment: Decimal::from_f64(0.0100000000000000).unwrap(),
    };
    let mut snapshot_store =
        RedisSnapshotStore::new_redis_snapshot_store(&product.id, "127.0.0.1", 6379)
            .await
            .unwrap();
    let mut order_reader =
        KafkaOrderReader::new_kafka_order_consumer(&["127.0.0.1:9092"], &product.id, 5).unwrap();
    let mut log_store =
        KafkaLogStore::new_kafka_log_producer(&["127.0.0.1:9092"], &product.id, 5).unwrap();

    let mut engine = Engine::new(&product, &mut snapshot_store).await;

    engine
        .start(&mut snapshot_store, &mut order_reader, &mut log_store)
        .await;
}
