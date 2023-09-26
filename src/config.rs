use futures_util::future::TryFutureExt;
use serde_derive::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::models::models::Product;

const CONFIG_FILE_NAME: &'static str = "config.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KafkaConfig {
    pub brokers: Vec<String>,
    pub message_timeout: u64,
    pub session_timeout: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub product: Product,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub log: LogConfig,
}

pub async fn read_config() -> Config {
    let mut file = File::open(CONFIG_FILE_NAME)
        .await
        .unwrap_or_else(|e| panic!("open config file: {}", e));

    let mut file_str = String::new();
    let _ = file
        .read_to_string(&mut file_str)
        .await
        .unwrap_or_else(|e| panic!("read config file: {}", e));

    let config: Config =
        serde_json::from_str(&file_str).unwrap_or_else(|e| panic!("serde config json: {}", e));

    config
}
