use chrono::NaiveDate;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum OrderForFee {
    Invest {
        date: NaiveDate,
        unit_nav: f64,
        amount: f64,
    },
    Redeem {
        date: NaiveDate,
        unit_nav: f64,
        shares: f64,
    },
}

pub trait Rule {
    fn fee(&mut self, order: OrderForFee) -> f64;
}

pub struct Fifo {
    lots: VecDeque<(NaiveDate, f64)>,
    investment_rates: Vec<(f64, f64)>,
    redemption_rates: Vec<(u64, f64)>,
}

impl Fifo {
    pub fn new(investment_rates: Vec<(f64, f64)>, redemption_rates: Vec<(u64, f64)>) -> Self {
        Self {
            lots: VecDeque::new(),
            investment_rates,
            redemption_rates,
        }
    }
}

impl Rule for Fifo {
    fn fee(&mut self, order: OrderForFee) -> f64 {
        match order {
            OrderForFee::Invest {
                date,
                unit_nav,
                amount,
            } => {
                let fee = self
                    .investment_rates
                    .iter()
                    .find(|&&(bound, _)| amount < bound)
                    .map_or(0.0, |&(_, rate)| rate * amount);
                self.lots.push_back((date, (amount - fee) / unit_nav));
                fee
            }
            OrderForFee::Redeem {
                date,
                unit_nav,
                mut shares,
            } => {
                let mut fee = 0.0;
                while let Some((invest_date, share)) = self.lots.pop_front() {
                    if shares < share {
                        self.lots.push_front((invest_date, share - shares));
                        fee += redemption_fee(
                            &self.redemption_rates,
                            invest_date,
                            date,
                            unit_nav,
                            shares,
                        );
                        break;
                    } else {
                        shares -= share;
                        fee += redemption_fee(
                            &self.redemption_rates,
                            invest_date,
                            date,
                            unit_nav,
                            share,
                        );
                    }
                }
                fee
            }
        }
    }
}

fn redemption_fee(
    redemption_rates: &[(u64, f64)],
    invest_date: NaiveDate,
    date: NaiveDate,
    unit_nav: f64,
    share: f64,
) -> f64 {
    share
        * unit_nav
        * redemption_rates
            .iter()
            .find(|&&(bound, _)| (date - invest_date).num_days() < bound as i64)
            .map_or(0.0, |&(_, rate)| rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7_30() {
        let mut rule = Fifo::new(vec![], vec![(7, 0.015), (30, 0.005)]);
        assert_eq!(
            rule.fee(OrderForFee::Invest {
                date: NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                unit_nav: 1.0,
                amount: 100.0,
            }),
            0.0
        );
        assert_eq!(
            rule.fee(OrderForFee::Redeem {
                date: NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
                unit_nav: 1.0,
                shares: 10.0,
            }),
            0.05
        );
        assert_eq!(
            rule.fee(OrderForFee::Invest {
                date: NaiveDate::from_ymd_opt(2021, 2, 1).unwrap(),
                unit_nav: 1.0,
                amount: 100.0,
            }),
            0.0
        );
        assert_eq!(
            rule.fee(OrderForFee::Redeem {
                date: NaiveDate::from_ymd_opt(2021, 2, 5).unwrap(),
                unit_nav: 1.05,
                shares: 190.0,
            }),
            100.0 * 1.05 * 0.015
        );
    }
}
