use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::Sub;

use crate::matching::order_book::BookOrder;
use crate::matching::ordering::{OrderingTrait, PriceOrderIdKeyAsc, PriceOrderIdKeyDesc};
use crate::utils::error::CustomError;

pub struct Depth<T: OrderingTrait + Ord> {
    pub orders: HashMap<u64, BookOrder>,
    pub queue: BTreeMap<T, u64>,
}

impl<T: OrderingTrait + Ord> Depth<T> {
    pub fn add(&mut self, order: &BookOrder) {
        self.orders.insert(order.order_id, order.clone());
        self.queue
            .insert(T::new(&order.price, order.order_id), order.order_id);
    }

    pub fn decr_size(&mut self, order_id: u64, size: &Decimal) -> Result<(), CustomError> {
        return match self.orders.get(&order_id) {
            Some(order) => {
                let mut order = order.clone();
                match Decimal::cmp(&order.size, size) {
                    // order found in order book is not enough size (maybe some fatal issue)
                    Ordering::Less => Err(CustomError::from_string(format!(
                        "order {} size {} less than {}",
                        order_id, order.size, size
                    ))),
                    _ => {
                        order.size = order.size.sub(size);
                        if order.size.is_zero() {
                            self.orders.remove(&order_id);
                            self.queue.remove(&T::new(&order.price, order.order_id));
                        }
                        Ok(())
                    }
                }
            }
            // order not found in order book (maybe some fatal issue)
            None => Err(CustomError::from_string(format!(
                "order {} not found on book",
                order_id
            ))),
        };
    }
}

// AskDepth is order by key PriceOrderIdKeyAsc
// order by price ASC first, and then order id ASC
pub type AskDepth = Depth<PriceOrderIdKeyAsc>;
// BidDepth is order by key PriceOrderIdKeyDesc
// order by price DESC first, and then order id ASC
pub type BidDepth = Depth<PriceOrderIdKeyDesc>;
