use crate::{
    eastmoney::{Fund, Nav},
    error::Result,
    fees::FeeRule,
    rules::{FeeKind, Tier},
};
use chrono::NaiveDate;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
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
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS fund_rules (
            fund_code TEXT NOT NULL REFERENCES funds(code),
            rule_type TEXT NOT NULL,
            lower_bound REAL NOT NULL,
            rate REAL NOT NULL,
            fee_kind TEXT NOT NULL,
            PRIMARY KEY (fund_code, rule_type, lower_bound)
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

pub async fn max_nav_date(pool: &SqlitePool, code: &str) -> Result<Option<NaiveDate>> {
    let date: Option<NaiveDate> =
        sqlx::query_scalar("SELECT MAX(date) FROM daily_nav WHERE fund_code = ?")
            .bind(code)
            .fetch_one(pool)
            .await?;
    Ok(date)
}

pub async fn upsert_rules(pool: &SqlitePool, code: &str, rule: &FeeRule) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM fund_rules WHERE fund_code = ?")
        .bind(code)
        .execute(&mut *tx)
        .await?;
    for tier in &rule.subscribe {
        insert_tier(&mut tx, code, "subscribe", tier).await?;
    }
    for tier in &rule.redeem {
        insert_tier(&mut tx, code, "redeem", tier).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn insert_tier(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    code: &str,
    rule_type: &str,
    tier: &Tier,
) -> Result<()> {
    let fee_kind = match tier.kind {
        FeeKind::Pct => "pct",
        FeeKind::Fixed => "fixed",
    };
    sqlx::query(
        "INSERT INTO fund_rules (fund_code, rule_type, lower_bound, rate, fee_kind)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(code)
    .bind(rule_type)
    .bind(tier.lower_bound)
    .bind(tier.rate)
    .bind(fee_kind)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn load_rules(pool: &SqlitePool, code: &str) -> Result<Option<FeeRule>> {
    let rows: Vec<RuleRow> = sqlx::query_as(
        "SELECT rule_type, lower_bound, rate, fee_kind
         FROM fund_rules
         WHERE fund_code = ?
         ORDER BY rule_type, lower_bound",
    )
    .bind(code)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut rule = FeeRule::default();
    for row in rows {
        let kind = match row.fee_kind.as_str() {
            "fixed" => FeeKind::Fixed,
            _ => FeeKind::Pct,
        };
        let tier = Tier {
            lower_bound: row.lower_bound,
            kind,
            rate: row.rate,
        };
        match row.rule_type.as_str() {
            "subscribe" => rule.subscribe.push(tier),
            _ => rule.redeem.push(tier),
        }
    }
    Ok(Some(rule))
}

pub async fn list_funds(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    let rows: Vec<FundRow> = sqlx::query_as("SELECT code, name FROM funds ORDER BY code")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| (r.code, r.name)).collect())
}

pub async fn fund_name(pool: &SqlitePool, code: &str) -> Result<Option<String>> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM funds WHERE code = ?")
        .bind(code)
        .fetch_one(pool)
        .await?;
    Ok(name)
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

#[derive(sqlx::FromRow)]
struct RuleRow {
    rule_type: String,
    lower_bound: f64,
    rate: f64,
    fee_kind: String,
}
