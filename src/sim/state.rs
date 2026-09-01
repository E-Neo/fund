use chrono::NaiveDate;

#[derive(Debug, Clone, Default)]
pub struct PortfolioState {
    pub holding_price: f64,
    pub holding_share: f64,
    pub cumulative_investment: f64,
    pub cumulative_redemption: f64,
}

impl PortfolioState {
    pub fn invest(&mut self, investment: f64, share: f64) {
        self.holding_price =
            (self.holding_price * self.holding_share + investment) / (self.holding_share + share);
        self.holding_share += share;
        self.cumulative_investment += investment;
    }

    pub fn redeem(&mut self, shares: f64, money: f64) {
        self.holding_share -= shares;
        self.cumulative_redemption += money;
    }
}

#[derive(Debug, Clone)]
pub struct DailySnapshot {
    pub date: NaiveDate,
    pub unit_nav: f64,
    pub holding_price: f64,
    pub holding_share: f64,
    pub cumulative_investment: f64,
    pub cumulative_redemption: f64,
}

impl DailySnapshot {
    pub fn market_value(&self) -> f64 {
        self.holding_share * self.unit_nav + self.cumulative_redemption
    }

    pub fn profit(&self) -> f64 {
        self.market_value() - self.cumulative_investment
    }

    pub fn return_pct(&self) -> f64 {
        if self.cumulative_investment == 0.0 {
            0.0
        } else {
            self.profit() / self.cumulative_investment * 100.0
        }
    }
}
