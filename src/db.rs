use crate::{
    eastmoney::{Fund, Nav},
    error::Result,
};
use chrono::NaiveDate;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

pub async fn open(path: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    create_schema(&pool).await?;
    Ok(pool)
}

async fn create_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS funds (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS daily_nav (
            fund_code TEXT NOT NULL REFERENCES funds(code),
            date TEXT NOT NULL,
            unit_nav REAL NOT NULL,
            accum_nav REAL NOT NULL,
            daily_return REAL,
            PRIMARY KEY (fund_code, date)
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_fund(pool: &SqlitePool, fund: &Fund) -> Result<()> {
    sqlx::query(
        "INSERT INTO funds (code, name) VALUES (?, ?)
         ON CONFLICT(code) DO UPDATE SET name = excluded.name",
    )
    .bind(&fund.code)
    .bind(&fund.name)
    .execute(pool)
    .await?;

    let mut tx = pool.begin().await?;
    for nav in &fund.navs {
        sqlx::query(
            "INSERT INTO daily_nav (fund_code, date, unit_nav, accum_nav, daily_return)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(fund_code, date) DO UPDATE SET
                 unit_nav = excluded.unit_nav,
                 accum_nav = excluded.accum_nav,
                 daily_return = excluded.daily_return",
        )
        .bind(&fund.code)
        .bind(nav.date)
        .bind(nav.unit_nav)
        .bind(nav.accum_nav)
        .bind(nav.daily_return)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn load_navs(pool: &SqlitePool, code: &str) -> Result<Vec<Nav>> {
    let rows: Vec<NavRow> = sqlx::query_as(
        "SELECT date, unit_nav, accum_nav, daily_return
         FROM daily_nav
         WHERE fund_code = ?
         ORDER BY date",
    )
    .bind(code)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Nav {
            date: r.date,
            unit_nav: r.unit_nav,
            accum_nav: r.accum_nav,
            daily_return: r.daily_return,
        })
        .collect())
}

pub async fn list_funds(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    let rows: Vec<FundRow> = sqlx::query_as("SELECT code, name FROM funds ORDER BY code")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| (r.code, r.name)).collect())
}

#[derive(sqlx::FromRow)]
struct NavRow {
    date: NaiveDate,
    unit_nav: f64,
    accum_nav: f64,
    daily_return: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct FundRow {
    code: String,
    name: String,
}
