use sqlx::{query_as, Error, PgPool};
use tokio::try_join;
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
    let enable_crypto = sqlx::query(MIGRATION_ENABLE_CRYPTO_PLUGIN).execute(pool);
    let enable_uuid = sqlx::query(MIGRATION_ENABLE_UUID_PLUGIN).execute(pool);

    match try_join!(enable_crypto, enable_uuid) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to enable plugins. Error: {}", e);
            Err(e)
        }
    }
}

pub fn generate_connection_string(
    url: &str,
    port: u32,
    user_name: Option<&str>,
    password: Option<&str>,
    database: Option<&str>,
) -> String {
    let mut connection_string = String::from("postgres://");

    if let Some(u) = user_name {
        connection_string += u;
        if let Some(p) = password {
            connection_string += &format!(":{}", p);
        }
        connection_string.push('@');
    }

    connection_string += &format!("{}:{}", url, port);

    if let Some(d) = database {
        connection_string += "/";
        connection_string += d;
    }

    connection_string
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_connection_string_all_params() {
        let url = "localhost";
        let port = 5432;
        let user_name = Some("user");
        let password = Some("password");
        let database = Some("test_db");

        let expected = "postgres://user:password@localhost:5432/test_db";
        let actual = generate_connection_string(url, port, user_name, password, database);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_generate_connection_string_no_user() {
        let url = "localhost";
        let port = 5432;
        let user_name = None;
        let password = Some("password");
        let database = Some("test_db");

        let expected = "postgres://localhost:5432/test_db";
        let actual = generate_connection_string(url, port, user_name, password, database);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_generate_connection_string_no_password() {
        let url = "localhost";
        let port = 5432;
        let user_name = Some("user");
        let password = None;
        let database = Some("test_db");

        let expected = "postgres://user@localhost:5432/test_db";
        let actual = generate_connection_string(url, port, user_name, password, database);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_generate_connection_string_no_database() {
        let url = "localhost";
        let port = 5432;
        let user_name = Some("user");
        let password = Some("password");
        let database = None;

        let expected = "postgres://user:password@localhost:5432";
        let actual = generate_connection_string(url, port, user_name, password, database);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_generate_connection_string_only_url_and_port() {
        let url = "localhost";
        let port = 5432;
        let user_name = None;
        let password = None;
        let database = None;

        let expected = "postgres://localhost:5432";
        let actual = generate_connection_string(url, port, user_name, password, database);

        assert_eq!(expected, actual);
    }
}
