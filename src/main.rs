use cdrs::authenticators::NoneAuthenticator;
use cdrs::cluster::session::new as new_session;
use cdrs::cluster::{ClusterTcpConfig, NodeTcpConfigBuilder};
use cdrs::load_balancing::RoundRobin;
use cdrs::query::QueryExecutor;

#[tokio::main]
async fn main() {
    let node = NodeTcpConfigBuilder::new("cassandra:9042", NoneAuthenticator {}).build();
    let cluster_config = ClusterTcpConfig(vec![node]);
    let no_compression =
        new_session(&cluster_config, RoundRobin::new()).expect("session should be created");
    let create_ks: &'static str = "CREATE KEYSPACE IF NOT EXISTS test_ks WITH REPLICATION = { \
            'class' : 'SimpleStrategy', 'replication_factor' : 1 };";
    match no_compression.query(create_ks) {
        Ok(_) => println!("Keyspace created successfully"),
        Err(e) => println!("Keyspace creation error: {:?}", e),
    }
}
