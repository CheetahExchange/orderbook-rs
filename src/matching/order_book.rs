// #[macro_use]
use serde::{Deserialize, Serialize};
use serde_json;

use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc, PriceOrderIdKeyOrdering};
use rust_decimal::Decimal;

use crate::matching::depth::{AskDepth, BidDepth};
use crate::matching::log::{new_done_log, new_match_log, DoneLog, Log};
use rust_decimal::prelude::Zero;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};

use crate::models::models::{Order, Product};
use crate::models::types::{
    OrderType, Side, TimeInForceType, DONE_REASON_CANCELLED, DONE_REASON_FILLED,
};
use crate::utils::error::CustomError;
use crate::utils::window::Window;

const ORDER_ID_WINDOW_CAP: u64 = 10000;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
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

    pub fn is_order_will_full_match(&self, order: Order) -> bool {
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

        match taker_order.side {
            Side::SideBuy => {
                for (k, v) in &self.ask_depths.queue {
                    let maker_order = self.ask_depths.orders.get(v).unwrap();
                    match taker_order.r#type {
                        OrderType::OrderTypeLimit => {
                            if taker_order.size.is_zero() {
                                break;
                            }
                            let size = Decimal::min(taker_order.size, maker_order.size);
                            taker_order.size = taker_order.size.sub(size);
                        }
                        OrderType::OrderTypeMarket => {
                            if taker_order.size.is_zero() {
                                break;
                            }
                            let taker_size = taker_order
                                .funds
                                .div(maker_order.price)
                                .trunc_with_scale(self.product.base_scale as u32);
                            let size = Decimal::min(taker_size, maker_order.size);
                            let funds = size.mul(maker_order.price);
                            taker_order.funds = taker_order.funds.sub(funds);
                        }
                    }
                }
            }
            Side::SideSell => {
                for (k, v) in &self.bid_depths.queue {
                    let maker_order = self.bid_depths.orders.get(v).unwrap();
                    if taker_order.size.is_zero() {
                        break;
                    }
                    let size = Decimal::min(taker_order.size, maker_order.size);
                    taker_order.size = taker_order.size.sub(size);
                }
            }
        }

        return match taker_order.r#type {
            OrderType::OrderTypeLimit => match Decimal::cmp(&taker_order.size, &Decimal::zero()) {
                Ordering::Greater => false,
                _ => true,
            },
            _ => true,
        };
    }

    pub fn cancel_order(&mut self, order: Order) -> Vec<DoneLog> {
        let mut logs: Vec<DoneLog> = Vec::new();
        let _ = self.order_id_window.put(order.id);
        let mut f = false;
        let mut book_order = BookOrder::default();

        match order.side {
            Side::SideBuy => {
                let r = self.ask_depths.orders.get(&order.id);
                if r.is_some() {
                    let o = r.unwrap().clone();
                    match self.ask_depths.decr_size(order.id, o.size) {
                        Some(e) => {
                            panic!("{}", e);
                        }
                        None => {
                            f = true;
                            book_order = o.clone();
                        }
                    }
                }
            }
            Side::SideSell => {
                let r = self.bid_depths.orders.get(&order.id);
                if r.is_some() {
                    let o = r.unwrap().clone();
                    match self.bid_depths.decr_size(order.id, o.size) {
                        Some(e) => {
                            panic!("{}", e);
                        }
                        None => {
                            f = true;
                            book_order = o.clone();
                        }
                    }
                }
            }
        };

        if f {
            let done_log = new_done_log(
                self.next_log_seq(),
                &self.product.id,
                &book_order,
                book_order.size,
                DONE_REASON_CANCELLED,
            );
            logs.push(done_log);
        }
        logs
    }

    pub fn next_log_seq(&mut self) -> u64 {
        self.log_seq += 1;
        self.log_seq
    }

    pub fn next_trade_seq(&mut self) -> u64 {
        self.trade_seq += 1;
        self.trade_seq
    }
}
