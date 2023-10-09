use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::types::*;

// use serde::{Deserializer, Serializer};

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
    #[serde(serialize_with = "serialize_order_type")]
    #[serde(deserialize_with = "deserialize_order_type")]
    pub r#type: OrderType,
    #[serde(serialize_with = "serialize_side")]
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
    pub time_in_force: TimeInForceType,
    #[serde(serialize_with = "serialize_order_status")]
    #[serde(deserialize_with = "deserialize_order_status")]
    pub status: OrderStatus,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use crate::models::models::Order;
    use crate::models::types::{OrderStatus, OrderType, Side, TimeInForceType};

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

    #[test]
    fn test_deserialize_order() {
        let string = r#"{"id":1,"created_at":1695783003020967000,"product_id":"BTC-USD","user_id":1,"client_oid":"","price":"1000.00","size":"3.00","funds":"0","type":"limit","side":"buy","time_in_force":"GTC","status":"new"}"#;
        let o: Order = serde_json::from_str(string).unwrap();

        println!("{:?}", o);
    }
}
