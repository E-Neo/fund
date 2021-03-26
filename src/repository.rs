use crate::error::{Error, Result};
use chrono::NaiveDate;

#[derive(Debug)]
pub enum Order {
    Investment {
        date: NaiveDate,
        net_asset_value: f64,
        investment: f64,
    },
    Redemption {
        date: NaiveDate,
        net_asset_value: f64,
        redemption: f64,
    },
}

#[derive(Debug, PartialEq)]
pub enum Transaction {
    Investment {
        date: NaiveDate,
        net_asset_value: f64,
        investment: f64,
        share: f64,
        fee: f64,
    },
    Redemption {
        date: NaiveDate,
        net_asset_value: f64,
        redemption: f64,
        money: f64,
        fee: f64,
    },
}

pub trait Rule {
    fn fee(&mut self, order: Order) -> f64;
}

impl std::fmt::Debug for dyn Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InvestmentRule {:?}", self as *const _)
    }
}

impl<F> Rule for F
where
    F: Fn(Order) -> f64,
{
    fn fee(&mut self, order: Order) -> f64 {
        self(order)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyInfo {
    transaction_id: usize,
    holding_price: f64,
    holding_share: f64,
    cumulative_investment: f64,
    cumulative_redemption: f64,
}

impl DailyInfo {
    pub fn transaction_id(&self) -> Option<usize> {
        if self.transaction_id == 0 {
            None
        } else {
            Some(self.transaction_id - 1)
        }
    }

    pub fn holding_price(&self) -> f64 {
        self.holding_price
    }

    pub fn holding_share(&self) -> f64 {
        self.holding_share
    }

    pub fn cumulative_investment(&self) -> f64 {
        self.cumulative_investment
    }

    pub fn cumulative_redemption(&self) -> f64 {
        self.cumulative_redemption
    }

    pub fn cumulative_income(&self) -> f64 {
        self.cumulative_redemption - self.cumulative_investment
    }
}

#[derive(Debug)]
pub struct Repository {
    rule: Box<dyn Rule>,
    net_asset_value_history: Vec<(NaiveDate, f64)>,
    transactions: Vec<Transaction>,
    daily_infos: Vec<DailyInfo>,
}

impl Repository {
    pub fn new(
        rule: Box<dyn Rule>,
        net_asset_value_history: Vec<(NaiveDate, f64)>,
    ) -> Result<Self> {
        if net_asset_value_history.len() >= 1 {
            Ok(Repository {
                rule,
                net_asset_value_history,
                transactions: vec![],
                daily_infos: vec![DailyInfo {
                    transaction_id: 0,
                    holding_price: 0.0,
                    holding_share: 0.0,
                    cumulative_investment: 0.0,
                    cumulative_redemption: 0.0,
                }],
            })
        } else {
            Err(Error::Insufficient)
        }
    }

    pub fn len(&self) -> usize {
        self.net_asset_value_history.len()
    }

    pub fn daily_infos(&self) -> &[DailyInfo] {
        &self.daily_infos[1..]
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn check(&self) -> Result<(NaiveDate, f64)> {
        self.net_asset_value_history
            .get(self.daily_infos().len())
            .map(|&x| x)
            .ok_or(Error::Overflow)
    }

    pub fn pass(&mut self) -> Result<()> {
        if self.len() == self.daily_infos().len() {
            Err(Error::Overflow)
        } else {
            let mut info = self.daily_infos.last().unwrap().clone();
            info.transaction_id = 0;
            self.daily_infos.push(info);
            Ok(())
        }
    }

    pub fn invest(&mut self, investment: f64) -> Result<()> {
        if self.len() == self.daily_infos().len() {
            Err(Error::Overflow)
        } else {
            let &(date, net_asset_value) = self
                .net_asset_value_history
                .get(self.daily_infos.len() - 1)
                .unwrap();
            let fee = self.rule.fee(Order::Investment {
                date,
                net_asset_value,
                investment,
            });
            let share = (investment - fee) / net_asset_value;
            self.transactions.push(Transaction::Investment {
                date,
                net_asset_value,
                investment,
                share,
                fee,
            });
            let mut info = self.daily_infos.last().unwrap().clone();
            info.transaction_id = self.transactions.len();
            info.holding_price = (info.holding_price * info.holding_share + investment)
                / (info.holding_share + share);
            info.holding_share += share;
            info.cumulative_investment += investment;
            self.daily_infos.push(info);
            Ok(())
        }
    }

    pub fn redeem(&mut self, redemption: f64) -> Result<()> {
        if self.len() == self.daily_infos().len() {
            Err(Error::Overflow)
        } else if self.daily_infos.last().unwrap().holding_share < redemption {
            Err(Error::Insufficient)
        } else {
            let &(date, net_asset_value) = self
                .net_asset_value_history
                .get(self.daily_infos.len() - 1)
                .unwrap();
            let fee = self.rule.fee(Order::Redemption {
                date,
                net_asset_value,
                redemption,
            });
            let money = net_asset_value * redemption - fee;
            self.transactions.push(Transaction::Redemption {
                date,
                net_asset_value,
                redemption,
                money,
                fee,
            });
            let mut info = self.daily_infos.last().unwrap().clone();
            info.transaction_id = self.transactions.len();
            info.holding_share -= redemption;
            info.cumulative_redemption += money;
            self.daily_infos.push(info);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_fee() {
        let mut repo = Repository::new(
            Box::new(|_| 0.0),
            NaiveDate::from_ymd(2021, 1, 1)
                .iter_days()
                .enumerate()
                .take(5)
                .map(|(i, date)| (date, if i & 1 == 0 { 1.0 } else { 1.05 }))
                .collect(),
        )
        .unwrap();
        assert_eq!(repo.len(), 5);
        assert_eq!(
            repo.check().unwrap(),
            (NaiveDate::from_ymd(2021, 1, 1), 1.0)
        );
        assert!(repo.invest(100.0).is_ok());
        assert!(if let Err(Error::Insufficient) = repo.redeem(101.0) {
            true
        } else {
            false
        });
        assert_eq!(repo.daily_infos().last().unwrap().holding_share(), 100.0);
        assert_eq!(
            repo.check().unwrap(),
            (NaiveDate::from_ymd(2021, 1, 2), 1.05)
        );
        assert!(repo.redeem(50.0).is_ok());
        assert_eq!(
            repo.check().unwrap(),
            (NaiveDate::from_ymd(2021, 1, 3), 1.0)
        );
        assert!(repo.pass().is_ok());
        assert_eq!(
            repo.check().unwrap(),
            (NaiveDate::from_ymd(2021, 1, 4), 1.05)
        );
        assert!(repo.invest(50.0).is_ok());
        assert_eq!(
            repo.check().unwrap(),
            (NaiveDate::from_ymd(2021, 1, 5), 1.0)
        );
        assert!(repo.invest(100.0).is_ok());
        assert!(if let Err(Error::Overflow) = repo.check() {
            true
        } else {
            false
        });
        assert_eq!(
            repo.transactions(),
            &[
                Transaction::Investment {
                    date: NaiveDate::from_ymd(2021, 1, 1),
                    net_asset_value: 1.0,
                    investment: 100.0,
                    share: 100.0,
                    fee: 0.0
                },
                Transaction::Redemption {
                    date: NaiveDate::from_ymd(2021, 1, 2),
                    net_asset_value: 1.05,
                    redemption: 50.0,
                    money: 52.5,
                    fee: 0.0
                },
                Transaction::Investment {
                    date: NaiveDate::from_ymd(2021, 1, 4),
                    net_asset_value: 1.05,
                    investment: 50.0,
                    share: 50.0 / 1.05,
                    fee: 0.0
                },
                Transaction::Investment {
                    date: NaiveDate::from_ymd(2021, 1, 5),
                    net_asset_value: 1.0,
                    investment: 100.0,
                    share: 100.0,
                    fee: 0.0
                }
            ]
        );
        assert_eq!(
            repo.daily_infos(),
            &[
                DailyInfo {
                    transaction_id: 1,
                    holding_price: 1.0,
                    holding_share: 100.0,
                    cumulative_investment: 100.0,
                    cumulative_redemption: 0.0
                },
                DailyInfo {
                    transaction_id: 2,
                    holding_price: 1.0,
                    holding_share: 50.0,
                    cumulative_investment: 100.0,
                    cumulative_redemption: 52.5
                },
                DailyInfo {
                    transaction_id: 0,
                    holding_price: 1.0,
                    holding_share: 50.0,
                    cumulative_investment: 100.0,
                    cumulative_redemption: 52.5
                },
                DailyInfo {
                    transaction_id: 3,
                    holding_price: 100.0 / (50.0 + 50.0 / 1.05),
                    holding_share: 50.0 + 50.0 / 1.05,
                    cumulative_investment: 150.0,
                    cumulative_redemption: 52.5
                },
                DailyInfo {
                    transaction_id: 4,
                    holding_price: 200.0 / (150.0 + 50.0 / 1.05),
                    holding_share: 150.0 + 50.0 / 1.05,
                    cumulative_investment: 250.0,
                    cumulative_redemption: 52.5
                }
            ]
        );
    }
}
