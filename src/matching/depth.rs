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
        match self.orders.get(&order_id) {
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
                        } else {
                            // Partial fill: update the order in HashMap with reduced size
                            self.orders.insert(order_id, order);
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
        }
    }
}

// AskDepth is order by key PriceOrderIdKeyAsc
// order by price ASC first, and then order id ASC
pub type AskDepth = Depth<PriceOrderIdKeyAsc>;
// BidDepth is order by key PriceOrderIdKeyDesc
// order by price DESC first, and then order id ASC
pub type BidDepth = Depth<PriceOrderIdKeyDesc>;

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    use crate::matching::depth::AskDepth;
    use crate::matching::order_book::BookOrder;
    use crate::models::types::{OrderType, Side, TimeInForceType};

    fn make_book_order(order_id: u64, size: &str, price: &str) -> BookOrder {
        BookOrder {
            order_id,
            user_id: 1,
            size: Decimal::from_str(size).unwrap(),
            funds: Decimal::ZERO,
            price: Decimal::from_str(price).unwrap(),
            side: Side::SideSell,
            r#type: OrderType::OrderTypeLimit,
            time_in_force: TimeInForceType::GoodTillCanceled,
        }
    }

    #[test]
    fn test_partial_fill_updates_hashmap() {
        let mut depth = AskDepth {
            orders: Default::default(),
            queue: Default::default(),
        };

        // Add maker order: size=10, price=100
        let order = make_book_order(1, "10", "100");
        depth.add(&order);

        // Partial fill: reduce by 3
        let reduce_size = Decimal::from_str("3").unwrap();
        depth.decr_size(1, &reduce_size).unwrap();

        // Verify HashMap has updated size (7), not original (10)
        let updated = depth.orders.get(&1).unwrap();
        assert_eq!(updated.size, Decimal::from_str("7").unwrap());

        // Order should still be in queue
        assert!(depth.queue.iter().any(|(_, &id)| id == 1));
    }

    #[test]
    fn test_full_fill_removes_from_hashmap() {
        let mut depth = AskDepth {
            orders: Default::default(),
            queue: Default::default(),
        };

        let order = make_book_order(1, "10", "100");
        depth.add(&order);

        // Full fill
        let reduce_size = Decimal::from_str("10").unwrap();
        depth.decr_size(1, &reduce_size).unwrap();

        // Order should be removed from both HashMap and queue
        assert!(depth.orders.get(&1).is_none());
        assert!(!depth.queue.iter().any(|(_, &id)| id == 1));
    }

    #[test]
    fn test_sequential_partial_fills() {
        let mut depth = AskDepth {
            orders: Default::default(),
            queue: Default::default(),
        };

        let order = make_book_order(1, "10", "100");
        depth.add(&order);

        // First partial fill: 10 -> 7
        depth.decr_size(1, &Decimal::from_str("3").unwrap()).unwrap();
        assert_eq!(depth.orders.get(&1).unwrap().size, Decimal::from_str("7").unwrap());

        // Second partial fill: 7 -> 2
        depth.decr_size(1, &Decimal::from_str("5").unwrap()).unwrap();
        assert_eq!(depth.orders.get(&1).unwrap().size, Decimal::from_str("2").unwrap());

        // Final fill: 2 -> 0 (removed)
        depth.decr_size(1, &Decimal::from_str("2").unwrap()).unwrap();
        assert!(depth.orders.get(&1).is_none());
    }

    #[test]
    fn test_decr_size_exceeds_available() {
        let mut depth = AskDepth {
            orders: Default::default(),
            queue: Default::default(),
        };

        let order = make_book_order(1, "5", "100");
        depth.add(&order);

        // Try to reduce by more than available
        let result = depth.decr_size(1, &Decimal::from_str("6").unwrap());
        assert!(result.is_err());
    }
}
