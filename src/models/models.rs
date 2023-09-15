// #[macro_use]
use serde::{Serialize, Deserialize};
use serde_json;

use chrono::prelude::*;
use rust_decimal::prelude::*;

use crate::models::types::*;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: u64,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub updated_at: DateTime<Utc>,
    pub base_currency: String,
    pub quote_currency: String,
    pub base_min_size: Decimal,
    pub base_max_size: Decimal,
    pub quote_min_size: Decimal,
    pub quote_max_size: Decimal,
    pub base_scale: i32,
    pub quote_scale: i32,
    pub quote_increment: Decimal,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: u64,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub updated_at: DateTime<Utc>,
    pub product_id: String,
    pub user_id: u64,
    pub client_oid: String,
    pub size: Decimal,
    pub funds: Decimal,
    pub filled_size: Decimal,
    pub executed_value: Decimal,
    pub price: Decimal,
    pub fill_fees: Decimal,
    pub r#type: OrderType,
    pub side: Side,
    pub time_in_force: TimeInForceType,
    pub taker_fee_ratio: Decimal,
    pub maker_fee_ratio: Decimal,
    pub status: OrderStatus,
    pub settled: bool,
}