use sqlx::{Error, PgPool};

pub trait Entity {
    async fn save(&self, pool: &PgPool) -> Result<(), Error>;
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

pub async fn execute_sql_script(pool: &PgPool, content: String) -> Result<(), Error> {
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
