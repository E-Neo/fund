use crate::world::{Bank, Market, Repository, TradingRule};

pub enum Plan {
    Buy(f64),
    Sell(f64),
    Pass,
}

pub trait Strategy<TR: TradingRule> {
    fn make_decision(&mut self, market: &Market, bank: &Bank, repository: &Repository<TR>) -> Plan;
}
