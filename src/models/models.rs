use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::types::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub base_scale: i32,
    pub quote_scale: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: u64,
    // use timestamp_nanos
    pub created_at: u64,
    pub product_id: String,
    pub user_id: u64,
    pub client_oid: String,
    pub price: Decimal,
    pub size: Decimal,
    pub funds: Decimal,
    pub r#type: OrderType,
    pub side: Side,
    pub time_in_force: TimeInForceType,
    pub status: OrderStatus,
}
