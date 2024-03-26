use core::fmt;
use std::{
    fs::{self, DirEntry},
    path::Path,
};

use derive_builder::Builder;
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
static SELECT_ALL_MIGRATIONS: &str = r#"SELECT * FROM migrations ORDER BY migration_number ASC"#;
static MIGRATION_ENABLE_CRYPTO_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto""#;
static MIGRATION_ENABLE_UUID_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#;

#[derive(sqlx::FromRow, Debug, Builder, Clone)]
struct Migration {
    id: Option<Uuid>,
    migration_number: i64,
    script_name: String,
    success: bool,
    message: Option<String>,
}

pub trait DbModel {
    async fn save(&self, pool: &PgPool) -> Result<(), Error>;
}

impl DbModel for Migration {
    async fn save(&self, pool: &PgPool) -> Result<(), Error> {
        if self.id.is_none() {
            sqlx::query(
                "INSERT INTO migrations (migration_number, script_name, success, message) 
            VALUES($1, $2, $3, $4)",
            )
            .bind(&self.migration_number)
            .bind(&self.script_name)
            .bind(&self.success)
            .bind(&self.message)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE migrations 
            SET migration_number=$1, script_name=$2, success=$3, message=$4 
            WHERE id=$5",
            )
            .bind(&self.migration_number)
            .bind(&self.script_name)
            .bind(&self.success)
            .bind(&self.message)
            .bind(&self.id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum MigrationError {
    Io(std::io::Error),
    Sql(Error),
    Builder(MigrationBuilderError),
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Io(err) => write!(f, "IO error: {}", err),
            MigrationError::Sql(err) => write!(f, "Sql error: {}", err),
            MigrationError::Builder(err) => write!(f, "Builder error: {}", err),
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrationError::Io(err) => Some(err),
            MigrationError::Sql(err) => Some(err),
            MigrationError::Builder(err) => Some(err),
        }
    }
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

pub async fn migrate(pool: &PgPool, directory: &Path) -> Result<(), MigrationError> {
    let files = fs::read_dir(directory).map_err(MigrationError::Io)?;
    let mut sql_files: Vec<_> = files
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sql"))
        .collect();
    sql_files.sort_by_key(|dir| dir.path());
    if sql_files.is_empty() {
        return Ok(());
    }

    let migrations = find_all_migrations(pool)
        .await
        .map_err(MigrationError::Sql)?;
    let mut migration_number: i64 = 0;
    for file in sql_files {
        let (mut migration, current_migration_number) =
            get_or_create_migration(&migrations, &file, migration_number)?;
        migration_number = current_migration_number;
        if migration.success {
            continue;
        }
        let content = fs::read_to_string(file.path()).map_err(MigrationError::Io)?;
        match execute_sql_script(pool, content).await {
            Ok(_) => {
                migration.success = true;
                match migration.save(pool).await {
                    Ok(_) => {}
                    Err(error) => return Err(MigrationError::Sql(error)),
                }
            }
            Err(error) => {
                migration.success = false;
                migration.message = Some(error.to_string());
                match migration.save(pool).await {
                    Ok(_) => {}
                    Err(error) => return Err(MigrationError::Sql(error)),
                }
                return Err(MigrationError::Sql(error));
            }
        }
    }
    Ok(())
}

fn get_or_create_migration(
    migrations: &Vec<Migration>,
    file: &DirEntry,
    migration_number: i64,
) -> Result<(Migration, i64), MigrationError> {
    match get_migration(&migrations, &file) {
        Ok(found_migration) => {
            return Ok((found_migration.clone(), found_migration.migration_number));
        }
        Err(_) => {
            let new_migration_number = migration_number + 1;
            let migration = MigrationBuilder::default()
                .id(None)
                .script_name(get_migration_script_name(&file))
                .migration_number(new_migration_number)
                .success(false)
                .message(None)
                .build()
                .map_err(MigrationError::Builder)
                .unwrap();
            return Ok((migration, new_migration_number));
        }
    };
}

fn get_migration(migrations: &Vec<Migration>, file: &DirEntry) -> Result<Migration, ()> {
    let filename = get_migration_script_name(file);
    return migrations
        .iter()
        .find(|migration| migration.script_name == filename)
        .cloned()
        .ok_or_else(|| ());
}

fn get_migration_script_name(file: &DirEntry) -> String {
    let path = file.path();
    return path
        .file_stem()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("")
        .to_string();
}

async fn execute_sql_script(pool: &PgPool, content: String) -> Result<(), Error> {
    let mut transaction = pool.begin().await?;
    let statements: Vec<&str> = content.split(';').collect();
    for mut statement in statements {
        statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        let rows = sqlx::query(statement).execute(&mut *transaction).await?;
        println!("Executed statement, {} rows affected", rows.rows_affected());
    }
    transaction.commit().await?;
    Ok(())
}

async fn find_all_migrations(pool: &PgPool) -> Result<Vec<Migration>, sqlx::Error> {
    let migrations: Vec<Migration> = sqlx::query_as::<_, Migration>(SELECT_ALL_MIGRATIONS)
        .fetch_all(pool)
        .await?;
    Ok(migrations)
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
