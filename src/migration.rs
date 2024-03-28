use core::fmt;
use std::{
    fs::{self, DirEntry},
    path::Path,
};

use derive_builder::Builder;
use log::{debug, info, trace};
use sqlx::{query_as, Error, PgPool};
use tokio::try_join;
use uuid::Uuid;

use crate::datasource::{execute_sql_script, Entity};

// Constants
const MIGRATION_TABLE_EXISTS_QUERY: &str =
    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'migrations')::integer";
const MIGRATION_TABLE_CREATE_QUERY: &str = "CREATE TABLE migrations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    migration_number bigint NOT NULL,
    script_name varchar(255) NOT NULL,
    success boolean DEFAULT false,
    message text
)";
const MIGRATION_ENABLE_CRYPTO_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto""#;
const MIGRATION_ENABLE_UUID_PLUGIN: &str = r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#;
const SELECT_ALL_MIGRATIONS: &str = "SELECT * FROM migrations ORDER BY migration_number ASC";

#[derive(sqlx::FromRow, Debug, Builder, Clone)]
struct Migration {
    id: Option<Uuid>,
    migration_number: i64,
    script_name: String,
    success: bool,
    message: Option<String>,
}

impl Entity for Migration {
    async fn save(&self, pool: &PgPool) -> Result<(), Error> {
        let query = match self.id {
            Some(id) => sqlx::query(
                "UPDATE migrations 
                    SET migration_number=$1, script_name=$2, success=$3, message=$4 
                    WHERE id=$5",
            )
            .bind(&self.migration_number)
            .bind(&self.script_name)
            .bind(&self.success)
            .bind(&self.message)
            .bind(id),
            None => sqlx::query(
                "INSERT INTO migrations (migration_number, script_name, success, message) 
                    VALUES($1, $2, $3, $4)",
            )
            .bind(&self.migration_number)
            .bind(&self.script_name)
            .bind(&self.success)
            .bind(&self.message),
        };

        query.execute(pool).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum MigrationError {
    Io(std::io::Error),
    Sql(Error),
    Builder(MigrationBuilderError),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Io(err) => write!(f, "IO error: {}", err),
            MigrationError::Sql(err) => write!(f, "Sql error: {}", err),
            MigrationError::Builder(err) => write!(f, "Builder error: {}", err),
        }
    }
}

impl std::error::Error for MigrationError {}

pub async fn enable_uuid_plugins(pool: &PgPool) -> Result<(), sqlx::Error> {
    let enable_crypto = sqlx::query(MIGRATION_ENABLE_CRYPTO_PLUGIN).execute(pool);
    let enable_uuid = sqlx::query(MIGRATION_ENABLE_UUID_PLUGIN).execute(pool);

    try_join!(enable_crypto, enable_uuid).map_err(|e| {
        eprintln!("Failed to enable plugins. Error: {}", e);
        e
    })?;
    debug!("DB extensions pgcrypto and uuid-ossp enabled");

    Ok(())
}

pub async fn init(pool: &PgPool) -> Result<(), Error> {
    enable_uuid_plugins(pool).await?;
    let (table_exists,): (i32,) = query_as(MIGRATION_TABLE_EXISTS_QUERY)
        .fetch_one(pool)
        .await?;

    if table_exists == 0 {
        debug!("Migration table doesn't exist. It will be created.");
        sqlx::query(MIGRATION_TABLE_CREATE_QUERY)
            .execute(pool)
            .await?;
        debug!("Migration table created");
        return Ok(());
    }
    debug!("Migration table already exists");

    Ok(())
}

pub async fn migrate(pool: &PgPool, directory: &Path) -> Result<(), MigrationError> {
    let sql_files = get_sql_files(directory)?;
    if sql_files.is_empty() {
        debug!("No migration scripts found.");
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
            trace!("Migration file {} was already applied", get_migration_script_name(&file));
            continue;
        }
        trace!("Processing migration file: {}", get_migration_script_name(&file));
        let content = fs::read_to_string(file.path()).map_err(MigrationError::Io)?;
        match execute_sql_script(pool, content).await {
            Ok(_) => {
                migration.success = true;
                info!("{} finished.", get_migration_script_name(&file));
                if let Err(error) = migration.save(pool).await {
                    debug!("{} could not be saved in DB", get_migration_script_name(&file));
                    return Err(MigrationError::Sql(error));
                }
            }
            Err(error) => {
                info!("{} failed. {:?}", get_migration_script_name(&file), error);
                migration.success = false;
                migration.message = Some(error.to_string());
                if let Err(error) = migration.save(pool).await {
                    return Err(MigrationError::Sql(error));
                }
                return Err(MigrationError::Sql(error));
            }
        }
    }
    Ok(())
}

fn get_or_create_migration(
    migrations: &[Migration],
    file: &DirEntry,
    migration_number: i64,
) -> Result<(Migration, i64), MigrationError> {
    match get_migration(migrations, file) {
        Some(found_migration) => {
            trace!("{} found in database", get_migration_script_name(file));
            Ok((found_migration.clone(), found_migration.migration_number))
        },
        None => {
            trace!("{} not found in database", get_migration_script_name(file));
            let new_migration_number = migration_number + 1;
            let migration = MigrationBuilder::default()
                .id(None)
                .script_name(get_migration_script_name(file))
                .migration_number(new_migration_number)
                .success(false)
                .message(None)
                .build()
                .map_err(MigrationError::Builder)?;
            trace!("Model for {} created", get_migration_script_name(file));
            Ok((migration, new_migration_number))
        }
    }
}

fn get_migration<'a>(migrations: &'a [Migration], file: &DirEntry) -> Option<&'a Migration> {
    let filename = get_migration_script_name(file);
    migrations
        .iter()
        .find(|migration| migration.script_name == filename)
}

fn get_migration_script_name(file: &DirEntry) -> String {
    file.path()
        .file_stem()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("")
        .to_string()
}

fn get_sql_files(directory: &Path) -> Result<Vec<DirEntry>, MigrationError> {
    let files = fs::read_dir(directory).map_err(MigrationError::Io)?;
    let mut sql_files: Vec<_> = files
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sql"))
        .collect();
    sql_files.sort_by_key(|dir| dir.path());
    debug!("Following migrations were found: {:?}", sql_files);
    Ok(sql_files)
}

async fn find_all_migrations(pool: &PgPool) -> Result<Vec<Migration>, sqlx::Error> {
    sqlx::query_as::<_, Migration>(SELECT_ALL_MIGRATIONS)
        .fetch_all(pool)
        .await
}
