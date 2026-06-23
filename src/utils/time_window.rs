use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::utils::error::CustomError;

/// Snowflake epoch: Nov 04 2010 01:42:54 UTC in milliseconds
/// This is the standard epoch used by bwmarrin/snowflake library
pub const SNOWFLAKE_EPOCH: i64 = 1288834974657;

/// Default duration of the deduplication window in milliseconds (30 seconds)
pub const TIME_WINDOW_DURATION: i64 = 30 * 1000;

/// Default number of sharded order tables
pub const DEFAULT_TABLE_SPLIT_COUNT: u64 = 128;

/// Extracts the timestamp (in milliseconds since Unix epoch) from a Snowflake ID.
///
/// Snowflake ID structure (64 bits):
/// | 1 bit sign | 41 bits timestamp (ms since Snowflake epoch) | 10 bits node ID | 12 bits sequence |
///
/// The timestamp is stored in bits 22-62 relative to the Snowflake epoch.
#[inline]
pub fn extract_timestamp_from_id(order_id: u64) -> i64 {
    ((order_id >> 22) as i64) + SNOWFLAKE_EPOCH
}

/// Extracts the node ID from a Snowflake ID.
/// The node ID is stored in bits 12-21 of the Snowflake ID.
#[inline]
pub fn extract_node_id_from_id(order_id: u64) -> u64 {
    (order_id >> 12) & 0x3FF // 10 bits mask
}

/// Returns the table shard index for a given order ID.
/// This uses the node ID embedded in the Snowflake ID to determine the shard.
/// The formula: (order_id >> 12) % split_count extracts the node ID and uses it for sharding.
pub fn get_table_index_by_order_id(order_id: u64, split_count: u64) -> usize {
    ((order_id >> 12) % split_count) as usize
}

/// Returns the table shard index for a given user ID.
/// The user ID is directly used for sharding to ensure consistent routing.
pub fn get_table_index_by_user_id(user_id: u64, split_count: u64) -> usize {
    (user_id % split_count) as usize
}

/// Key for storing orders in the time-based window.
/// Orders are sorted by timestamp first, then by order_id for uniqueness.
#[derive(Debug, Clone, Eq, PartialEq)]
struct TimeOrderIdKey {
    timestamp: i64, // milliseconds since Snowflake epoch
    order_id: u64,
}

impl Ord for TimeOrderIdKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare by timestamp
        match self.timestamp.cmp(&other.timestamp) {
            std::cmp::Ordering::Equal => self.order_id.cmp(&other.order_id),
            ord => ord,
        }
    }
}

impl PartialOrd for TimeOrderIdKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// TimeWindow is a time-based sliding window for order deduplication.
/// It keeps track of orders within a time duration (e.g., 30 seconds).
/// Orders older than the window are automatically removed.
///
/// Unlike ID-based windows which have limited capacity (e.g., 10000 IDs covering only ~2ms),
/// a time-based window can tolerate much larger delays (30 seconds) making it suitable
/// for Snowflake IDs where the ID range grows rapidly (4M IDs per millisecond).
#[derive(Debug)]
pub struct TimeWindow {
    /// Window time range in milliseconds since Snowflake epoch
    min_time: i64,
    max_time: i64,

    /// Duration of the window in milliseconds
    duration: i64,

    /// Orders sorted by (timestamp, order_id)
    orders: BTreeMap<TimeOrderIdKey, bool>,
}

impl Default for TimeWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeWindow {
    /// Creates a new time-based deduplication window.
    pub fn new() -> Self {
        TimeWindow {
            min_time: 0,
            max_time: 0,
            duration: TIME_WINDOW_DURATION,
            orders: BTreeMap::new(),
        }
    }

    /// Creates a time window with custom duration (in milliseconds).
    pub fn with_duration(duration: i64) -> Self {
        TimeWindow {
            min_time: 0,
            max_time: 0,
            duration,
            orders: BTreeMap::new(),
        }
    }

    /// Extracts the timestamp from a Snowflake ID.
/// Snowflake ID structure: | 1bit sign | 41bit timestamp(ms) | 10bit node | 12bit sequence |
    pub fn extract_timestamp(&self, order_id: u64) -> i64 {
        extract_timestamp_from_id(order_id)
    }

    /// Adds an order to the time window.
    /// Returns error if:
    /// - The order timestamp is older than min_time (expired)
    /// - The order already exists in the window (duplicate)
    pub fn put(&mut self, order_id: u64, now_time: i64) -> Result<(), CustomError> {
        // Extract timestamp from order_id (snowflake ID structure: timestamp is in upper 41 bits)
        let order_timestamp = self.extract_timestamp(order_id);

        // Slide window forward if needed - this ensures cleanup even for expired orders
        if now_time > self.max_time {
            self.max_time = now_time;

            // Slide min_time forward, but cap it at max_time - duration
            let new_min_time = self.max_time - self.duration;
            if new_min_time > self.min_time {
                // Remove expired orders (orders with timestamp < new_min_time)
                self.remove_expired_internal(new_min_time);
                self.min_time = new_min_time;
            }
        }

        // Check if order is expired (older than window start)
        if order_timestamp < self.min_time {
            return Err(CustomError::from_string(format!(
                "expired order {}, orderTime={}, window [{},{})",
                order_id, order_timestamp, self.min_time, self.max_time
            )));
        }

        // Check if order already exists
        let key = TimeOrderIdKey {
            timestamp: order_timestamp,
            order_id,
        };
        if self.orders.contains_key(&key) {
            return Err(CustomError::from_string(format!(
                "duplicate order {} at timestamp {}",
                order_id, order_timestamp
            )));
        }

        // Add order to window
        self.orders.insert(key, true);
        Ok(())
    }

    /// Removes expired orders based on current time.
    /// This should be called periodically to prevent memory leaks when there are no new orders.
    pub fn cleanup(&mut self, now_time: i64) {
        if now_time > self.max_time {
            let new_min_time = now_time - self.duration;
            if new_min_time > self.min_time {
                self.remove_expired_internal(new_min_time);
                self.min_time = new_min_time;
                self.max_time = now_time;
            }
        }
    }

    /// Returns the number of orders in the window.
    pub fn size(&self) -> usize {
        self.orders.len()
    }

    /// Checks if an order exists in the window.
    pub fn contains(&self, order_id: u64) -> bool {
        let order_timestamp = self.extract_timestamp(order_id);
        let key = TimeOrderIdKey {
            timestamp: order_timestamp,
            order_id,
        };
        self.orders.contains_key(&key)
    }

    /// Removes all orders with timestamp < min_time.
    fn remove_expired_internal(&mut self, min_time: i64) {
        // Collect keys to remove
        let keys_to_remove: Vec<TimeOrderIdKey> = self
            .orders
            .keys()
            .filter(|k| k.timestamp < min_time)
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.orders.remove(&key);
        }
    }

    /// Returns a snapshot of the window state for serialization.
    pub fn snapshot(&self) -> TimeWindowSnapshot {
        TimeWindowSnapshot {
            min_time: self.min_time,
            max_time: self.max_time,
            duration: self.duration,
            epoch: SNOWFLAKE_EPOCH,
        }
    }

    /// Restores the window from a snapshot.
    /// Note: epoch is ignored since it's always SNOWFLAKE_EPOCH.
    pub fn restore(&mut self, snapshot: &TimeWindowSnapshot) {
        self.min_time = snapshot.min_time;
        self.max_time = snapshot.max_time;
        self.duration = snapshot.duration;
        // Orders are not restored from snapshot - they will be repopulated as orders come in
        self.orders.clear();
    }
}

/// Snapshot of TimeWindow state for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindowSnapshot {
    pub min_time: i64,
    pub max_time: i64,
    pub duration: i64,
    pub epoch: i64,
}

impl Default for TimeWindowSnapshot {
    fn default() -> Self {
        TimeWindowSnapshot {
            min_time: 0,
            max_time: 0,
            duration: TIME_WINDOW_DURATION,
            epoch: SNOWFLAKE_EPOCH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Base timestamp for tests: Jan 1, 2020 (1577836800000 ms)
    const TEST_BASE_TIME: i64 = 1577836800000;

    fn make_snowflake_id(timestamp_ms: i64, node_id: u64, sequence: u64) -> u64 {
        // timestamp_ms should be an absolute timestamp (ms since Unix epoch)
        // Snowflake stores (timestamp - SNOWFLAKE_EPOCH) in the ID
        let timestamp_part = (timestamp_ms - SNOWFLAKE_EPOCH) as u64;
        (timestamp_part << 22) | ((node_id & 0x3FF) << 12) | (sequence & 0xFFF)
    }

    #[test]
    fn test_extract_timestamp() {
        let window = TimeWindow::new();

        // Test with a known timestamp (absolute, greater than SNOWFLAKE_EPOCH)
        let timestamp = 1700000000000i64; // Nov 14, 2023
        let order_id = make_snowflake_id(timestamp, 1, 1);

        let extracted = window.extract_timestamp(order_id);
        assert_eq!(extracted, timestamp);
    }

    #[test]
    fn test_extract_node_id() {
        // Test extracting node ID from Snowflake ID
        for node_id in [0, 1, 50, 127, 255, 1023].iter() {
            let order_id = make_snowflake_id(1700000000000, *node_id, 123);
            assert_eq!(extract_node_id_from_id(order_id), *node_id);
        }
    }

    #[test]
    fn test_table_sharding() {
        // Test that order ID and user ID route to the same shard when node_id == user_id % split_count
        let split_count = DEFAULT_TABLE_SPLIT_COUNT;

        for user_id in [0, 1, 100, 127, 128, 255, 1000, 12345].iter() {
            // When generating an order ID, the node ID should equal user_id % split_count
            let node_id = user_id % split_count;
            let order_id = make_snowflake_id(1700000000000, node_id, 0);

            // Verify both route to the same shard
            assert_eq!(
                get_table_index_by_order_id(order_id, split_count),
                get_table_index_by_user_id(*user_id, split_count),
                "Order ID {} and user ID {} should route to same shard",
                order_id, user_id
            );
        }
    }

    #[test]
    fn test_extract_timestamp_function() {
        // Test the standalone function
        let timestamp = 1700000000000i64;
        let order_id = make_snowflake_id(timestamp, 1, 1);

        assert_eq!(extract_timestamp_from_id(order_id), timestamp);
    }

    #[test]
    fn test_put_and_contains() {
        let mut window = TimeWindow::new();
        let now = TEST_BASE_TIME + 100000; // 100 seconds after base time

        let order_id = make_snowflake_id(now, 1, 1);

        // First put should succeed
        assert!(window.put(order_id, now).is_ok());

        // Should be contained
        assert!(window.contains(order_id));

        // Second put with same ID should fail (duplicate)
        assert!(window.put(order_id, now).is_err());
    }

    #[test]
    fn test_expired_order() {
        let mut window = TimeWindow::new();
        window.duration = 10000; // 10 seconds

        // First, advance the window by adding an order at t=TEST_BASE_TIME + 50000
        let now = TEST_BASE_TIME + 50000;
        let order_id1 = make_snowflake_id(now, 1, 1);
        window.put(order_id1, now).unwrap();

        // After this, min_time = now - 10000 = TEST_BASE_TIME + 40000

        // Try to add an expired order with timestamp TEST_BASE_TIME + 30000 (< min_time)
        let old_time = TEST_BASE_TIME + 30000;
        let old_order_id = make_snowflake_id(old_time, 1, 2);

        let result = window.put(old_order_id, now);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[test]
    fn test_window_sliding() {
        let mut window = TimeWindow::new();
        window.duration = 10000; // 10 seconds

        // Add order at t=TEST_BASE_TIME + 10000 with now_time=TEST_BASE_TIME + 10000
        // min_time becomes TEST_BASE_TIME + 0
        let t1 = TEST_BASE_TIME + 10000;
        let id1 = make_snowflake_id(t1, 1, 1);
        window.put(id1, t1).unwrap();
        assert!(window.contains(id1));

        // Advance window to t=TEST_BASE_TIME + 25000
        // min_time becomes TEST_BASE_TIME + 15000
        // Order at TEST_BASE_TIME + 10000 should be removed (10000 < 15000)
        let t2 = TEST_BASE_TIME + 25000;
        let id2 = make_snowflake_id(t2, 1, 2);
        window.put(id2, t2).unwrap();

        // First order should be removed (timestamp TEST_BASE_TIME + 10000 < min_time TEST_BASE_TIME + 15000)
        assert!(!window.contains(id1));
        assert!(window.contains(id2));
    }

    #[test]
    fn test_cleanup() {
        let mut window = TimeWindow::new();
        window.duration = 10000; // 10 seconds

        // Add order at TEST_BASE_TIME + 10000
        let t1 = TEST_BASE_TIME + 10000;
        let id1 = make_snowflake_id(t1, 1, 1);
        window.put(id1, t1).unwrap();

        // Cleanup at TEST_BASE_TIME + 25000 (15 seconds later, outside 10s window)
        window.cleanup(TEST_BASE_TIME + 25000);

        // First order should be removed
        assert!(!window.contains(id1));
    }
}
