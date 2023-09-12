// #[macro_use]
use serde::{Serialize, Deserialize};
use serde_json;

use chrono::prelude::*;
use rust_decimal::Decimal;
use crate::models::types::{Side, TimeInForceType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogType {
    LogTypeMatch,
    LogTypeOpen,
    LogTypeDone,
}

pub trait Log {
    fn get_seq(self) -> i64;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Base {
    pub r#type: LogType,
    pub sequence: i64,
    pub product_id: String,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenLog {
    pub base: Base,
    pub order_id: i64,
    pub user_id: i64,
    pub remaining_size: Decimal,
    pub price: Decimal,
    pub side: Side,
    pub time_in_force: TimeInForceType,
}

impl Log for OpenLog {
    fn get_seq(self) -> i64 {
        self.base.sequence
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DoneLog {
    pub base: Base,
    pub order_id: i64,
    pub user_id: i64,
    pub price: Decimal,
    pub remaining_size: Decimal,
    pub reason: String,
    pub side: Side,
    pub time_in_force: TimeInForceType,
}

impl Log for DoneLog {
    fn get_seq(self) -> i64 {
        self.base.sequence
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchLog {
    pub base: Base,
    pub trade_seq: i64,
    pub taker_order_id: i64,
    pub maker_order_id: i64,
    pub taker_user_id: i64,
    pub maker_user_id: i64,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub taker_time_in_force: TimeInForceType,
    pub maker_time_in_force: TimeInForceType,
}

impl Log for MatchLog {
    fn get_seq(self) -> i64 {
        self.base.sequence
    }
}