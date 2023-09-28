// #[macro_use]
use chrono::prelude::*;
use erased_serde::serialize_trait_object;
use log::debug;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::matching::order_book::BookOrder;
use crate::models::types::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogType {
    LogTypeMatch,
    LogTypeOpen,
    LogTypeDone,
}

pub fn serialize_log_type<S>(log_type: &LogType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match log_type {
        LogType::LogTypeMatch => "match",
        LogType::LogTypeOpen => "open",
        LogType::LogTypeDone => "done",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_log_type<'de, D>(deserializer: D) -> Result<LogType, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "match" => Ok(LogType::LogTypeMatch),
        "open" => Ok(LogType::LogTypeOpen),
        "done" => Ok(LogType::LogTypeDone),
        _ => Err(serde::de::Error::custom("invalid log_type string")),
    }
}

pub trait LogTrait: erased_serde::Serialize {
    fn get_seq(&self) -> u64;
}

serialize_trait_object!(LogTrait);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Base {
    #[serde(serialize_with = "serialize_log_type")]
    #[serde(deserialize_with = "deserialize_log_type")]
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
    #[serde(serialize_with = "serialize_side")]
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
    pub time_in_force: TimeInForceType,
}

impl LogTrait for OpenLog {
    fn get_seq(&self) -> u64 {
        self.base.sequence
    }
}

pub fn new_open_log(log_seq: u64, product_id: &str, taker_order: &BookOrder) -> OpenLog {
    debug!(
        "new_open_log: product_id: {} | log_seq:{} | order:{}",
        product_id,
        log_seq,
        serde_json::to_string(&taker_order).unwrap()
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
    #[serde(serialize_with = "serialize_done_reason")]
    #[serde(deserialize_with = "deserialize_done_reason")]
    pub reason: DoneReason,
    #[serde(serialize_with = "serialize_side")]
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
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
    debug!(
        "new_done_log: product_id: {} | log_seq:{} | order_id:{} | reason:{:?}",
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
    #[serde(serialize_with = "serialize_side")]
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
    pub taker_time_in_force: TimeInForceType,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
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
    debug!(
        "new_match_log: product_id: {} | log_seq:{} | trade_seq:{} | taker_order_id:{} | maker_order_id:{} | price:{} | size:{}",
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
