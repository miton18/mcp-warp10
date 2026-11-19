use std::sync::Mutex;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KafkaError {
    #[error("kafka produce failed: {0}")]
    Produce(String),
}

#[derive(Debug, Clone)]
pub struct SentMessage {
    pub topic: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub trait Producer: Send + Sync {
    fn send(&self, topic: &str, key: &[u8], value: &[u8]) -> Result<(), KafkaError>;
}

pub struct MockProducer {
    pub sent: Mutex<Vec<SentMessage>>,
}

impl MockProducer {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }
}

impl Producer for MockProducer {
    fn send(&self, topic: &str, key: &[u8], value: &[u8]) -> Result<(), KafkaError> {
        self.sent.lock().unwrap().push(SentMessage {
            topic: topic.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
        });
        Ok(())
    }
}

#[cfg(feature = "kafka")]
pub struct RdkafkaProducer {
    producer: rdkafka::producer::FutureProducer,
}

#[cfg(feature = "kafka")]
impl RdkafkaProducer {
    pub fn new(brokers: &str) -> Result<Self, KafkaError> {
        let producer = rdkafka::ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()
            .map_err(|e| KafkaError::Produce(e.to_string()))?;
        Ok(Self { producer })
    }
}

#[cfg(feature = "kafka")]
impl Producer for RdkafkaProducer {
    fn send(&self, topic: &str, key: &[u8], value: &[u8]) -> Result<(), KafkaError> {
        use rdkafka::producer::FutureRecord;
        let record = FutureRecord::to(topic).key(key).payload(value);
        self.producer
            .send(record, std::time::Duration::from_secs(5))
            .map_err(|(e, _)| KafkaError::Produce(e.to_string()))?;
        Ok(())
    }
}
