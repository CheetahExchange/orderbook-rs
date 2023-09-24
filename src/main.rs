#![feature(future_join)]
#![feature(let_chains)]

use crate::matching::engine::Engine;
use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::Product;

mod matching;
mod models;
mod utils;

#[tokio::main]
async fn main() {
    let product = Product {
        id: "BTC-USD".to_string(),
        base_currency: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        base_scale: 6,
        quote_scale: 2,
    };

    let mut snapshot_store =
        RedisSnapshotStore::new_redis_snapshot_store(&product.id, "127.0.0.1", 6379)
            .await
            .unwrap_or_else(|e| panic!("{}", e));

    let mut order_reader =
        KafkaOrderReader::new_kafka_order_consumer(&["127.0.0.1:9092"], &product.id, 30)
            .unwrap_or_else(|e| panic!("{}", e));

    let mut log_store = KafkaLogStore::new_kafka_log_producer(&["127.0.0.1:9092"], &product.id, 30)
        .unwrap_or_else(|e| panic!("{}", e));

    let mut engine = Engine::new(&product, &mut snapshot_store).await;

    engine
        .start(&mut snapshot_store, &mut order_reader, &mut log_store)
        .await;
}
