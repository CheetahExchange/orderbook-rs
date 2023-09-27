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

#[cfg(test)]
mod tests {
    use crate::models::models::Order;
    use crate::models::types::{OrderStatus, OrderType, Side, TimeInForceType};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_serialize_order() {
        let order = Order {
            id: 1,
            created_at: 1695783003020967000,
            product_id: "BTC-USD".to_string(),
            user_id: 1,
            client_oid: "".to_string(),
            price: Decimal::from_str(&*"1000.00".to_string()).unwrap(),
            size: Decimal::from_str(&*"3.00".to_string()).unwrap(),
            funds: Default::default(),
            r#type: OrderType::OrderTypeLimit,
            side: Side::SideBuy,
            time_in_force: TimeInForceType::GoodTillCanceled,
            status: OrderStatus::OrderStatusNew,
        };

        let s = serde_json::to_string(&order).unwrap();
        println!("{}", s);
    }
}
