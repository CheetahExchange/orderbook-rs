// #[macro_use]
use serde::{Serialize, Deserialize};
use serde_json;

use chrono::prelude::*;
use rust_decimal::prelude::*;

use crate::models::types::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: i64,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    pub updated_at: DateTime<Utc>,
    pub product_id: String,
    pub user_id: i64,
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