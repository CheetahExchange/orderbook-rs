use std::time::Duration;

use chrono::Utc;
use rdkafka::client::DefaultClientContext;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json;

// Import from the library
use orderbook_rs::models::models::Order;
use orderbook_rs::models::types::*;

const BROKERS: &str = "192.168.1.123:9092";
const PRODUCT_ID: &str = "BTC-USD";

fn create_producer() -> FutureProducer<DefaultClientContext> {
    rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", BROKERS)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation failed")
}

fn create_order(
    order_id: u64,
    user_id: u64,
    side: Side,
    price: Decimal,
    size: Decimal,
    order_type: OrderType,
    time_in_force: TimeInForceType,
    status: OrderStatus,
) -> Order {
    Order {
        id: order_id,
        created_at: Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
        product_id: PRODUCT_ID.to_string(),
        user_id,
        client_oid: format!("client_{}", order_id),
        price,
        size,
        funds: Decimal::ZERO,
        r#type: order_type,
        side,
        time_in_force,
        status,
    }
}

async fn send_order(producer: &FutureProducer<DefaultClientContext>, order: &Order) {
    let topic = format!("matching_order_{}", PRODUCT_ID);
    let payload = serde_json::to_string(order).expect("Serialize order failed");

    let result = producer
        .send(
            FutureRecord::to(&topic).payload(&payload).key(&order.id.to_string()),
            Timeout::After(Duration::from_secs(5)),
        )
        .await;

    match result {
        Ok(_) => println!("[SENT] Order {} - {:?} {} @ {}", order.id, order.side, order.size, order.price),
        Err((e, _)) => println!("[ERROR] Failed to send order {}: {}", order.id, e),
    }
}

fn print_test_header(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}", "=".repeat(60));
}

#[tokio::main]
async fn main() {
    let producer = create_producer();

    // Wait for any previous messages to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    print_test_header("TEST 1: Basic Limit Order Matching");
    println!("Scenario: Place buy order, then matching sell order");

    // Order 1: Buy 1 BTC @ 50000 (GTC limit order)
    let buy_order = create_order(
        1001,
        1,
        Side::SideBuy,
        dec!(50000.00),
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &buy_order).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 2: Sell 0.5 BTC @ 50000 (should match with buy order)
    let sell_order = create_order(
        1002,
        2,
        Side::SideSell,
        dec!(50000.00),
        dec!(0.5),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_order).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 2: Partial Fill and Remaining");
    println!("Scenario: Sell order partially fills buy order, check remaining");

    // Order 3: Sell 0.3 BTC @ 49900 (should match at 50000)
    let sell_order2 = create_order(
        1003,
        3,
        Side::SideSell,
        dec!(49900.00),
        dec!(0.3),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_order2).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 3: Order Cancellation");
    println!("Scenario: Cancel remaining buy order");

    // Order 4: Cancel the remaining buy order
    let cancel_order = create_order(
        1001, // Same order_id as the buy order
        1,
        Side::SideBuy,
        dec!(50000.00),
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusCancelling,
    );
    send_order(&producer, &cancel_order).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 4: IOC (Immediate Or Cancel)");
    println!("Scenario: IOC order that partially matches");

    // Order 5: Place new buy order for IOC test
    let buy_order2 = create_order(
        1005,
        4,
        Side::SideBuy,
        dec!(51000.00),
        dec!(2.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &buy_order2).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 6: IOC sell order - match what it can, cancel rest
    let ioc_sell = create_order(
        1006,
        5,
        Side::SideSell,
        dec!(51000.00),
        dec!(3.0), // More than available
        OrderType::OrderTypeLimit,
        TimeInForceType::ImmediateOrCancel,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &ioc_sell).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 5: GTX (Good Till Crossing) - Maker Only");
    println!("Scenario: GTX order that would cross is rejected");

    // Order 7: New sell order on the book
    let sell_gtx = create_order(
        1007,
        6,
        Side::SideSell,
        dec!(52000.00),
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_gtx).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 8: GTX buy that would cross - should be cancelled
    let gtx_buy = create_order(
        1008,
        7,
        Side::SideBuy,
        dec!(52000.00), // Would match with sell @ 52000
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCrossing,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &gtx_buy).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 9: GTX buy that won't cross - should be placed
    let gtx_buy2 = create_order(
        1009,
        8,
        Side::SideBuy,
        dec!(51000.00), // Below best ask
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCrossing,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &gtx_buy2).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 6: FOK (Fill Or Kill)");
    println!("Scenario: FOK order that cannot be fully filled is rejected");

    // Order 10: Small sell order
    let sell_fok = create_order(
        1010,
        9,
        Side::SideSell,
        dec!(53000.00),
        dec!(0.5),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_fok).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 11: FOK buy order requesting more than available
    let fok_buy = create_order(
        1011,
        10,
        Side::SideBuy,
        dec!(53000.00),
        dec!(1.0), // More than available 0.5
        OrderType::OrderTypeLimit,
        TimeInForceType::FillOrKill,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &fok_buy).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 12: FOK buy order that CAN be filled
    let fok_buy2 = create_order(
        1012,
        11,
        Side::SideBuy,
        dec!(53000.00),
        dec!(0.5), // Exactly available
        OrderType::OrderTypeLimit,
        TimeInForceType::FillOrKill,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &fok_buy2).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 7: Market Order");
    println!("Scenario: Market order execution");

    // Order 13: Add liquidity for market order
    let sell_market_liquidity = create_order(
        1013,
        12,
        Side::SideSell,
        dec!(54000.00),
        dec!(2.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_market_liquidity).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 14: Market buy order
    let mut market_buy = create_order(
        1014,
        13,
        Side::SideBuy,
        Decimal::ZERO, // Price ignored for market orders
        dec!(1.0),
        OrderType::OrderTypeMarket,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    market_buy.funds = dec!(100000.00); // Buy with $100k worth
    send_order(&producer, &market_buy).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 8: Price-Time Priority");
    println!("Scenario: Orders at same price are matched in order");

    // Order 15-17: Multiple buy orders at same price
    for i in 15u64..=17u64 {
        let buy_order = create_order(
            1000 + i,
            20 + i,
            Side::SideBuy,
            dec!(55000.00),
            dec!(0.5),
            OrderType::OrderTypeLimit,
            TimeInForceType::GoodTillCanceled,
            OrderStatus::OrderStatusNew,
        );
        send_order(&producer, &buy_order).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Order 18: Sell that matches multiple orders
    let sell_multi = create_order(
        1018,
        30,
        Side::SideSell,
        dec!(55000.00),
        dec!(1.0), // Should match first two buy orders
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &sell_multi).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    print_test_header("TEST 9: Duplicate Order ID");
    println!("Scenario: Send same order_id twice");

    // Order 21: First order
    let dup_order = create_order(
        1021,
        50,
        Side::SideBuy,
        dec!(57000.00),
        dec!(1.0),
        OrderType::OrderTypeLimit,
        TimeInForceType::GoodTillCanceled,
        OrderStatus::OrderStatusNew,
    );
    send_order(&producer, &dup_order).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Order 21 again: Duplicate (should be rejected by order_id_window)
    send_order(&producer, &dup_order).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("\n{}", "=".repeat(60));
    println!("  ALL TESTS SENT - Check matching engine logs for results");
    println!("  Expected Kafka topics:");
    println!("    - Input:  matching_order_{}", PRODUCT_ID);
    println!("    - Output: matching_message_{}", PRODUCT_ID);
    println!("  Run matching engine and consume matching_message to verify");
    println!("{}", "=".repeat(60));
}