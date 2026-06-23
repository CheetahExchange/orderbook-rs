use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};
use std::time::{SystemTime, UNIX_EPOCH};

use log::info;
use rust_decimal::prelude::Zero;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::matching::depth::{AskDepth, BidDepth};
use crate::matching::log::{new_done_log, new_match_log, new_open_log, LogTrait};
use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc};
use crate::models::models::{Order, Product};
use crate::models::types::*;
use crate::utils::time_window::{TimeWindow, TimeWindowSnapshot, SNOWFLAKE_EPOCH};

/// Normalize price to the specified scale using rounding (matches Go version's Round behavior)
fn normalize_price(price: Decimal, scale: u32) -> Decimal {
    price.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

/// Normalize size to the specified scale using rounding (matches Go version's Round behavior)
fn normalize_size(size: Decimal, scale: u32) -> Decimal {
    size.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

/// Get current time in milliseconds relative to Snowflake epoch.
/// This is used for time-based deduplication window.
/// Returns: (Unix timestamp ms) - SNOWFLAKE_EPOCH
fn current_time_since_snowflake_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64 - SNOWFLAKE_EPOCH)
        .unwrap_or(0)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookOrder {
    pub order_id: u64,
    pub user_id: u64,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    #[serde(serialize_with = "serialize_side")]
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    #[serde(serialize_with = "serialize_order_type")]
    #[serde(deserialize_with = "deserialize_order_type")]
    pub r#type: OrderType,
    #[serde(serialize_with = "serialize_time_in_force_type")]
    #[serde(deserialize_with = "deserialize_time_in_force_type")]
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
    #[serde(default)]
    pub time_window: TimeWindowSnapshot,
}

pub struct OrderBook {
    pub product: Product,
    pub ask_depths: AskDepth,
    pub bid_depths: BidDepth,
    pub trade_seq: u64,
    pub log_seq: u64,
    time_window: TimeWindow,
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
            time_window: TimeWindow::new(),
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
                for v in self.ask_depths.queue.values() {
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
                for v in self.bid_depths.queue.values() {
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

        if let OrderType::OrderTypeLimit = taker_order.r#type
            && Ordering::Greater == Decimal::cmp(&taker_order.size, &Decimal::zero()) {
                return false;
            }

        true
    }

    pub fn apply_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();

        // Prevent orders from being submitted repeatedly to the matching engine
        // Get current time in milliseconds since snowflake epoch
        let now_time = current_time_since_snowflake_epoch();

        if let Err(e) = self.time_window.put(order.id, now_time) {
            // Check if this is an "expired" order (window has moved past this order's time)
            // This can happen when Kafka replays old messages after a restart or rebalance.
            //
            // If the order is not in the order book, it was never processed or already completed,
            // so we should process it anyway (it will generate an empty result if already completed).
            let found_in_buy = self.bid_depths.orders.contains_key(&order.id);
            let found_in_sell = self.ask_depths.orders.contains_key(&order.id);

            if found_in_buy || found_in_sell {
                // Order is already in the book, this is a duplicate - skip it
                info!("expired order {} already in order book, skipping", order.id);
                return logs;
            }

            // Order not in orderBook - allow processing
            info!("expired order {} not in order book, processing anyway", order.id);
        }

        let mut taker_order = BookOrder::new_book_order(order);

        // Normalize price and size to product scales
        taker_order.price = normalize_price(taker_order.price, self.product.quote_scale as u32);
        taker_order.size = normalize_size(taker_order.size, self.product.base_scale as u32);

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
                // Collect order IDs to match first to avoid borrow issues
                let order_ids: Vec<u64> = self.ask_depths.queue.values().copied().collect();

                for order_id in order_ids {
                    let maker_order = match self.ask_depths.orders.get(&order_id) {
                        Some(o) => o.clone(),
                        None => continue,
                    };

                    // check whether there is price crossing between the taker and the maker
                    if Ordering::Less == Decimal::cmp(&taker_order.price, &maker_order.price) {
                        break;
                    }

                    let mut size = Decimal::default();

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
                    let mut maker_order = maker_order;
                    maker_order.size = maker_order.size.sub(size);

                    // matched, new match log
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

                    // check if taker is exhausted after this match
                    match taker_order.r#type {
                        OrderType::OrderTypeLimit => {
                            if taker_order.size.is_zero() {
                                break;
                            }
                        }
                        OrderType::OrderTypeMarket => {
                            if taker_order.funds.is_zero() {
                                break;
                            }
                        }
                    }
                }
            }
            Side::SideSell => {
                // Collect order IDs to match first to avoid borrow issues
                let order_ids: Vec<u64> = self.bid_depths.queue.values().copied().collect();

                for order_id in order_ids {
                    let maker_order = match self.bid_depths.orders.get(&order_id) {
                        Some(o) => o.clone(),
                        None => continue,
                    };

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
                    let mut maker_order = maker_order;
                    maker_order.size = maker_order.size.sub(size);

                    // matched, new match log
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

                    // check if taker is exhausted after this match
                    if taker_order.size.is_zero() {
                        break;
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

        // Mark order as seen in time window
        let now_time = current_time_since_snowflake_epoch();
        let _ = self.time_window.put(order.id, now_time);

        match order.side {
            Side::SideBuy => {
                if let Some(r) = self.bid_depths.orders.get(&order.id) {
                    let o = r.clone();
                    let remaining_size = o.size;
                    match self.bid_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
                            logs.push(Box::new(new_done_log(
                                self.next_log_seq(),
                                &self.product.id,
                                &o,
                                &remaining_size,
                                &DONE_REASON_CANCELLED,
                            )));
                        }
                    }
                }
            }
            Side::SideSell => {
                if let Some(r) = self.ask_depths.orders.get(&order.id) {
                    let o = r.clone();
                    let remaining_size = o.size;
                    match self.ask_depths.decr_size(order.id, &o.size) {
                        Err(e) => {
                            panic!("{}", e);
                        }
                        Ok(()) => {
                            logs.push(Box::new(new_done_log(
                                self.next_log_seq(),
                                &self.product.id,
                                &o,
                                &remaining_size,
                                &DONE_REASON_CANCELLED,
                            )));
                        }
                    }
                }
            }
        };

        logs
    }

    pub fn nullify_order(&mut self, order: &Order) -> Vec<Box<dyn LogTrait>> {
        let mut logs: Vec<Box<dyn LogTrait>> = Vec::new();

        // Mark order as seen in time window
        let now_time = current_time_since_snowflake_epoch();
        let _ = self.time_window.put(order.id, now_time);

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
            time_window: self.time_window.snapshot(),
        };
        snapshot
            .orders
            .reserve(self.ask_depths.orders.len() + self.bid_depths.orders.len());

        for o in self.ask_depths.orders.values() {
            snapshot.orders.push(o.clone());
        }
        for o in self.bid_depths.orders.values() {
            snapshot.orders.push(o.clone());
        }

        snapshot
    }

    pub fn restore(&mut self, snapshot: &OrderBookSnapshot) {
        self.log_seq = snapshot.log_seq;
        self.trade_seq = snapshot.trade_seq;

        // Restore time window
        self.time_window.restore(&snapshot.time_window);

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

    /// Cleanup expired orders from the time window.
    /// This should be called periodically to prevent memory leaks when there are no new orders.
    pub fn cleanup_time_window(&mut self) {
        let now_time = current_time_since_snowflake_epoch();
        self.time_window.cleanup(now_time);
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
