use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};

use crate::matching::depth::{AskDepth, BidDepth};
use crate::matching::log::{new_done_log, new_match_log, new_open_log, LogTrait};
use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc};
use crate::models::models::{Order, Product};
use crate::models::types::{OrderType, Side, TimeInForceType};
use crate::models::types::{DONE_REASON_CANCELLED, DONE_REASON_FILLED};
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

impl Default for BookOrder {
    fn default() -> Self {
        BookOrder {
            order_id: 0,
            user_id: 0,
            size: Default::default(),
            funds: Default::default(),
            price: Default::default(),
            side: Side::SideBuy,
            r#type: OrderType::OrderTypeLimit,
            time_in_force: TimeInForceType::GoodTillCanceled,
        }
    }
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

        // If it's a Market-Buy order, set price to infinite high, and if it's market-sell,
        // set price to zero, which ensures that prices will cross.
        if let OrderType::OrderTypeMarket = taker_order.r#type {
            taker_order.price = match taker_order.side {
                Side::SideBuy => Decimal::MAX,
                Side::SideSell => Decimal::ZERO,
            }
        }

        match taker_order.side {
            // Need to check sell-one price
            Side::SideBuy => match self.ask_depths.queue.first_key_value() {
                None => return true,
                Some((_, v)) => {
                    let maker_order = self.ask_depths.orders.get(v).unwrap();
                    // if taker's buy price is less than sell-one price
                    if taker_order.price.lt(&maker_order.price) {
                        return true;
                    }
                }
            },
            // Need to check buy-one price
            Side::SideSell => match self.bid_depths.queue.first_key_value() {
                None => return true,
                Some((_k, v)) => {
                    let maker_order = self.bid_depths.orders.get(v).unwrap();
                    // if taker's sell price is greater than buy-one price
                    if taker_order.price.gt(&maker_order.price) {
                        return true;
                    }
                }
            },
        };

        false
    }

    pub fn is_order_will_full_match(&self, order: &Order) -> bool {
        let mut taker_order = BookOrder::new_book_order(order);

        // If it's a Market-Buy order, set price to infinite high, and if it's market-sell,
        // set price to zero, which ensures that prices will cross.
        if let OrderType::OrderTypeMarket = taker_order.r#type {
            taker_order.price = match taker_order.side {
                Side::SideBuy => Decimal::MAX,
                Side::SideSell => Decimal::ZERO,
            }
        }

        match taker_order.side {
            Side::SideBuy => {
                for (_, v) in &self.ask_depths.queue {
                    let maker_order = self.ask_depths.orders.get(v).unwrap();

                    // check whether there is price crossing between the taker and the maker
                    if Ordering::Less == Decimal::cmp(&taker_order.price, &maker_order.price) {
                        break;
                    }

                    match taker_order.r#type {
                        OrderType::OrderTypeLimit => {
                            if taker_order.size.is_zero() {
                                break;
                            }

                            // Take the minimum size of taker and maker as trade size
                            let size = Decimal::min(taker_order.size, maker_order.size);

                            // adjust the size of taker order
                            taker_order.size = taker_order.size.sub(size);
                        }
                        OrderType::OrderTypeMarket => {
                            if taker_order.funds.is_zero() {
                                break;
                            }

                            // calculate the size of taker at current price
                            let taker_size = taker_order
                                .funds
                                .div(maker_order.price)
                                .trunc_with_scale(self.product.base_scale as u32);
                            if taker_size.is_zero() {
                                break;
                            }

                            // Take the minimum size of taker and maker as trade size
                            let size = Decimal::min(taker_size, maker_order.size);
                            let funds = size.mul(maker_order.price);

                            // adjust the funds of taker order
                            taker_order.funds = taker_order.funds.sub(funds);
                        }
                    }
                }
            }
            Side::SideSell => {
                for (_, v) in &self.bid_depths.queue {
                    let maker_order = self.bid_depths.orders.get(v).unwrap();

                    // check whether there is price crossing between the taker and the maker
                    if Ordering::Greater == Decimal::cmp(&taker_order.price, &maker_order.price) {
                        break;
                    }

                    if taker_order.size.is_zero() {
                        break;
                    }

                    // Take the minimum size of taker and maker as trade size
                    let size = Decimal::min(taker_order.size, maker_order.size);

                    // adjust the size of taker order
                    taker_order.size = taker_order.size.sub(size);
                }
            }
        }

        if let OrderType::OrderTypeLimit = taker_order.r#type {
            if Ordering::Greater == Decimal::cmp(&taker_order.size, &Decimal::zero()) {
                return false;
            }
        }

        true
    }

    pub fn apply_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();

        // prevent orders from being submitted repeatedly to the matching engine
        if let Err(_) = self.order_id_window.put(order.id) {
            return logs;
        }

        let mut taker_order = BookOrder::new_book_order(order);

        // If it's a Market-Buy order, set price to infinite high, and if it's market-sell,
        // set price to zero, which ensures that prices will cross.
        if let OrderType::OrderTypeMarket = taker_order.r#type {
            taker_order.price = match taker_order.side {
                Side::SideBuy => Decimal::MAX,
                Side::SideSell => Decimal::ZERO,
            }
        }

        match taker_order.side {
            Side::SideBuy => {
                for (_, v) in &(self.ask_depths.queue.clone()) {
                    let maker_order = self.ask_depths.orders.get(v).unwrap().clone();

                    let mut size = Decimal::default();

                    // check whether there is price crossing between the taker and the maker
                    if Ordering::Less == Decimal::cmp(&taker_order.price, &maker_order.price) {
                        break;
                    }

                    match taker_order.r#type {
                        OrderType::OrderTypeLimit => {
                            if taker_order.size.is_zero() {
                                break;
                            }

                            // Take the minimum size of taker and maker as trade size
                            size = Decimal::min(taker_order.size, maker_order.size);

                            // adjust the size of taker order
                            taker_order.size = taker_order.size.sub(size);
                        }
                        OrderType::OrderTypeMarket => {
                            if taker_order.funds.is_zero() {
                                break;
                            }

                            // calculate the size of taker at current price
                            let taker_size = taker_order
                                .funds
                                .div(maker_order.price)
                                .trunc_with_scale(self.product.base_scale as u32);

                            if taker_size.is_zero() {
                                break;
                            }

                            // Take the minimum size of taker and maker as trade size
                            size = Decimal::min(taker_size, maker_order.size);
                            let funds = size.mul(maker_order.price);

                            // adjust the funds of taker order
                            taker_order.funds = taker_order.funds.sub(funds);
                        }
                    }

                    // adjust the size of maker order
                    if let Err(e) = self.ask_depths.decr_size(maker_order.order_id, &size) {
                        panic!("{}", e);
                    }

                    // matched,write a log
                    let (log_seq, trade_seq) = (self.next_log_seq(), self.next_trade_seq());
                    logs.push(Box::new(new_match_log(
                        log_seq,
                        &self.product.id,
                        trade_seq,
                        &taker_order,
                        &maker_order,
                        &maker_order.price,
                        &size,
                    )));

                    // maker is filled
                    if maker_order.size.is_zero() {
                        logs.push(Box::new(new_done_log(
                            self.next_log_seq(),
                            &self.product.id,
                            &maker_order,
                            &maker_order.size,
                            &DONE_REASON_FILLED,
                        )));
                    }
                }
            }
            Side::SideSell => {
                for (_, v) in &(self.bid_depths.queue.clone()) {
                    let maker_order = self.bid_depths.orders.get(v).unwrap().clone();

                    // check whether there is price crossing between the taker and the maker
                    if Ordering::Greater == Decimal::cmp(&taker_order.price, &maker_order.price) {
                        break;
                    }

                    if taker_order.size.is_zero() {
                        break;
                    }

                    // Take the minimum size of taker and maker as trade size
                    let size = Decimal::min(taker_order.size, maker_order.size);

                    // adjust the size of taker order
                    taker_order.size = taker_order.size.sub(size);

                    // adjust the size of maker order
                    if let Err(e) = self.bid_depths.decr_size(maker_order.order_id, &size) {
                        panic!("{}", e);
                    }

                    // matched,write a log
                    let (log_seq, trade_seq) = (self.next_log_seq(), self.next_trade_seq());
                    logs.push(Box::new(new_match_log(
                        log_seq,
                        &self.product.id,
                        trade_seq,
                        &taker_order,
                        &maker_order,
                        &maker_order.price,
                        &size,
                    )));

                    // maker is filled
                    if maker_order.size.is_zero() {
                        logs.push(Box::new(new_done_log(
                            self.next_log_seq(),
                            &self.product.id,
                            &maker_order,
                            &maker_order.size,
                            &DONE_REASON_FILLED,
                        )));
                    }
                }
            }
        }

        if let OrderType::OrderTypeLimit = taker_order.r#type
            && Ordering::Greater == Decimal::cmp(&taker_order.size, &Decimal::zero()) {
            // If taker has an uncompleted size, put taker in orderBook
            match taker_order.side {
                Side::SideBuy => {
                    self.bid_depths.add(&taker_order);
                }
                Side::SideSell => {
                    self.ask_depths.add(&taker_order);
                }
            }
            logs.push(Box::new(new_open_log(self.next_log_seq(), &self.product.id, &taker_order)));
        } else {
            let mut remaining_size = taker_order.size;
            let mut reason = DONE_REASON_FILLED;

            if let OrderType::OrderTypeMarket = taker_order.r#type {
                taker_order.price = Decimal::zero();
                remaining_size = Decimal::zero();

                if let Side::SideSell = taker_order.side && Ordering::Greater == Decimal::cmp(&taker_order.size, &Decimal::zero()) {
                    reason = DONE_REASON_CANCELLED;
                } else if let Side::SideBuy = taker_order.side && Ordering::Greater == Decimal::cmp(&taker_order.funds, &Decimal::zero()) {
                    reason = DONE_REASON_CANCELLED;
                }
            }

            logs.push(Box::new(new_done_log(
                self.next_log_seq(),
                &self.product.id,
                &taker_order,
                &remaining_size,
                &reason,
            )));
        }

        logs
    }

    pub fn cancel_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();
        let mut f = false;
        let mut book_order = BookOrder::default();

        let _ = self.order_id_window.put(order.id);

        match order.side {
            Side::SideBuy => {
                if let Some(r) = self.ask_depths.orders.get(&order.id) {
                    let o = r.clone();
                    match self.ask_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
                            f = true;
                            book_order = o;
                        }
                    }
                }
            }
            Side::SideSell => {
                if let Some(r) = self.bid_depths.orders.get(&order.id) {
                    let o = r.clone();
                    match self.bid_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
                            f = true;
                            book_order = o;
                        }
                    }
                }
            }
        };

        if f {
            logs.push(Box::new(new_done_log(
                self.next_log_seq(),
                &self.product.id,
                &book_order,
                &book_order.size,
                &DONE_REASON_CANCELLED,
            )));
        }

        logs
    }

    pub fn nullify_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();

        let _ = self.order_id_window.put(order.id);

        let book_order = BookOrder::new_book_order(order);
        logs.push(Box::new(new_done_log(
            self.next_log_seq(),
            &self.product.id,
            &book_order,
            &order.size,
            &DONE_REASON_CANCELLED,
        )));

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
