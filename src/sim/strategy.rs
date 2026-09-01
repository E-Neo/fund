use crate::{
    error::{Error, Result},
    sim::{
        event::{Event, Order, Transaction},
        state::PortfolioState,
    },
};
use chrono::NaiveDate;

pub struct SimContext<'a> {
    pub date: NaiveDate,
    pub unit_nav: f64,
    pub accum_nav: f64,
    pub state: &'a PortfolioState,
    pub transactions: &'a [Transaction],
}

pub trait Strategy {
    fn name(&self) -> &str;
    fn on_event(&mut self, event: &Event, ctx: &mut SimContext) -> Vec<Order>;
}

pub struct BuyHold {
    amount: f64,
    invested: bool,
}

impl Strategy for BuyHold {
    fn name(&self) -> &str {
        "buy_hold"
    }

    fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
        if !self.invested {
            if let Event::NavUpdate { .. } = event {
                self.invested = true;
                return vec![Order::Invest {
                    amount: self.amount,
                }];
            }
        }
        Vec::new()
    }
}

pub struct Dca {
    amount: f64,
    interval: u64,
    day: u64,
}

impl Strategy for Dca {
    fn name(&self) -> &str {
        "dca"
    }

    fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
        if let Event::NavUpdate { .. } = event {
            let buy = self.day == 0 || self.day.is_multiple_of(self.interval);
            self.day += 1;
            if buy {
                return vec![Order::Invest {
                    amount: self.amount,
                }];
            }
        }
        Vec::new()
    }
}

pub fn create(
    name: &str,
    initial: f64,
    dca_amount: f64,
    dca_interval: u64,
) -> Result<Box<dyn Strategy>> {
    match name {
        "buy_hold" => Ok(Box::new(BuyHold {
            amount: initial,
            invested: false,
        })),
        "dca" => Ok(Box::new(Dca {
            amount: dca_amount,
            interval: dca_interval,
            day: 0,
        })),
        other => Err(Error::UnknownStrategy(other.to_string())),
    }
}

pub fn names() -> [&'static str; 2] {
    ["buy_hold", "dca"]
}
