use log::{debug, info, error};
use rdkafka::Offset;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{sleep, Duration};
use tokio::{join, select};

use crate::matching::kafka_log::KafkaLogStore;
use crate::matching::kafka_order::KafkaOrderReader;
use crate::matching::log::LogTrait;
use crate::matching::order_book::{OrderBook, OrderBookSnapshot};
use crate::matching::ordering::OrderingTrait;
use crate::matching::redis_snapshot::RedisSnapshotStore;
use crate::models::models::{Order, Product};
use crate::models::types::{OrderStatus, TimeInForceType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub order_book_snapshot: Option<OrderBookSnapshot>,
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
        let (log_tx, log_rx) = mpsc::channel::<Box<dyn LogTrait>>(10000);
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
        self.order_book
            .restore(&snapshot.order_book_snapshot.clone().unwrap());
    }

    pub async fn run_fetcher(
        order_offset: u64,
        order_reader: &mut KafkaOrderReader,
        order_tx: Sender<OffsetOrder>,
    ) {
        let offset = if order_offset == 0 {
            Offset::Beginning
        } else {
            Offset::Offset(order_offset as i64 + 1)
        };

        if let Err(e) = order_reader.set_offset(offset) {
            panic!("set order reader offset error: {}", e);
        }

        loop {
            match order_reader.fetch_order().await {
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
                Ok((offset, order)) => {
                    if let Some(o) = order {
                        debug!("consume order: {:?}", o.clone());
                        if let Err(e) = order_tx
                            .send(OffsetOrder {
                                offset: offset as u64,
                                order: o,
                            })
                            .await
                        {
                            error!("{}", e);
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
        log_tx: Sender<Box<dyn LogTrait>>,
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
                        if let Err(e) = log_tx.send(log).await{
                            error!("{}", e);
                            continue;
                        }
                    }

                    order_offset = offset_order.offset;
                },
                Some(mut snapshot) = snapshot_req_rx.recv() => {
                    let delta: i64 = order_offset as i64 - snapshot.order_offset as i64;
                    if delta <= 1000 {
                        continue;
                    }

                    info!("should take snapshot: {} {}-[{}]-{}->",
                        self.product_id, snapshot.order_offset, delta, order_offset);

                    snapshot.order_book_snapshot = Some(self.order_book.snapshot());
                    snapshot.order_offset = order_offset;

                    if let Err(e) = snapshot_approve_req_tx.send(snapshot).await {
                        error!("{}", e);
                        continue;
                    }
                }
            }
        }
    }

    pub async fn run_committer(
        log_seq: u64,
        log_rx: Receiver<Box<dyn LogTrait>>,
        snapshot_approve_req_rx: Receiver<Snapshot>,
        snapshot_tx: Sender<Snapshot>,
        log_store: &mut KafkaLogStore,
    ) {
        let mut seq = log_seq;
        let mut pending: Option<Snapshot> = None;
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();

        let mut snapshot_approve_req_rx = snapshot_approve_req_rx;
        let mut log_rx = log_rx;

        loop {
            select! {
                Some(log) = log_rx.recv() => {
                    // discard duplicate log
                    if log.get_seq() <= seq {
                        info!("discard log seq={}", seq);
                        continue;
                    }

                    seq = log.get_seq();
                    logs.push(log);

                    // channel is not empty and buffer is not full, continue read.
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

                    // store log, clean buffer
                    if let Err(e) = log_store.store(&logs).await {
                        panic!("{}", e);
                    }
                    logs.clear();

                    // approve pending snapshot
                    if let Some(p) = &pending {
                        if seq >= p.order_book_snapshot.clone().unwrap().log_seq {
                            if let Err(e) = snapshot_tx.send(p.clone()).await{
                                error!("{}", e);
                                continue;
                            };
                            pending = None;
                        }
                    }
                },
                Some(snapshot) = snapshot_approve_req_rx.recv() => {
                    if seq >= snapshot.order_book_snapshot.clone().unwrap().log_seq {
                        if let Err(e) = snapshot_tx.send(snapshot.clone()).await{
                            error!("{}", e);
                            continue;
                        };
                        pending = None;
                        continue;
                    }

                    if let Some(p) = &pending {
                        info!("discard snapshot request (seq={}), new one (seq={}) received", p.order_book_snapshot.clone().unwrap().log_seq, snapshot.order_book_snapshot.clone().unwrap().log_seq);
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
                    // make a new snapshot request
                    if let Err(e) = snapshot_req_tx.send(Snapshot{
                        order_book_snapshot: None,
                        order_offset: order_offset,
                    }).await{
                        error!("{}", e);
                        continue;
                    };
                },
                Some(snapshot) = snapshot_rx.recv() => {
                    // store snapshot
                    if let Err(e) = snapshot_store.store(&snapshot).await {
                        error!("store snapshot failed: {}", e);
                        continue;
                    }
                    info!("new snapshot stored :product={} OrderOffset={} LogSeq={}", product_id, snapshot.order_offset, snapshot.order_book_snapshot.unwrap().log_seq);

                    // update offset for next snapshot request
                    order_offset = snapshot.order_offset;
                }
            }
        }
    }
}
