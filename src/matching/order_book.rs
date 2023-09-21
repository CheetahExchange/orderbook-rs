use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::matching::depth::{AskDepth, BidDepth};
use crate::matching::log::{new_done_log, new_match_log, new_open_log, LogTrait};
use rust_decimal::prelude::Zero;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};

use crate::models::models::{Order, Product};
use crate::models::types::{
    OrderType, Side, TimeInForceType, DONE_REASON_CANCELLED, DONE_REASON_FILLED,
};
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
    pub fn new_book_order(order: &Order) -> Self {
        BookOrder {
            order_id: order.id,
            user_id: order.user_id,
            size: order.size,
            funds: order.funds,
            price: order.price,
            side: order.side.clone(),
            r#type: order.r#type.clone(),
            time_in_force: order.time_in_force.clone(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
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
    pub fn new_order_book(product: &Product) -> Self {
        OrderBook {
            product: product.clone(),
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

    pub fn is_order_will_not_match(&self, order: &Order) -> bool {
        let mut taker_order = BookOrder::new_book_order(order);
        if taker_order.r#type == OrderType::OrderTypeMarket {
            taker_order.price = match taker_order.side {
                Side::SideBuy => Decimal::MAX,
                Side::SideSell => Decimal::ZERO,
            }
        }

        return match taker_order.side {
            // the Sell One
            Side::SideBuy => match self.ask_depths.queue.first_key_value() {
                None => true,
                Some((_k, v)) => {
                    let maker_order = self.ask_depths.orders.get(v).unwrap();
                    // if taker buy price is less than Sell One price
                    if taker_order.price.lt(&maker_order.price) {
                        true
                    } else {
                        false
                    }
                }
            },
            // the Buy One
            Side::SideSell => match self.bid_depths.queue.first_key_value() {
                None => true,
                Some((_k, v)) => {
                    let maker_order = self.bid_depths.orders.get(v).unwrap();
                    // if taker sell price is greater than Buy One price
                    if taker_order.price.gt(&maker_order.price) {
                        true
                    } else {
                        false
                    }
                }
            },
        };
    }

    pub fn is_order_will_full_match(&self, order: &Order) -> bool {
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
                for (_k, v) in &self.ask_depths.queue {
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
                for (_k, v) in &self.bid_depths.queue {
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

    pub fn apply_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();
        match self.order_id_window.put(order.id) {
            Some(_e) => {
                return logs;
            }
            _ => {}
        }

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
                for (_k, v) in &(self.ask_depths.queue.clone()) {
                    let maker_order = self.ask_depths.orders.get(v).unwrap().clone();
                    let mut size = Decimal::default();
                    match taker_order.r#type {
                        OrderType::OrderTypeLimit => {
                            if taker_order.size.is_zero() {
                                break;
                            }
                            size = Decimal::min(taker_order.size, maker_order.size);
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
                            size = Decimal::min(taker_size, maker_order.size);
                            let funds = size.mul(maker_order.price);
                            taker_order.funds = taker_order.funds.sub(funds);
                        }
                    }

                    match self.ask_depths.decr_size(maker_order.order_id, &size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {}
                    }

                    let log_seq = self.next_log_seq();
                    let trade_seq = self.next_trade_seq();
                    let match_log = new_match_log(
                        log_seq,
                        &self.product.id,
                        trade_seq,
                        &taker_order,
                        &maker_order,
                        &maker_order.price,
                        &size,
                    );
                    logs.push(Box::new(match_log));

                    if maker_order.size.is_zero() {
                        let log_seq = self.next_log_seq();
                        let done_log = new_done_log(
                            log_seq,
                            &self.product.id,
                            &maker_order,
                            &maker_order.size,
                            &DONE_REASON_FILLED,
                        );
                        logs.push(Box::new(done_log));
                    }
                }
            }
            Side::SideSell => {
                for (_k, v) in &(self.bid_depths.queue.clone()) {
                    let maker_order = self.bid_depths.orders.get(v).unwrap().clone();
                    if taker_order.size.is_zero() {
                        break;
                    }
                    let size = Decimal::min(taker_order.size, maker_order.size);
                    taker_order.size = taker_order.size.sub(size);

                    match self.bid_depths.decr_size(maker_order.order_id, &size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {}
                    }

                    let log_seq = self.next_log_seq();
                    let trade_seq = self.next_trade_seq();
                    let match_log = new_match_log(
                        log_seq,
                        &self.product.id,
                        trade_seq,
                        &taker_order,
                        &maker_order,
                        &maker_order.price,
                        &size,
                    );
                    logs.push(Box::new(match_log));

                    if maker_order.size.is_zero() {
                        let log_seq = self.next_log_seq();
                        let done_log = new_done_log(
                            log_seq,
                            &self.product.id,
                            &maker_order,
                            &maker_order.size,
                            &DONE_REASON_FILLED,
                        );
                        logs.push(Box::new(done_log));
                    }
                }
            }
        }

        let (mut f1, mut f2) = (false, false);
        match taker_order.r#type {
            OrderType::OrderTypeLimit => {
                f1 = true;
            }
            _ => {}
        }
        match Decimal::cmp(&taker_order.size, &Decimal::zero()) {
            Ordering::Greater => {
                f2 = true;
            }
            _ => {}
        }

        if f1 && f2 {
            match taker_order.side {
                Side::SideBuy => {
                    self.bid_depths.add(&taker_order);
                }
                Side::SideSell => {
                    self.ask_depths.add(&taker_order);
                }
            }

            let log_seq = self.next_log_seq();
            let open_log = new_open_log(log_seq, &self.product.id, &taker_order);
            logs.push(Box::new(open_log));
        } else {
            let mut remaining_size = taker_order.size;
            let mut reason = DONE_REASON_FILLED;

            if !f1 {
                taker_order.price = Decimal::zero();
                remaining_size = Decimal::zero();

                match taker_order.side {
                    Side::SideSell => match Decimal::cmp(&taker_order.size, &Decimal::zero()) {
                        Ordering::Greater => {
                            reason = DONE_REASON_CANCELLED;
                        }
                        _ => {}
                    },
                    Side::SideBuy => match Decimal::cmp(&taker_order.funds, &Decimal::zero()) {
                        Ordering::Greater => {
                            reason = DONE_REASON_CANCELLED;
                        }
                        _ => {}
                    },
                }
            }

            let log_seq = self.next_log_seq();
            let done_log = new_done_log(
                log_seq,
                &self.product.id,
                &taker_order,
                &remaining_size,
                &reason,
            );
            logs.push(Box::new(done_log));
        }

        logs
    }

    pub fn cancel_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();
        let _ = self.order_id_window.put(order.id);
        let mut f = false;
        let mut book_order = BookOrder::default();

        match order.side {
            Side::SideBuy => {
                let r = self.ask_depths.orders.get(&order.id);
                if r.is_some() {
                    let o = r.unwrap().clone();
                    match self.ask_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
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
                    match self.bid_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
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
                &book_order.size,
                &DONE_REASON_CANCELLED,
            );
            logs.push(Box::new(done_log));
        }
        logs
    }

    pub fn nullify_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();
        let _ = self.order_id_window.put(order.id);

        let book_order = BookOrder::new_book_order(order);
        let done_log = new_done_log(
            self.next_log_seq(),
            &self.product.id,
            &book_order,
            &order.size,
            &DONE_REASON_CANCELLED,
        );
        logs.push(Box::new(done_log));
        logs
    }

    pub fn snapshot(&self) -> OrderBookSnapshot {
        let mut snapshot = OrderBookSnapshot {
            product_id: self.product.id.clone(),
            orders: Vec::new(),
            trade_seq: self.trade_seq,
            log_seq: self.log_seq,
            order_id_window: self.order_id_window.clone(),
        };
        snapshot
            .orders
            .reserve(self.ask_depths.orders.len() + self.bid_depths.orders.len());
        for (_, o) in &self.ask_depths.orders {
            snapshot.orders.push(o.clone());
        }
        for (_, o) in &self.bid_depths.orders {
            snapshot.orders.push(o.clone());
        }
        snapshot
    }

    pub fn restore(&mut self, snapshot: &OrderBookSnapshot) {
        self.log_seq = snapshot.log_seq;
        self.trade_seq = snapshot.trade_seq;
        self.order_id_window = snapshot.order_id_window.clone();
        if self.order_id_window.cap == 0 {
            self.order_id_window = Window::new(0, ORDER_ID_WINDOW_CAP);
        }
        for o in &snapshot.orders {
            match o.side {
                Side::SideBuy => {
                    self.bid_depths.add(o);
                }
                Side::SideSell => {
                    self.ask_depths.add(o);
                }
            }
        }
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
