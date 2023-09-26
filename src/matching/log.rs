// #[macro_use]
use erased_serde::serialize_trait_object;
use serde::{Deserialize, Serialize};

use chrono::prelude::*;
use rust_decimal::Decimal;

use crate::matching::order_book::BookOrder;
use crate::models::types::{DoneReason, Side, TimeInForceType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogType {
    LogTypeMatch,
    LogTypeOpen,
    LogTypeDone,
}

pub trait LogTrait: erased_serde::Serialize {
    fn get_seq(&self) -> u64;
}

serialize_trait_object!(LogTrait);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Base {
    pub r#type: LogType,
    pub sequence: u64,
    pub product_id: String,
    pub time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenLog {
    pub base: Base,
    pub order_id: u64,
    pub user_id: u64,
    pub remaining_size: Decimal,
    pub price: Decimal,
    pub side: Side,
    pub time_in_force: TimeInForceType,
}

impl LogTrait for OpenLog {
    fn get_seq(&self) -> u64 {
        self.base.sequence
    }
}

pub fn new_open_log(log_seq: u64, product_id: &str, taker_order: &BookOrder) -> OpenLog {
    println!(
        "new_open_log: product_id: {}\nlog_seq:{}\norder:{:?}",
        product_id, log_seq, taker_order
    );
    OpenLog {
        base: Base {
            r#type: LogType::LogTypeOpen,
            sequence: log_seq,
            product_id: product_id.to_string(),
            time: Utc::now().timestamp_nanos() as u64,
        },
        order_id: taker_order.order_id,
        user_id: taker_order.user_id,
        remaining_size: taker_order.size,
        price: taker_order.price,
        side: taker_order.side.clone(),
        time_in_force: taker_order.time_in_force.clone(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DoneLog {
    pub base: Base,
    pub order_id: u64,
    pub user_id: u64,
    pub price: Decimal,
    pub remaining_size: Decimal,
    pub reason: DoneReason,
    pub side: Side,
    pub time_in_force: TimeInForceType,
}

impl LogTrait for DoneLog {
    fn get_seq(&self) -> u64 {
        self.base.sequence
    }
}

pub fn new_done_log(
    log_seq: u64,
    product_id: &str,
    order: &BookOrder,
    remaining_size: &Decimal,
    reason: &DoneReason,
) -> DoneLog {
    println!(
        "new_done_log: product_id: {}\nlog_seq:{}\norder_id:{}\nreason:{:?}",
        product_id,
        log_seq,
        order.order_id.clone(),
        reason.clone()
    );
    DoneLog {
        base: Base {
            r#type: LogType::LogTypeDone,
            sequence: log_seq,
            product_id: product_id.to_string(),
            time: Utc::now().timestamp_nanos() as u64,
        },
        order_id: order.order_id,
        user_id: order.user_id,
        price: order.price,
        remaining_size: remaining_size.clone(),
        reason: reason.clone(),
        side: order.side.clone(),
        time_in_force: order.time_in_force.clone(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchLog {
    pub base: Base,
    pub trade_seq: u64,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub taker_user_id: u64,
    pub maker_user_id: u64,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub taker_time_in_force: TimeInForceType,
    pub maker_time_in_force: TimeInForceType,
}

impl LogTrait for MatchLog {
    fn get_seq(&self) -> u64 {
        self.base.sequence
    }
}

pub fn new_match_log(
    log_seq: u64,
    product_id: &str,
    trade_seq: u64,
    taker_order: &BookOrder,
    maker_order: &BookOrder,
    price: &Decimal,
    size: &Decimal,
) -> MatchLog {
    println!(
        "new_match_log: product_id: {}\nlog_seq:{}\ntrade_seq:{}\ntaker_order_id:{}\nmaker_order_id:{}\nprice:{}\nsize:{}",
        product_id,
        log_seq,
        trade_seq,
        taker_order.order_id.clone(),
        maker_order.order_id.clone(),
        price.clone(),
        size.clone()
    );
    MatchLog {
        base: Base {
            r#type: LogType::LogTypeMatch,
            sequence: log_seq,
            product_id: product_id.to_string(),
            time: Utc::now().timestamp_nanos() as u64,
        },
        trade_seq,
        taker_order_id: taker_order.order_id,
        maker_order_id: maker_order.order_id,
        taker_user_id: taker_order.user_id,
        maker_user_id: maker_order.user_id,
        side: maker_order.side.clone(),
        price: price.clone(),
        size: size.clone(),
        taker_time_in_force: taker_order.time_in_force.clone(),
        maker_time_in_force: maker_order.time_in_force.clone(),
    }
}
