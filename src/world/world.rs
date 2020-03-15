use crate::strategy::Plan;
use crate::world::{Bank, Market, Repository, TradingRule};

pub struct World<TR: TradingRule> {
    bank: Bank,
    market: Market,
    repository: Repository<TR>,
}

impl<TR: TradingRule> World<TR> {
    pub fn new(bank: Bank, market: Market, repository: Repository<TR>) -> Self {
        World {
            bank,
            market,
            repository,
        }
    }

    pub fn bank(&self) -> &Bank {
        &self.bank
    }

    pub fn market(&self) -> &Market {
        &self.market
    }

    pub fn repository(&self) -> &Repository<TR> {
        &self.repository
    }

    pub fn update(&mut self, plan: Plan) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some((date, nav)) = self.market.next() {
            match plan {
                Plan::Buy(cost) => {
                    self.bank.withdraw(cost)?;
                    self.repository.buy(date, nav, cost)?;
                }
                Plan::Sell(share) => {
                    self.repository.sell(date, nav, share)?;
                    self.bank.deposit(nav * share)?;
                }
                Plan::Pass => {
                    self.repository.pass()?;
                    self.bank.pass()?;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
