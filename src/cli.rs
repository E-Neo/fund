use clap::{Parser, Subcommand};
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Clone)]
pub enum StrategyArg {
    Bundled(String),
    File(PathBuf),
}

impl FromStr for StrategyArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with(".toml") {
            Ok(StrategyArg::File(PathBuf::from(s)))
        } else {
            Ok(StrategyArg::Bundled(s.to_string()))
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "fund", about = "Event-driven fund trading backtester")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Fetch real daily NAV history from Eastmoney and store it in SQLite.
    Fetch {
        /// Fund code, e.g. 110022.
        code: String,
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
    },
    /// Run an event-driven backtest over stored history.
    Backtest {
        /// Fund code, e.g. 110022.
        code: String,
        /// Strategy name or path to a strategy.toml file.
        #[arg(long, value_parser = clap::value_parser!(StrategyArg), default_value = "buy_hold")]
        strategy: StrategyArg,
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
        /// Ignore stored fee rules and backtest with zero fees.
        #[arg(long)]
        no_rules: bool,
        /// Start date (inclusive), ISO yyyy-mm-dd.
        #[arg(long)]
        from: Option<String>,
        /// End date (inclusive), ISO yyyy-mm-dd.
        #[arg(long)]
        to: Option<String>,
        /// Initial one-time investment.
        #[arg(long, default_value_t = 1000.0)]
        initial: f64,
        /// Fixed amount invested by the dca strategy on each buy.
        #[arg(long, default_value_t = 100.0)]
        dca_amount: f64,
        /// Days between buys for the dca strategy.
        #[arg(long, default_value_t = 7)]
        dca_interval: u64,
    },
    /// Fetch daily NAV rows newer than the last stored date (incremental).
    Update {
        /// Fund code, e.g. 110022.
        code: String,
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
    },
    /// List funds cached in the database.
    List {
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
    },
    /// List available strategies.
    Strategies,
}
