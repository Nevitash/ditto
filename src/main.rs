use std::path::Path;

mod datasource;
extern crate derive_builder;

#[tokio::main]
async fn main() {
    sqlx::any::install_default_drivers();
    let url: &str = "postgres";
    let user_name = "os";
    let password = "os";
    let port = 5432;
    let database = "ditto";
    let connection_string = datasource::generate_connection_string(url, port, Some(user_name), Some(password), Some(database));
    let pool = datasource::connect(&connection_string)
        .await
        .expect("No connection could be established");
    datasource::init(&pool).await.expect("Init failed.");
    datasource::migrate(&pool, Path::new("src/files/db/migrations")).await.unwrap_err();
}
