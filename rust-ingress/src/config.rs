use std::collections::HashMap;
use std::env;
use std::fs;

use thiserror::Error;

use crate::policy::{MissingBehavior, PolicyConfig, TokenClaims};
use crate::siphash::get_key;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing env var {0}")]
    MissingEnv(&'static str),
    #[error("invalid hex key {0}: {1}")]
    BadKey(&'static str, String),
    #[error("invalid tokens file: {0}")]
    BadTokensFile(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: String,
    pub kafka_brokers: String,
    pub data_topic: String,
    pub meta_topic: String,
    pub class_key: (u64, u64),
    pub labels_key: (u64, u64),
    pub tokens: HashMap<String, TokenClaims>,
    pub policy: PolicyConfig,
    pub use_kafka: bool,
}

fn env_or(key: &'static str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn key_from_env(name: &'static str) -> Result<(u64, u64), ConfigError> {
    let hexstr = env::var(name).map_err(|_| ConfigError::MissingEnv(name))?;
    let bytes = hex::decode(&hexstr).map_err(|e| ConfigError::BadKey(name, e.to_string()))?;
    if bytes.len() != 16 {
        return Err(ConfigError::BadKey(name, "expected 16 bytes".to_string()));
    }
    Ok(get_key(&bytes))
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let tokens = match env::var("RUST_INGRESS_TOKENS_FILE") {
            Ok(path) => {
                let content = fs::read_to_string(&path)
                    .map_err(|e| ConfigError::BadTokensFile(e.to_string()))?;
                serde_json::from_str(&content)
                    .map_err(|e| ConfigError::BadTokensFile(e.to_string()))?
            }
            Err(_) => HashMap::new(),
        };
        let missing = |name: &'static str| match env_or(name, "reject").as_str() {
            "default" => MissingBehavior::Default,
            _ => MissingBehavior::Reject,
        };
        Ok(Self {
            listen: env_or("RUST_INGRESS_LISTEN", "0.0.0.0:8082"),
            kafka_brokers: env_or("RUST_INGRESS_KAFKA_BROKERS", "localhost:9092"),
            data_topic: env_or("RUST_INGRESS_DATA_TOPIC", "data"),
            meta_topic: env_or("RUST_INGRESS_META_TOPIC", "metadata"),
            class_key: key_from_env("RUST_INGRESS_CLASS_KEY_HEX")?,
            labels_key: key_from_env("RUST_INGRESS_LABELS_KEY_HEX")?,
            tokens,
            policy: PolicyConfig {
                default_owner: env_or(
                    "RUST_INGRESS_DEFAULT_OWNER",
                    "00000000-0000-0000-0000-000000000000",
                ),
                default_producer: env_or(
                    "RUST_INGRESS_DEFAULT_PRODUCER",
                    "00000000-0000-0000-0000-000000000000",
                ),
                missing_owner: missing("RUST_INGRESS_MISSING_OWNER"),
                missing_producer: missing("RUST_INGRESS_MISSING_PRODUCER"),
                allow_owner_attr: env_or("RUST_INGRESS_ALLOW_OWNER_ATTR", ".allowowner"),
                allow_producer_attr: env_or("RUST_INGRESS_ALLOW_PRODUCER_ATTR", ".allowproducer"),
            },
            use_kafka: env_or("RUST_INGRESS_PRODUCER", "mock") == "kafka",
        })
    }
}
