use log::{debug, error}; // Use error for actual errors
use tonic::{transport::Channel, Status};

use self::kafka_service::{kafka_service_client::KafkaServiceClient, CreateTopicRequest};

pub mod kafka_service {
    tonic::include_proto!("kafka");
}

pub async fn create_topic(
    broker_string: &str,
    group_id: &str,
    client_id: &str,
    topic: &str,
) -> Result<(), KafkaGrpcError> {
    let mut client = match connect().await {
        Ok(grpc_client) => grpc_client,
        Err(err) => {
            error!("Error connecting to Kafka service: {}", err);
            return Err(KafkaGrpcError::Connection(err.into())); // Convert error to Box<dyn std::error::Error>
        }
    };

    let request = tonic::Request::new(CreateTopicRequest {
        broker_string: broker_string.into(),
        group_id: group_id.into(),
        client_id: client_id.into(),
        topic: topic.into(),
    });

    debug!(
        "Request for Kafka grpc service -> create topic: {:?}",
        request
    );

    let response = match client.create_topic(request).await {
        Ok(_) => (), // Successful response, handle if needed
        Err(status) => return Err(KafkaGrpcError::CreateTopic(status)),
    };

    debug!(
        "Response for Kafka grpc service -> create topic: {:?}",
        response
    );

    Ok(())
}

async fn connect() -> Result<KafkaServiceClient<Channel>, Box<dyn std::error::Error>> {
    Ok(KafkaServiceClient::connect("http://regent:50051").await?)
}

#[derive(Debug)]
pub enum KafkaGrpcError {
    Connection(Box<dyn std::error::Error>),
    CreateTopic(Status),
}
