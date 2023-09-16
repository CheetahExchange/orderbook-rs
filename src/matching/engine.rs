// #[macro_use]
use serde::{Deserialize, Serialize};
use serde_json;

use crate::matching::order_book::OrderBookSnapshot;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub order_book_snapshot: OrderBookSnapshot,
    pub order_offset: u64,
}
