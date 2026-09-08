use chrono::NaiveDate;

#[derive(Debug, Clone, Default)]
pub struct PortfolioState {
    pub holding_price: f64,
    pub holding_share: f64,
    pub cumulative_investment: f64,
    pub cumulative_redemption: f64,
    /// Uninvested cash available to the strategy.
    pub cash: f64,
    /// Initial capital contributed by the investor.
    pub capital: f64,
}

impl PortfolioState {
    pub fn with_capital(capital: f64) -> Self {
        Self {
            cash: capital,
            capital,
            ..Self::default()
        }
    }

    pub fn invest(&mut self, investment: f64, share: f64) {
        self.holding_price =
            (self.holding_price * self.holding_share + investment) / (self.holding_share + share);
        self.holding_share += share;
        self.cumulative_investment += investment;
        self.cash -= investment;
    }

    pub fn redeem(&mut self, shares: f64, money: f64) {
        self.holding_share -= shares;
        self.cumulative_redemption += money;
        self.cash += money;
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
    pub cash: f64,
    /// Initial capital contributed by the investor.
    pub capital: f64,
}

impl DailySnapshot {
    pub fn market_value(&self) -> f64 {
        self.holding_share * self.unit_nav + self.cash
    }

    pub fn profit(&self) -> f64 {
        self.market_value() - self.capital
    }

    pub fn return_pct(&self) -> f64 {
        if self.capital == 0.0 {
            0.0
        } else {
            self.profit() / self.capital * 100.0
        }
    }
}
