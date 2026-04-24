pub mod sqlite;

use std::fs;
use std::path::Path;

use serde::Deserialize;

pub use sqlite::{SqliteConnection, SqliteConnector};

pub type DatabaseRow = Vec<String>;

/// Database settings loaded from a YAML config file.
#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub driver: String,
    pub path: String,
}

/// Runtime database connection selected from config.
pub enum Database {
    Sqlite(SqliteConnection),
}

/// Common behavior expected from a database connection.
pub trait DatabaseConnection {
    fn execute(&self, sql: &str) -> Result<(), String>;
    fn query(&self, sql: &str) -> Result<Vec<DatabaseRow>, String>;
}

/// Common behavior expected from a database connector.
pub trait DatabaseConnector {
    type Connection: DatabaseConnection;

    fn open(&self, path: impl AsRef<Path>) -> Result<Self::Connection, String>;
    fn open_memory(&self) -> Result<Self::Connection, String>;
}

impl DatabaseConnection for Database {
    fn execute(&self, sql: &str) -> Result<(), String> {
        match self {
            Database::Sqlite(connection) => connection.execute(sql),
        }
    }

    fn query(&self, sql: &str) -> Result<Vec<DatabaseRow>, String> {
        match self {
            Database::Sqlite(connection) => connection.query(sql),
        }
    }
}

/// Loads database settings from YAML.
pub fn load_database_config(path: impl AsRef<Path>) -> Result<DatabaseConfig, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_yaml::from_str(&content).map_err(|err| err.to_string())
}

/// Opens the configured database connection.
pub fn connect_from_config(path: impl AsRef<Path>) -> Result<Database, String> {
    let config = load_database_config(path)?;
    connect(config)
}

/// Opens a database connection from already loaded settings.
pub fn connect(config: DatabaseConfig) -> Result<Database, String> {
    match config.driver.as_str() {
        "sqlite" => {
            ensure_parent_dir(&config.path)?;
            SqliteConnector.open(config.path).map(Database::Sqlite)
        }
        driver => Err(format!("unsupported database driver: {driver}")),
    }
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    if path == ":memory:" {
        return Ok(());
    }

    let Some(parent) = Path::new(path).parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{DatabaseConnection, connect_from_config, load_database_config};

    #[test]
    fn loads_database_config_from_yaml() {
        let path = temp_path("database_config", "yaml");
        fs::write(&path, "driver: sqlite\npath: \":memory:\"\n").expect("should write config");

        let config = load_database_config(&path).expect("should load database config");

        assert_eq!(config.driver, "sqlite");
        assert_eq!(config.path, ":memory:");
        fs::remove_file(path).expect("should remove temp config");
    }

    #[test]
    fn connects_database_from_config_file() {
        let db_path = temp_path("tiny_server_configured_sqlite", "db");
        let config_path = temp_path("database_configured", "yaml");
        fs::write(
            &config_path,
            format!("driver: sqlite\npath: {}\n", db_path.display()),
        )
        .expect("should write config");

        {
            let database = connect_from_config(&config_path).expect("should connect database");
            database
                .execute("CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
                .expect("should execute sql");
        }

        assert!(db_path.exists());
        fs::remove_file(config_path).expect("should remove temp config");
        fs::remove_file(db_path).expect("should remove temp db");
    }

    #[test]
    fn rejects_unknown_database_driver() {
        let config_path = temp_path("database_unknown", "yaml");
        fs::write(&config_path, "driver: postgres\npath: test\n").expect("should write config");

        let err = match connect_from_config(&config_path) {
            Ok(_) => panic!("driver should be rejected"),
            Err(err) => err,
        };

        assert!(err.contains("unsupported database driver"));
        fs::remove_file(config_path).expect("should remove temp config");
    }

    fn temp_path(name: &str, extension: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{unique}.{extension}"))
    }
}
