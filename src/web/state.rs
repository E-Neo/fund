use crate::db;
use sqlx::SqlitePool;
use std::sync::OnceLock;

/// The application's SQLite pool, initialized once in `main`.
pub static POOL: OnceLock<SqlitePool> = OnceLock::new();

/// Open the database and store the pool in the process-wide `POOL`.
pub async fn init(path: &str) -> Result<(), crate::error::Error> {
    let pool = db::open(path).await?;
    let _ = POOL.set(pool);
    Ok(())
}

/// Access the shared pool, panicking if it was never initialized.
pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("database pool not initialized")
}
