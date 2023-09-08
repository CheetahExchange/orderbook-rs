// #[macro_use]
use serde::{Serialize, Deserialize};
use serde_json;

use chrono::prelude::*;
use rust_decimal::prelude::*;

pub type OrderType = String;
pub type Side = String;
pub type TimeInForceType = String;
pub type OrderStatus = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    id: i64,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    updated_at: DateTime<Utc>,
    product_id: String,
    user_id: i64,
    client_oid: String,
    size: Decimal,
    funds: Decimal,
    filled_size: Decimal,
    executed_value: Decimal,
    price: Decimal,
    fill_fees: Decimal,
    r#type: OrderType,
    side: Side,
    time_in_force: TimeInForceType,
    taker_fee_ratio: Decimal,
    maker_fee_ratio: Decimal,
    status: OrderStatus,
    settled: bool,
}