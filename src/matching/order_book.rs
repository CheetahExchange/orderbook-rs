// #[macro_use]
use serde::{Serialize, Deserialize};
use serde_json;

use rust_decimal::Decimal;

use crate::models::types::{Side, OrderType, TimeInForceType};
use crate::models::models::Order;
use crate::utils::window::Window;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookOrder {
    pub order_id: i64,
    pub user_id: i64,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    pub side: Side,
    pub r#type: OrderType,
    pub time_in_force: TimeInForceType,
}

pub fn new_book_order(order: Order) -> BookOrder {
    BookOrder {
        order_id: order.id,
        user_id: order.user_id,
        size: order.size,
        funds: order.funds,
        price: order.price,
        side: order.side,
        r#type: order.r#type,
        time_in_force: order.time_in_force,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderBookSnapshot {
    pub product_id: String,
    pub orders: Vec<BookOrder>,
    pub trade_seq: u64,
    pub log_seq: u64,
    pub order_id_window: Window,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub order_book_snapshot: OrderBookSnapshot,
    pub order_offset: u64,
}