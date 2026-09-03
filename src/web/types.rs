use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundInfo {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavPoint {
    pub date: String,
    pub unit_nav: f64,
    pub accum_nav: f64,
    pub daily_return: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTier {
    pub rule_type: String,
    pub lower_bound: f64,
    pub rate: f64,
    pub fee_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestInput {
    pub code: String,
    pub strategy: String,
    pub initial: f64,
    pub dca_amount: f64,
    pub dca_interval: u64,
    pub from: Option<String>,
    pub to: Option<String>,
    pub no_rules: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub date: String,
    pub market_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestReport {
    pub start: String,
    pub end: String,
    pub days: usize,
    pub transactions: usize,
    pub total_invested: f64,
    pub total_redeemed: f64,
    pub final_holding_share: f64,
    pub final_market_value: f64,
    pub profit: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub curve: Vec<CurvePoint>,
}
