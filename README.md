# orderbook-rs

A high-performance order matching engine built with Rust and Tokio.

![Architecture](https://github.com/CheetahExchange/orderbook-rs/blob/main/asset/png/simple_architecture.png)

## Overview

**orderbook-rs** is a high-performance cryptocurrency order matching engine designed for exchanges and trading platforms. It leverages Rust's safety guarantees and Tokio's async runtime to provide [...]

### Key Features

- **High Performance**: Built on Tokio async runtime for optimal throughput
- **Multiple Order Types**: Supports limit orders, market orders
- **Time-in-Force Options**: GTC (Good Till Canceled), IOC (Immediate Or Cancel), GTX (Good Till Crossing), FOK (Fill Or Kill)
- **Price-Time Priority**: Orders at the same price level are matched in arrival order (FIFO)
- **Fault Tolerance**: State persistence via Redis snapshots for crash recovery
- **Event Sourcing**: All matching events published to Kafka for audit and downstream processing

### Origin

This project is based on the matching engine from [gitbitex-spot](https://github.com/gitbitex/gitbitex-spot), with improvements and refinements for better performance and maintainability.

## Architecture

The engine consists of four concurrent tasks working together:

1. **Fetcher**: Reads orders from Kafka input topic
2. **Applier**: Applies orders to the order book and generates matching logs
3. **Committer**: Commits logs to Kafka output topic
4. **Snapshots**: Periodically saves order book state to Redis

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Kafka Order    │────▶│  Matching Engine │────▶│  Kafka Log      │
│  (Order Input)  │     │  (Core Logic)    │     │  (Event Output) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌─────────────────┐
                        │  Redis Snapshot │
                        │  (State Backup) │
                        └─────────────────┘
```

## Order Types

### Limit Orders
Orders with a specified price. They will be matched against opposing orders at the same or better price, or placed on the book if no immediate match is available.

### Market Orders
Orders executed immediately at the best available price. Market buy orders use `funds` field to specify the quote currency amount to spend.

## Order ID Format

Orders submitted to this engine must use **Snowflake IDs** as order identifiers. Order IDs are generated externally (typically by the order service) and passed to the matching engine via Kafka.

### Snowflake ID Structure (64 bits)

```
| 1 bit |     41 bits      |   10 bits   |   12 bits   |
| Sign  | Timestamp (ms)   | Node ID     | Sequence    |
```

- **Sign bit (1 bit)**: Always 0 for positive IDs
- **Timestamp (41 bits)**: Milliseconds since Snowflake epoch (Nov 04 2010 01:42:54 UTC)
- **Node ID (10 bits)**: Values 0-1023, used for database sharding
- **Sequence (12 bits)**: Values 0-4095, for multiple IDs per millisecond

### Order ID Generation Rule

When generating an order ID externally, the Node ID should be set to `user_id % 128`:

```go
// Example (Go with bwmarrin/snowflake library)
node := GetTableNodeByUserId(userId)  // node ID = userId % 128
order.Id = node.Generate().Int64()
```

This ensures:
- Orders from the same user always route to the same database shard
- Shard index can be extracted from order ID: `(order_id >> 12) % 128`

### Why Snowflake IDs?

The primary purpose is **database sharding**. The Node ID embedded in the order ID enables:

1. **Consistent Routing**: Orders from the same user route to the same shard table
2. **Efficient Lookups**: Query by user_id or order_id both lead to the correct shard
3. **High Throughput**: Distributed ID generation without coordination

### Time-Based Deduplication

The matching engine uses a **time-based sliding window** (30 seconds) for order deduplication.

Clarification about timestamp extraction:

- The 41-bit timestamp field inside a Snowflake ID stores milliseconds *since the Snowflake epoch* (SNOWFLAKE_EPOCH = 1288834974657, i.e. 2010-11-04T01:42:54Z).
- Extract the relative timestamp (ms since Snowflake epoch) with:

```rust
fn extract_relative_ms(order_id: u64) -> i64 {
    (order_id >> 22) as i64
}
```

- To obtain an absolute Unix epoch millisecond timestamp, add SNOWFLAKE_EPOCH:

```rust
fn extract_unix_ms(order_id: u64) -> i64 {
    extract_relative_ms(order_id) + SNOWFLAKE_EPOCH
}
```

Note on repository code: the code in `src/utils/time_window.rs` and related functions uses the *relative* timestamp (ms since SNOWFLAKE_EPOCH) when comparing against the window and when calling `current_time_since_snowflake_epoch()` — so do NOT add SNOWFLAKE_EPOCH in those comparisons. If you need a human-readable Unix timestamp (or to log wall-clock time), then add SNOWFLAKE_EPOCH.

This approach is more reliable than fixed-capacity ID windows for Snowflake IDs, as the ID range can grow rapidly (up to ~4M sequence values per millisecond per node).

## Time-in-Force Options

| Type | Code | Description |
|------|------|-------------|
| Good Till Canceled | GTC | Order remains active until filled or cancelled |
| Immediate Or Cancel | IOC | Execute immediately, cancel any unfilled portion |
| Good Till Crossing | GTX | Only place order if it won't match immediately (maker-only) |
| Fill Or Kill | FOK | Execute entire order immediately or cancel entirely |

## Installing Dependencies

### Install Rust Compiler

```bash
sudo apt-get update
sudo apt-get install git curl build-essential

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Install Redis and Kafka

```bash
sudo apt-get install redis-server

# For Kafka, follow instructions at https://kafka.apache.org/
```

## Build

```bash
git clone https://github.com/CheetahExchange/orderbook-rs
cd orderbook-rs

cargo build --release
```

## Configuration

Create `config.json` with your settings:

```json
{
  "product": {
    "id": "BTC-USD",
    "base_currency": "BTC",
    "quote_currency": "USD",
    "base_scale": 8,
    "quote_scale": 2
  },
  "redis": {
    "ip": "127.0.0.1",
    "port": 6379
  },
  "kafka": {
    "brokers": ["localhost:9092"],
    "message_timeout": 5000,
    "session_timeout": 10000,
    "group_id": "matching_engine"
  },
  "log": {
    "level": "info"
  }
}
```

## Run

```bash
./target/release/orderbook-rs
```

## Kafka Topics

| Topic Pattern | Direction | Description |
|---------------|-----------|-------------|
| `matching_order_{product_id}` | Input | Orders to be processed |
| `matching_message_{product_id}` | Output | Matching events (match, open, done) |

## Log Types

### Match Log
Generated when a trade is executed:
```json
{
  "base": {
    "type": "match",
    "sequence": 1,
    "product_id": "BTC-USD",
    "time": 1695783003020967000
  },
  "trade_seq": 1,
  "taker_order_id": 1001,
  "maker_order_id": 1002,
  "taker_user_id": 1,
  "maker_user_id": 2,
  "side": "buy",
  "price": "50000.00",
  "size": "0.5"
}
```

### Open Log
Generated when an order is placed on the book:
```json
{
  "base": {
    "type": "open",
    "sequence": 2,
    "product_id": "BTC-USD",
    "time": 1695783003020967000
  },
  "order_id": 1001,
  "user_id": 1,
  "remaining_size": "0.5",
  "price": "50000.00",
  "side": "buy",
  "time_in_force": "GTC"
}
```

### Done Log
Generated when an order is completed (filled or cancelled):
```json
{
  "base": {
    "type": "done",
    "sequence": 3,
    "product_id": "BTC-USD",
    "time": 1695783003020967000
  },
  "order_id": 1001,
  "user_id": 1,
  "price": "50000.00",
  "remaining_size": "0.0",
  "reason": "filled",
  "side": "buy",
  "time_in_force": "GTC"
}
```

## Testing

See [TEST_GUIDE.md](TEST_GUIDE.md) for detailed testing instructions.

### Python Test Example

```python
#!/usr/bin/env python
# encoding: utf-8

import json
from kafka import KafkaProducer
from decimal import Decimal

producer = KafkaProducer(bootstrap_servers='127.0.0.1:9092')

order = {
    "id": 1001,
    "created_at": 1695783003020967000,
    "product_id": "BTC-USD",
    "user_id": 1,
    "client_oid": "",
    "price": "50000.00",
    "size": "1.0",
    "funds": "0.00",
    "type": "limit",
    "side": "buy",
    "time_in_force": "GTC",
    "status": "new"
}

producer.send('matching_order_BTC-USD', json.dumps(order).encode("utf8"))
producer.flush()
producer.close()
```

## License

This project is open source. Please refer to the LICENSE file for details.

## Acknowledgments

This project is based on the matching engine from [gitbitex-spot](https://github.com/gitbitex/gitbitex-spot).
