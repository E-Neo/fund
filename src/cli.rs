use clap::{Parser, Subcommand};

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
        /// Strategy name.
        #[arg(long, default_value = "buy_hold")]
        strategy: String,
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
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
    /// List funds cached in the database.
    List {
        /// Path to the SQLite database file.
        #[arg(long)]
        db: String,
    },
    /// List available strategies.
    Strategies,
}
