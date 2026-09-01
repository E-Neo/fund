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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeKind {
    Pct,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tier {
    /// Investment: amount in yuan; redemption: holding days.
    pub lower_bound: f64,
    pub kind: FeeKind,
    /// `Pct`: percent (1.5 = 1.5%); `Fixed`: flat fee amount in yuan.
    pub rate: f64,
}

impl Tier {
    pub const fn pct(lower_bound: f64, rate: f64) -> Self {
        Self {
            lower_bound,
            kind: FeeKind::Pct,
            rate,
        }
    }

    pub const fn fixed(lower_bound: f64, amount: f64) -> Self {
        Self {
            lower_bound,
            kind: FeeKind::Fixed,
            rate: amount,
        }
    }
}

pub trait Rule {
    fn fee(&mut self, order: OrderForFee) -> f64;
}

pub struct Fifo {
    lots: VecDeque<(NaiveDate, f64)>,
    investment_tiers: Vec<Tier>,
    redemption_tiers: Vec<Tier>,
}

impl Fifo {
    pub fn new(investment_tiers: Vec<Tier>, redemption_tiers: Vec<Tier>) -> Self {
        Self {
            lots: VecDeque::new(),
            investment_tiers,
            redemption_tiers,
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
                let fee = tier_fee(&self.investment_tiers, amount, amount);
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
                            &self.redemption_tiers,
                            invest_date,
                            date,
                            unit_nav,
                            shares,
                        );
                        break;
                    } else {
                        shares -= share;
                        fee += redemption_fee(
                            &self.redemption_tiers,
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
    redemption_tiers: &[Tier],
    invest_date: NaiveDate,
    date: NaiveDate,
    unit_nav: f64,
    share: f64,
) -> f64 {
    let days = (date - invest_date).num_days();
    tier_fee(redemption_tiers, days as f64, share * unit_nav)
}

fn tier_fee(tiers: &[Tier], threshold: f64, base: f64) -> f64 {
    tiers
        .iter()
        .filter(|tier| threshold >= tier.lower_bound)
        .max_by(|a, b| a.lower_bound.total_cmp(&b.lower_bound))
        .map_or(0.0, |tier| match tier.kind {
            FeeKind::Pct => tier.rate / 100.0 * base,
            FeeKind::Fixed => tier.rate,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7_30() {
        let mut rule = Fifo::new(
            vec![],
            vec![
                Tier::pct(0.0, 1.5),
                Tier::pct(7.0, 0.5),
                Tier::pct(30.0, 0.0),
            ],
        );
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

    #[test]
    fn test_investment_tiers() {
        let mut rule = Fifo::new(
            vec![
                Tier::pct(0.0, 1.5),
                Tier::pct(1_000_000.0, 1.2),
                Tier::fixed(10_000_000.0, 1000.0),
            ],
            vec![],
        );
        assert_eq!(
            rule.fee(OrderForFee::Invest {
                date: NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                unit_nav: 1.0,
                amount: 100_000.0,
            }),
            1500.0
        );
        assert_eq!(
            rule.fee(OrderForFee::Invest {
                date: NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
                unit_nav: 1.0,
                amount: 2_000_000.0,
            }),
            24_000.0
        );
        assert_eq!(
            rule.fee(OrderForFee::Invest {
                date: NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(),
                unit_nav: 1.0,
                amount: 20_000_000.0,
            }),
            1000.0
        );
    }
}
