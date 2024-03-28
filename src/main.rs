use std::path::Path;

use log::{debug, info, LevelFilter};
use log4rs::{append::console::ConsoleAppender, config::{Appender, Logger, Root}, Config};

mod datasource;
mod migration;
extern crate derive_builder;

#[tokio::main]
async fn main() {
    init_logging().await;
    db_migration().await;
}

async fn init_logging() {
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("sqlx", LevelFilter::Warn))
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
        .unwrap();
    log4rs::init_config(config).unwrap();
    info!("log4rs is initialized, ready to log!");
}

async fn db_migration(){
    debug!("installing db drivers.");
    sqlx::any::install_default_drivers();
    debug!("db drivers installed");
    let url: &str = "postgres";
    let user_name = "os";
    let password = "os";
    let port = 5432;
    let database = "ditto";
    let connection_string = datasource::generate_connection_string(url, port, Some(user_name), Some(password), Some(database));
    info!("Connection string, {}", connection_string);
    let pool = datasource::connect(&connection_string)
        .await
        .expect("No connection could be established");
    info!("DB pool is connected");
    migration::init(&pool).await.expect("Init failed.");
    migration::migrate(&pool, Path::new("src/files/db/migrations")).await.expect("DB migration failed");
}
