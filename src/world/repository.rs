use crate::world::Trade;
use chrono::NaiveDate;

#[derive(Debug)]
pub enum RepositoryError {
    BuyError,
    SellError,
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RepositoryError::BuyError => write!(f, "BuyError"),
            RepositoryError::SellError => write!(f, "SellError"),
        }
    }
}

impl std::error::Error for RepositoryError {}

pub trait TradingRule {
    fn buy(&mut self, date: NaiveDate, nav: f64, cost: f64) -> Result<Trade, RepositoryError>;
    fn sell(&mut self, date: NaiveDate, nav: f64, share: f64) -> Result<Trade, RepositoryError>;
}

pub struct Repository<TR: TradingRule> {
    code: String,
    trading_rule: TR,
    trading_history: Vec<Trade>,
    uncomfirmed_trade: Option<Trade>,
    holding_share: f64,
    invested_money: f64,
}

impl<TR: TradingRule> Repository<TR> {
    pub fn new(code: &str, trading_rule: TR) -> Self {
        Repository {
            code: String::from(code),
            trading_rule,
            trading_history: vec![],
            uncomfirmed_trade: None,
            holding_share: 0.,
            invested_money: 0.,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn trading_history(&self) -> &[Trade] {
        &self.trading_history
    }

    pub fn uncomfirmed_trade(&self) -> &Option<Trade> {
        &self.uncomfirmed_trade
    }

    pub fn holding_share(&self) -> f64 {
        self.holding_share
    }

    pub fn invested_money(&self) -> f64 {
        self.invested_money
    }

    pub fn pass(&mut self) -> Result<(), RepositoryError> {
        self.update(None)
    }

    pub fn buy(&mut self, date: NaiveDate, nav: f64, cost: f64) -> Result<(), RepositoryError> {
        if cost > 0. {
            let trade = self.trading_rule.buy(date, nav, cost)?;
            self.update(Some(trade))
        } else {
            Err(RepositoryError::BuyError)
        }
    }

    pub fn sell(&mut self, date: NaiveDate, nav: f64, share: f64) -> Result<(), RepositoryError> {
        if share > 0. && self.holding_share > share {
            let trade = self.trading_rule.sell(date, nav, share)?;
            self.update(Some(trade))
        } else {
            Err(RepositoryError::SellError)
        }
    }

    fn update(&mut self, trade_opt: Option<Trade>) -> Result<(), RepositoryError> {
        if let Some(trade) = std::mem::replace(&mut self.uncomfirmed_trade, trade_opt) {
            self.holding_share += trade.share();
            self.invested_money += trade.money();
            self.trading_history.push(trade);
        }
        Ok(())
    }
}
