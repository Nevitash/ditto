use sqlx::{query_as, Error, PgPool};
use uuid::Uuid;

static MIGRATION_TABLE_EXISTS_QUERY: &str =
    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'migrations')::integer";
static MIGRATION_TABLE_CREATE_QUERY: &str = "CREATE TABLE migrations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    migration_number bigint NOT NULL,
    script_name varchar(255) NOT NULL,
    success boolean DEFAULT false,
    message text
)";
static MIGRATION_ENABLE_CRYPTO_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto""#;
static MIGRATION_ENABLE_UUID_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#;

#[derive(sqlx::FromRow, Debug)]
struct Migration {
    id: Uuid,
    migration_number: i64,
    script_name: String,
    success: bool,
    message: Option<String>,
}

pub async fn connect(url: &String) -> Result<PgPool, Error> {
    match PgPool::connect(url).await {
        Ok(pool) => {
            println!("Successfully connected to the database.");
            return Ok(pool);
        }
        Err(e) => {
            println!("Failed to connect to the database. Error: {}", e);
            return Err(e);
        }
    }
}

async fn enable_uuid_plugins(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(MIGRATION_ENABLE_CRYPTO_PLUGIN)
        .execute(pool)
        .await
        .expect("Could not enable pgcrypto plugin. This is needed to create uuids in postgres");
    sqlx::query(MIGRATION_ENABLE_UUID_PLUGIN)
        .execute(pool)
        .await
        .expect("Could not enable uuid-ossp plugin. This is needed to create uuids in postgres");
    return Ok(());
}

pub fn generate_connection_string(
    url: &str,
    port: u32,
    user_name: &str,
    password: &str,
    database: &str,
) -> Result<String, Error> {
    return Ok(format!(
        "postgres://{}:{}@{}:{}/{}",
        user_name, password, url, port, database
    ));
}

pub async fn init(pool: &PgPool) -> Result<(), Error> {
    enable_uuid_plugins(pool).await?;
    let (table_exists,): (i32,) = query_as(MIGRATION_TABLE_EXISTS_QUERY)
        .fetch_one(pool)
        .await
        .unwrap();

    if table_exists == 1 {
        return Ok(());
    }

    sqlx::query(MIGRATION_TABLE_CREATE_QUERY)
        .execute(pool)
        .await?;

    return Ok(());
}
