use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::log::Log;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use tokio::{join, select};

use crate::matching::order_book::{OrderBook, OrderBookSnapshot};
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::{Order, Product};
use crate::models::types::{OrderStatus, TimeInForceType};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
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

    pub async fn start(
        &mut self,
        snapshot_store: &mut RedisSnapshotStore,
        order_reader: &mut KafkaOrderReader,
        log_store: &mut KafkaLogStore,
    ) {
        let (log_tx, log_rx) = mpsc::channel::<Box<dyn Log>>(10000);
        let (order_tx, order_rx) = mpsc::channel::<OffsetOrder>(10000);
        let (snapshot_req_tx, snapshot_req_rx) = mpsc::channel::<Snapshot>(32);
        let (snapshot_approve_req_tx, snapshot_approve_req_rx) = mpsc::channel::<Snapshot>(32);
        let (snapshot_tx, snapshot_rx) = mpsc::channel::<Snapshot>(32);

        let product_id = self.product_id.clone();
        let order_offset = self.order_offset;
        let log_seq = self.order_book.log_seq;

        let fut1 = Engine::run_fetcher(order_offset, order_reader, order_tx);
        let fut2 = Engine::run_applier(
            self,
            order_rx,
            log_tx,
            snapshot_req_rx,
            snapshot_approve_req_tx,
        );
        let fut3 = Engine::run_committer(
            log_seq,
            log_rx,
            snapshot_approve_req_rx,
            snapshot_tx,
            log_store,
        );
        let fut4 = Engine::run_snapshots(
            &product_id,
            order_offset,
            snapshot_req_tx,
            snapshot_rx,
            snapshot_store,
        );

        join!(fut1, fut2, fut3, fut4);
    }

    pub fn restore(&mut self, snapshot: &Snapshot) {
        self.order_offset = snapshot.order_offset;
        self.order_book.restore(&snapshot.order_book_snapshot);
    }

    pub async fn run_fetcher(
        order_offset: u64,
        order_reader: &mut KafkaOrderReader,
        order_tx: Sender<OffsetOrder>,
    ) {
        let mut offset = order_offset;
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
        order_rx: Receiver<OffsetOrder>,
        log_tx: Sender<Box<dyn Log>>,
        snapshot_req_rx: Receiver<Snapshot>,
        snapshot_approve_req_tx: Sender<Snapshot>,
    ) {
        let mut order_offset = 0u64;
        let mut order_rx = order_rx;
        let mut snapshot_req_rx = snapshot_req_rx;

        loop {
            select! {
                Some(offset_order) = order_rx.recv() => {
                    let mut logs= Vec::default();
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

    pub async fn run_committer(
        log_seq: u64,
        log_rx: Receiver<Box<dyn Log>>,
        snapshot_approve_req_rx: Receiver<Snapshot>,
        snapshot_tx: Sender<Snapshot>,
        log_store: &mut KafkaLogStore,
    ) {
        let mut seq = log_seq;
        let mut pending: Option<Snapshot> = None;
        let mut logs: Vec<Box<dyn Log>> = Vec::new();
        let mut snapshot_approve_req_rx = snapshot_approve_req_rx;
        let mut log_rx = log_rx;

        loop {
            select! {
                Some(log) = log_rx.recv() => {
                    if log.get_seq() <= seq {
                        println!("discard log seq={}", seq);
                        continue;
                    }
                    seq = log.get_seq();
                    logs.push(log);

                    for _ in 0..100 {
                        match log_rx.try_recv() {
                            Ok(log) => {
                                seq = log.get_seq();
                                logs.push(log);
                            }
                            Err(_e) => {
                                break;
                            }
                        }
                    }

                    match log_store.store(&logs).await {
                        Some(e) => { panic!("{}", e);}
                        None => {}
                    }
                    logs.clear();

                    match &pending {
                        Some(p) => {
                            if seq >= p.order_book_snapshot.log_seq {
                                match snapshot_tx.send(p.clone()).await{
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("{}", e);
                                        continue;
                                    }
                                };
                                pending = None;
                            }
                        },
                        None => {}
                    }
                },
                Some(snapshot) = snapshot_approve_req_rx.recv() => {
                    if seq >= snapshot.order_book_snapshot.log_seq {
                        match snapshot_tx.send(snapshot.clone()).await{
                            Ok(_) => {},
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };
                        pending = None;
                        continue;
                    }

                    match &pending {
                        Some(p) => {
                            println!("discard snapshot request (seq={}), new one (seq={}) received",
                            p.order_book_snapshot.log_seq, snapshot.order_book_snapshot.log_seq);
                        },
                        None => {}
                    }
                    pending = Some(snapshot);
                }
            }
        }
    }

    pub async fn run_snapshots(
        product_id: &str,
        order_offset: u64,
        snapshot_req_tx: Sender<Snapshot>,
        snapshot_rx: Receiver<Snapshot>,
        snapshot_store: &mut RedisSnapshotStore,
    ) {
        let mut order_offset = order_offset;
        let mut snapshot_rx = snapshot_rx;
        loop {
            select! {
                _ = sleep(Duration::from_secs(30)) => {
                    match snapshot_req_tx.send(Snapshot{
                        order_book_snapshot:OrderBookSnapshot::default(),
                        order_offset: order_offset,
                    }).await{
                        Ok(_) => {},
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    };
                },
                Some(snapshot) = snapshot_rx.recv() => {
                    match snapshot_store.store(&snapshot).await {
                        Some(e) => {
                            println!("store snapshot failed: {}", e);
                            continue;
                        },
                        None => {}
                    }
                    println!("new snapshot stored :product={} OrderOffset={} LogSeq={}",
                    product_id, snapshot.order_offset, snapshot.order_book_snapshot.log_seq);

                    order_offset = snapshot.order_offset;
                }
            }
        }
    }
}
