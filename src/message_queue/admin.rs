use derive_builder::Builder;
use log::error;

use crate::grpc::{self, kafka::KafkaGrpcError};

#[derive(Debug)]
pub enum AdminOperationError {
    Connection(String),
    Operation(String),
    Config(KafkaConfigBuilderError),
}

pub async fn create_topic(topic: &str) -> Result<(), AdminOperationError> {
    let config = get_kafka_config()?;
    match grpc::kafka::create_topic(&config.broker_string,&config.group_id,&config.client_id,topic,).await {
        Ok(_) => Ok(()),
        Err(_) => {
            error!("Could not create topic {}", topic);
            Err(AdminOperationError::Operation(format!("Could not create topic {}", topic)))
        }
    }
}

fn get_kafka_config() -> Result<KafkaConfig, AdminOperationError> {
    match KafkaConfigBuilder::default()
        .broker_string("kafka:9092,kafka2:9092".into())
        .client_id("admin".into())
        .group_id("ditto".into())
        .build()
    {
        Ok(config) => Ok(config),
        Err(error) => {
            error!("Could not read config for kafka grpc service!");
            Err(AdminOperationError::Config(error))
        }
    }
}

#[derive(Debug, Builder)]
struct KafkaConfig {
    broker_string: String,
    client_id: String,
    group_id: String,
}
