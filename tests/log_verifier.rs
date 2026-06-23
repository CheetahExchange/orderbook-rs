use rdkafka::consumer::{stream_consumer::StreamConsumer, Consumer};
use rdkafka::message::Message;
use serde_json::Value;

const BROKERS: &str = "192.168.1.123:9092";
const PRODUCT_ID: &str = "BTC-USD";
const GROUP_ID: &str = "log_verifier_group";

#[tokio::main]
async fn main() {
    println!("=== Matching Log Verifier ===");
    println!("Consuming from: matching_message_{}", PRODUCT_ID);
    println!();

    let consumer: StreamConsumer<_> = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", BROKERS)
        .set("group.id", GROUP_ID)
        .set("enable.partition.eof", "false")
        .set("api.version.request", "true")
        .set("broker.version.fallback", "2.1.0")
        .set("session.timeout.ms", "10000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed");

    let topic = format!("matching_message_{}", PRODUCT_ID);
    consumer.subscribe(&[topic.as_str()]).expect("Subscribe failed");

    println!("Waiting for matching logs...");
    println!("{}", "-".repeat(80));

    let mut match_count = 0;
    let mut open_count = 0;
    let mut done_count = 0;
    let mut last_seq = 0u64;

    loop {
        match consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    let json_str = String::from_utf8_lossy(payload);
                    let offset = message.offset();

                    // Debug: print raw JSON
                    println!("[RAW] offset={} payload={}", offset, &json_str);

                    match serde_json::from_str::<Value>(&json_str) {
                        Ok(log_value) => {
                            // type is nested inside "base" object
                            let base = &log_value["base"];
                            let log_type = base["type"].as_str().unwrap_or("unknown");
                            let seq = base["sequence"].as_u64().unwrap_or(0);

                            // Check sequence continuity
                            if last_seq > 0 && seq != last_seq + 1 {
                                println!("⚠️  SEQUENCE GAP: {} -> {} (expected {})", last_seq, seq, last_seq + 1);
                            }
                            last_seq = seq;

                            match log_type {
                                "match" => {
                                    match_count += 1;
                                    print_match_log(&log_value, offset);
                                }
                                "open" => {
                                    open_count += 1;
                                    print_open_log(&log_value, offset);
                                }
                                "done" => {
                                    done_count += 1;
                                    print_done_log(&log_value, offset);
                                }
                                _ => {
                                    println!("[UNKNOWN] offset={}, type={}", offset, log_type);
                                }
                            }

                            // Print summary
                            println!("--- Summary: Match={}, Open={}, Done={} ---\n",
                                match_count, open_count, done_count);
                        }
                        Err(e) => {
                            println!("[PARSE ERROR] offset={}, error={}", offset, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("[CONSUME ERROR] {}", e);
            }
        }
    }
}

fn print_match_log(log: &Value, offset: i64) {
    let base = &log["base"];
    println!("[MATCH] offset={} seq={}", offset, base["sequence"].as_u64().unwrap_or(0));
    println!("  Trade #: {}", log["trade_seq"].as_u64().unwrap_or(0));
    println!("  Price: {} | Size: {}", log["price"], log["size"]);
    println!("  Taker: order={}, user={}", log["taker_order_id"], log["taker_user_id"]);
    println!("  Maker: order={}, user={}", log["maker_order_id"], log["maker_user_id"]);
    println!("  Side: {}", log["side"]);
    println!();
}

fn print_open_log(log: &Value, offset: i64) {
    let base = &log["base"];
    println!("[OPEN] offset={} seq={}", offset, base["sequence"].as_u64().unwrap_or(0));
    println!("  Order: {} | User: {}", log["order_id"], log["user_id"]);
    println!("  Price: {} | Remaining: {}", log["price"], log["remaining_size"]);
    println!("  Side: {} | TIF: {}", log["side"], log["time_in_force"]);
    println!();
}

fn print_done_log(log: &Value, offset: i64) {
    let base = &log["base"];
    println!("[DONE] offset={} seq={}", offset, base["sequence"].as_u64().unwrap_or(0));
    println!("  Order: {} | User: {}", log["order_id"], log["user_id"]);
    println!("  Price: {} | Remaining: {}", log["price"], log["remaining_size"]);
    println!("  Reason: {}", log["reason"]);
    println!("  Side: {} | TIF: {}", log["side"], log["time_in_force"]);
    println!();
}