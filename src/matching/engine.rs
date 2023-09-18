// #[macro_use]
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::log::Log;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt::Display;
use tokio::select;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::matching::order_book::{OrderBook, OrderBookSnapshot};
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::{Order, Product};
use crate::models::types::{OrderStatus, TimeInForceType};
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

    pub async fn run_applier(
        &mut self,
        order_rx: &mut Receiver<OffsetOrder>,
        log_tx: &Sender<Box<dyn Log>>,
        snapshot_req_rx: &mut Receiver<Snapshot>,
        snapshot_approve_req_tx: &Sender<Snapshot>,
    ) {
        let mut order_offset = 0u64;
        loop {
            select! {
                Some(offset_order) = order_rx.recv() => {
                    let mut logs: Vec<Box<dyn Log>> = Vec::new();
                    match offset_order.order.status {
                        OrderStatus::OrderStatusCancelling => {
                            logs = self.order_book.cancel_order(&offset_order.order);
                        }
                        _ => {
                            match offset_order.order.time_in_force {
                                TimeInForceType::ImmediateOrCancel => {
                                    logs = self.order_book.apply_order(&offset_order.order);
                                    let ioc_logs = self.order_book.cancel_order(&offset_order.order);
                                    if !ioc_logs.is_empty() {
                                        logs.extend(ioc_logs);
                                    }
                                },
                                TimeInForceType::GoodTillCrossing => {
                                    if self.order_book.is_order_will_not_match(&offset_order.order) {
                                        logs = self.order_book.apply_order(&offset_order.order);
                                    } else {
                                        logs = self.order_book.nullify_order(&offset_order.order);
                                    }
                                },
                                TimeInForceType::FillOrKill => {
                                 if self.order_book.is_order_will_full_match(&offset_order.order) {
                                        logs = self.order_book.apply_order(&offset_order.order);
                                    } else {
                                        logs = self.order_book.nullify_order(&offset_order.order);
                                    }
                                },
                                TimeInForceType::GoodTillCanceled => {
                                    logs = self.order_book.apply_order(&offset_order.order);
                                },
                                _ => {
                                    continue;
                                }
                            }
                        }
                    }

                    for log in logs {
                        match log_tx.send(log).await{
                            Ok(_) => {}
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        }
                    }

                    order_offset = offset_order.offset;
                },
                Some(mut snapshot) = snapshot_req_rx.recv() => {
                    let delta: i64 = order_offset as i64 - snapshot.order_offset as i64;
                    if delta <= 1000 {
                        continue;
                    }

                    println!("should take snapshot: {} {}-[{}]-{}->",
                        self.product_id, snapshot.order_offset, delta, order_offset);
                    snapshot.order_book_snapshot = self.order_book.snapshot();
                    snapshot.order_offset = order_offset;

                    match snapshot_approve_req_tx.send(snapshot).await{
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
