use crate::matching::order_book::BookOrder;
use crate::matching::ordering::{PriceOrderIdKeyAsc, PriceOrderIdKeyDesc, PriceOrderIdKeyOrdering};
use crate::utils::error::CustomError;
use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::Sub;

pub struct Depth<T: PriceOrderIdKeyOrdering + Ord> {
    pub orders: HashMap<u64, BookOrder>,
    pub queue: BTreeMap<T, u64>,
}

impl<T: PriceOrderIdKeyOrdering + Ord> Depth<T> {
    pub fn add(&mut self, order: &BookOrder) {
        self.orders.insert(order.order_id, order.clone());
        self.queue
            .insert(T::new(&order.price, order.order_id), order.order_id);
    }

    pub fn decr_size(&mut self, order_id: u64, size: Decimal) -> Option<CustomError> {
        return match self.orders.get(&order_id) {
            Some(order) => {
                let mut order = order.clone();
                match Decimal::cmp(&order.size, &size) {
                    Ordering::Less => Some(CustomError::from_string(format!(
                        "order {} Size {} less than {}",
                        order_id, order.size, size
                    ))),
                    _ => {
                        order.size = order.size.sub(size);
                        if order.size.is_zero() {
                            self.orders.remove(&order_id);
                            self.queue.remove(&T::new(&order.price, order.order_id));
                        }
                        None
                    }
                }
            }
            None => Some(CustomError::from_string(format!(
                "order {} not found on book",
                order_id
            ))),
        };
    }
}

pub type AskDepth = Depth<PriceOrderIdKeyAsc>;
pub type BidDepth = Depth<PriceOrderIdKeyDesc>;
