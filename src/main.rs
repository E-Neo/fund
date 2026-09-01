use chrono::NaiveDate;
use clap::Parser;
use fund::{
    cli::{Cli, Command},
    db, eastmoney,
    error::{Error, Result},
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
            println!("fetched {} ({} rows)", fund.name, fund.navs.len());
        }
        Command::Backtest {
            code,
            strategy,
            db,
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

            let mut fee_rule = rules::Fifo::new(vec![], vec![(7, 0.015), (30, 0.005)]);
            let mut strategy = sim::strategy::create(&strategy, initial, dca_amount, dca_interval)?;
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
