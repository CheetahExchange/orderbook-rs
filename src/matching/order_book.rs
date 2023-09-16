// #[macro_use]
use serde::{Deserialize, Serialize};
use serde_json;

use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc};
use rust_decimal::Decimal;

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::models::models::{Order, Product};
use crate::models::types::{OrderType, Side, TimeInForceType};
use crate::utils::window::Window;

const ORDER_ID_WINDOW_CAP: u64 = 10000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookOrder {
    pub order_id: u64,
    pub user_id: u64,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    pub side: Side,
    pub r#type: OrderType,
    pub time_in_force: TimeInForceType,
}

impl BookOrder {
    pub fn new_book_order(order: Order) -> Self {
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderBookSnapshot {
    pub product_id: String,
    pub orders: Vec<BookOrder>,
    pub trade_seq: u64,
    pub log_seq: u64,
    pub order_id_window: Window,
}

pub struct AskDepth {
    pub orders: HashMap<u64, BookOrder>,
    pub queue: BTreeMap<PriceOrderIdKeyAsc, u64>,
}

pub struct BidDepth {
    pub orders: HashMap<u64, BookOrder>,
    pub queue: BTreeMap<PriceOrderIdKeyDesc, u64>,
}

pub struct OrderBook {
    pub product: Product,
    pub ask_depths: AskDepth,
    pub bid_depths: BidDepth,
    pub trade_seq: u64,
    pub log_seq: u64,
    pub order_id_window: Window,
}

impl OrderBook {
    pub fn new_order_book(product: Product) -> Self {
        OrderBook {
            product,
            ask_depths: AskDepth {
                orders: HashMap::<u64, BookOrder>::new(),
                queue: BTreeMap::<PriceOrderIdKeyAsc, u64>::new(),
            },
            bid_depths: BidDepth {
                orders: HashMap::<u64, BookOrder>::new(),
                queue: BTreeMap::<PriceOrderIdKeyDesc, u64>::new(),
            },
            trade_seq: 0,
            log_seq: 0,
            order_id_window: Window::new(0, ORDER_ID_WINDOW_CAP),
        }
    }

    pub fn is_order_will_not_match(&self, order: Order) -> bool {
        let mut taker_order = BookOrder::new_book_order(order);
        match taker_order.r#type {
            OrderType::OrderTypeMarket => {
                taker_order.price = match taker_order.side {
                    Side::SideBuy => Decimal::MAX,
                    Side::SideSell => Decimal::ZERO,
                }
            }
            _ => {}
        }

        return match taker_order.side {
            Side::SideBuy => match self.ask_depths.queue.first_key_value() {
                None => true,
                Some((k, v)) => {
                    let maker_order = self.ask_depths.orders.get(v).unwrap();
                    if taker_order.price.lt(&maker_order.price) {
                        true
                    } else {
                        false
                    }
                }
            },
            Side::SideSell => match self.bid_depths.queue.first_key_value() {
                None => true,
                Some((k, v)) => {
                    let maker_order = self.bid_depths.orders.get(v).unwrap();
                    if taker_order.price.gt(&maker_order.price) {
                        true
                    } else {
                        false
                    }
                }
            },
        };
    }
}
