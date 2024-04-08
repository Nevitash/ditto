use rdkafka::{client::Client, ClientConfig};

async fn connect(client_id:&str, brokers: &str){
    let mut config = ClientConfig::new();
    config.set(KafkaConfigProperty::BOOTSTRAP_SERVERS, brokers);
    config.set(KafkaConfigProperty::CLIENT_ID, client_id);
    let client = Client::new(&config)?;
}

#[non_exhaustive]
pub struct KafkaConfigProperty;
impl KafkaConfigProperty {
    pub const BOOTSTRAP_SERVERS: &str = "bootstrap.servers";
    pub const CLIENT_ID: &str = "client.id";
    pub const MESSAGE_TIMEOUT_MS: &str = "message.timeout.ms";
    pub const ENABLE_AUTO_COMMIT: &str = "enable.auto.commit";
    pub const AUTO_OFFSET_COMMIT: &str = "auto.offset.reset";
    pub const GROUP_ID: &str = "group.id";
    pub const MAX_POLL_RECORDS: &str = "max.poll.records";
}