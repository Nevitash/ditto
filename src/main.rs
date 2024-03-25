mod datasource;

#[tokio::main]
async fn main() {
    sqlx::any::install_default_drivers();
    let url: &str = "postgres";
    let user_name = "os";
    let password = "os";
    let port = 5432;
    let database = "ditto";
    let connection_string = datasource::generate_connection_string(url, port, user_name, password, database)
        .unwrap();
    let pool = datasource::connect(&connection_string)
        .await
        .expect("No connection could be established");
    datasource::init(&pool).await.expect("Init failed.");
}
