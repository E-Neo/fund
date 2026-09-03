use crate::error::{Error, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// The name of the database file inside the home directory.
pub const DB_FILE: &str = "db.sqlite3";
/// The name of the configuration file inside the home directory.
pub const CONFIG_FILE: &str = "config.toml";

/// The application's SQLite pool, initialized once at startup.
static POOL: OnceLock<SqlitePool> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerConfig {
    pub addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "127.0.0.1:8080".to_string(),
            },
        }
    }
}

/// Resolve the database path for a home directory.
pub fn db_path(home: &Path) -> PathBuf {
    home.join(DB_FILE)
}

/// Load configuration from `<home>/config.toml`, falling back to defaults.
pub fn load_config(home: &Path) -> Result<Config> {
    let path = home.join(CONFIG_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let config: Config = toml::from_str(&text)
                .map_err(|err| Error::Parse(format!("invalid {}: {err}", path.display())))?;
            Ok(config)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(err) => Err(Error::Io(err)),
    }
}

/// Open the database for a home directory and store the pool process-wide.
pub async fn init(home: &Path) -> Result<()> {
    std::fs::create_dir_all(home)?;
    let pool = crate::db::open(&db_path(home).to_string_lossy()).await?;
    let _ = POOL.set(pool);
    Ok(())
}

/// Access the shared pool, panicking if it was never initialized.
pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("database pool not initialized")
}
