use crate::sim::{engine::SimulationResult, state::DailySnapshot};
use chrono::NaiveDate;
use std::fmt;

pub struct Report {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub days: usize,
    pub transactions: usize,
    pub total_invested: f64,
    pub total_redeemed: f64,
    pub final_holding_share: f64,
    pub final_market_value: f64,
    pub profit: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
}

pub fn build(start: NaiveDate, end: NaiveDate, days: usize, result: &SimulationResult) -> Report {
    let snapshots = &result.snapshots;
    let last = snapshots.last().expect("at least one snapshot");
    let final_market_value = last.market_value();
    let total_invested = last.cumulative_investment;
    let total_redeemed = last.cumulative_redemption;
    let profit = last.profit();
    let total_return_pct = last.return_pct();
    let max_drawdown_pct = max_drawdown_pct(snapshots);

    Report {
        start,
        end,
        days,
        transactions: result.transactions.len(),
        total_invested,
        total_redeemed,
        final_holding_share: last.holding_share,
        final_market_value,
        profit,
        total_return_pct,
        max_drawdown_pct,
    }
}

fn max_drawdown_pct(snapshots: &[DailySnapshot]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut max_dd: f64 = 0.0;
    for snapshot in snapshots {
        let value = snapshot.market_value();
        peak = peak.max(value);
        if peak > 0.0 {
            let drawdown = (peak - value) / peak;
            max_dd = max_dd.max(drawdown);
        }
    }
    max_dd * 100.0
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Period: {} to {} ({} trading days)",
            self.start, self.end, self.days
        )?;
        writeln!(f, "Transactions: {}", self.transactions)?;
        writeln!(f, "Total invested: {:.2}", self.total_invested)?;
        writeln!(f, "Total redeemed: {:.2}", self.total_redeemed)?;
        writeln!(f, "Final holding share: {:.4}", self.final_holding_share)?;
        writeln!(f, "Final market value: {:.2}", self.final_market_value)?;
        writeln!(f, "Profit: {:.2}", self.profit)?;
        writeln!(f, "Total return: {:.2}%", self.total_return_pct)?;
        writeln!(f, "Max drawdown: {:.2}%", self.max_drawdown_pct)?;
        Ok(())
    }
}
