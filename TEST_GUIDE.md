# Matching Engine Test Guide

## Test Environment

- **Kafka Broker**: 192.168.1.123:9092
- **Redis**: 127.0.0.1:6379
- **Product**: BTC-USD

## Test Programs

### 1. Matching Engine (orderbook-rs.exe)
Main program that reads orders from Kafka, executes matching, and outputs logs to Kafka.

### 2. Log Verifier (log_verifier.exe)
Consumes from `matching_message_BTC-USD` topic and displays matching results:
- **MATCH**: Trade executed
- **OPEN**: Order placed on the book
- **DONE**: Order completed (filled or cancelled)

### 3. Test Order Sender (test_order_sender.exe)
Sends a series of test orders to verify matching logic.

## Test Cases

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| 1 | Basic limit order matching | Buy 1 BTC @ 50000, Sell 0.5 BTC matches |
| 2 | Partial fill | Sell 0.3 BTC @ 49900 matches at 50000 |
| 3 | Order cancellation | Cancel remaining 0.2 BTC order |
| 4 | IOC order | Only matches available portion, rest cancelled |
| 5 | GTX order | Order that would immediately match is rejected |
| 6 | FOK order | Rejected when cannot be fully filled |
| 7 | Market order | Executes based on funds amount |
| 8 | Price-time priority | Orders at same price matched in time order |
| 9 | Duplicate order ID | Second submission is rejected |

## Running the Tests

```bash
# Terminal 1: Start the matching engine
./target/release/orderbook-rs.exe

# Terminal 2: Start the log monitor
./target/release/log_verifier.exe

# Terminal 3: Send test orders
./target/release/test_order_sender.exe
```

## Verification Points

1. **MATCH log**: Verify trade_seq, price, size are correct
2. **OPEN log**: Verify remaining_size is correct
3. **DONE log**: Verify reason (filled/cancelled)
4. **Sequence continuity**: Log sequence should increment continuously
