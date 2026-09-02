use chrono::{Duration, Local, NaiveDate};
use clap::Parser;
use fund::{
    cli::{Cli, Command},
    db, eastmoney,
    error::{Error, Result},
    fees::FeeRule,
    report, rules, sim,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Fetch { code, db } => {
            let pool = db::open(&db).await?;
            let fund = eastmoney::fetch_fund(&code).await?;
            db::upsert_fund(&pool, &fund).await?;
            let fee_rule = eastmoney::fetch_fees(&code).await?;
            db::upsert_rules(&pool, &code, &fee_rule).await?;
            println!(
                "fetched {} ({} rows, {} subscribe / {} redeem fee tiers)",
                fund.name,
                fund.navs.len(),
                fee_rule.subscribe.len(),
                fee_rule.redeem.len(),
            );
        }
        Command::Update { code, db } => {
            let pool = db::open(&db).await?;
            match db::max_nav_date(&pool, &code).await? {
                None => {
                    let fund = eastmoney::fetch_fund(&code).await?;
                    db::upsert_fund(&pool, &fund).await?;
                    println!("fetched {} ({} rows)", fund.name, fund.navs.len());
                }
                Some(last) => {
                    let name = db::fund_name(&pool, &code)
                        .await?
                        .unwrap_or_else(|| "unknown".to_string());
                    let navs = eastmoney::fetch_nav_range(&code, last + Duration::days(1), today())
                        .await?;
                    if navs.is_empty() {
                        println!("{} is already up to date", code);
                    } else {
                        let fund = eastmoney::Fund { code, name, navs };
                        db::upsert_fund(&pool, &fund).await?;
                        println!("updated {} rows", fund.navs.len());
                    }
                }
            }
        }
        Command::Backtest {
            code,
            strategy,
            db,
            no_rules,
            from,
            to,
            initial,
            dca_amount,
            dca_interval,
        } => {
            let pool = db::open(&db).await?;
            let navs = db::load_navs(&pool, &code).await?;
            let from = from
                .as_deref()
                .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .transpose()?;
            let to = to
                .as_deref()
                .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .transpose()?;
            let navs: Vec<_> = navs
                .into_iter()
                .filter(|nav| {
                    from.is_none_or(|f| nav.date >= f) && to.is_none_or(|t| nav.date <= t)
                })
                .collect();
            if navs.is_empty() {
                return Err(Error::NoData(code));
            }
            let start = navs.first().expect("non-empty").date;
            let end = navs.last().expect("non-empty").date;
            let days = navs.len();

            let stored = if no_rules {
                None
            } else {
                db::load_rules(&pool, &code).await?
            };
            let fee_rule = stored.unwrap_or_else(|| FeeRule {
                subscribe: vec![],
                redeem: vec![],
            });
            let mut fee_rule = rules::Fifo::new(fee_rule.subscribe, fee_rule.redeem);
            let mut strategy = sim::strategy::load(&strategy, initial, dca_amount, dca_interval)?;
            let result = sim::engine::simulate(&navs, &mut fee_rule, strategy.as_mut())?;
            let report = report::build(start, end, days, &result);
            println!("{}", report);
        }
        Command::List { db } => {
            let pool = db::open(&db).await?;
            for (code, name) in db::list_funds(&pool).await? {
                println!("{code}  {name}");
            }
        }
        Command::Strategies => {
            for name in sim::strategy::names() {
                println!("{name}");
            }
        }
    }
    Ok(())
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}
