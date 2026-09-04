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
pub struct Config {
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

impl ServerConfig {
    /// The socket address the server should bind to.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

/// Resolve the database path for a home directory.
pub fn db_path(home: &Path) -> PathBuf {
    home.join(DB_FILE)
}

/// Load configuration from `<home>/config.toml`. The file is required; a
/// missing file is an error, not a fallback.
pub fn load_config(home: &Path) -> Result<Config> {
    let path = home.join(CONFIG_FILE);
    let text = std::fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "config file not found: {} (expected {} under --home)",
                    path.display(),
                    CONFIG_FILE
                ),
            ))
        } else {
            crate::error::Error::Io(err)
        }
    })?;
    toml::from_str(&text).map_err(|err| Error::Parse(format!("invalid {}: {err}", path.display())))
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
