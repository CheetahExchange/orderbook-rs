// #[macro_use]
use crate::matching::kafka_order::KafkaOrderReader;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};
use std::fmt::Display;

use crate::matching::order_book::{OrderBook, OrderBookSnapshot};
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::{Order, Product};
use crate::utils::error::CustomError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub order_book_snapshot: OrderBookSnapshot,
    pub order_offset: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OffsetOrder {
    pub offset: u64,
    pub order: Order,
}

pub struct Engine {
    pub product_id: String,
    pub order_book: OrderBook,
    pub order_offset: u64,
}

impl Engine {
    pub async fn new(product: &Product, snapshot_store: &mut RedisSnapshotStore) -> Self {
        let mut engine = Engine {
            product_id: product.id.clone(),
            order_book: OrderBook::new_order_book(product),
            order_offset: 0,
        };
        match snapshot_store.get_latest().await {
            Ok(o) => match o {
                Some(snapshot) => {
                    engine.restore(&snapshot);
                }
                None => {}
            },
            Err(e) => {
                panic!("{}", e);
            }
        }

        engine
    }

    pub fn restore(&mut self, snapshot: &Snapshot) {
        self.order_offset = snapshot.order_offset;
        self.order_book.restore(&snapshot.order_book_snapshot);
    }

    pub async fn run_fetcher(
        &self,
        order_reader: &mut KafkaOrderReader,
        order_tx: &Sender<OffsetOrder>,
    ) {
        let mut offset = self.order_offset;
        if offset > 0 {
            offset += 1;
        }
        match order_reader.set_offset(offset as i64, 5) {
            Some(e) => {
                panic!("{}", e);
            }
            None => {}
        }

        loop {
            let (offset, order, err) = order_reader.fetch_order().await;
            match err {
                Some(e) => {
                    println!("{}", e);
                    continue;
                }
                None => {}
            }
            match order {
                None => {
                    continue;
                }
                Some(o) => {
                    match order_tx
                        .send(OffsetOrder {
                            offset: offset as u64,
                            order: o,
                        })
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    }
                }
            }
        }
    }
}
