use crate::repository::{Order, Rule};
use chrono::NaiveDate;
use std::collections::VecDeque;

pub struct FIFO {
    queue: VecDeque<(NaiveDate, f64)>, // (invest_date, share)
    investment_rates: Vec<(f64, f64)>,
    redemption_rates: Vec<(usize, f64)>,
}

impl FIFO {
    pub fn new(investment_rates: Vec<(f64, f64)>, redemption_rates: Vec<(usize, f64)>) -> Self {
        Self {
            queue: VecDeque::new(),
            investment_rates,
            redemption_rates,
        }
    }
}

impl Rule for FIFO {
    fn fee(&mut self, order: Order) -> f64 {
        match order {
            Order::Investment {
                date,
                net_asset_value,
                investment,
            } => {
                let fee = self
                    .investment_rates
                    .iter()
                    .find(|&&(bound, _)| investment < bound)
                    .map_or(0., |&(_, rate)| rate * investment);
                self.queue
                    .push_back((date, (investment - fee) / net_asset_value));
                fee
            }
            Order::Redemption {
                date,
                net_asset_value,
                mut redemption,
            } => {
                let mut fee = 0.;
                while let Some((invest_date, share)) = self.queue.pop_front() {
                    if redemption < share {
                        self.queue.push_front((invest_date, share - redemption));
                        fee += calculate_redemption_fee(
                            &self.redemption_rates,
                            invest_date,
                            date,
                            net_asset_value,
                            redemption,
                        );
                        break;
                    } else {
                        redemption -= share;
                        fee += calculate_redemption_fee(
                            &self.redemption_rates,
                            invest_date,
                            date,
                            net_asset_value,
                            share,
                        );
                    }
                }
                fee
            }
        }
    }
}

fn calculate_redemption_fee(
    redemption_rates: &[(usize, f64)],
    invest_date: NaiveDate,
    date: NaiveDate,
    net_asset_value: f64,
    share: f64,
) -> f64 {
    share
        * net_asset_value
        * redemption_rates
            .into_iter()
            .find(|&&(bound, _)| (date - invest_date).num_days() < bound as i64)
            .map_or(0., |&(_, rate)| rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7_30() {
        let mut rule = FIFO::new(vec![], vec![(7, 0.015), (30, 0.005)]);
        assert_eq!(
            rule.fee(Order::Investment {
                date: NaiveDate::from_ymd(2021, 1, 1),
                net_asset_value: 1.0,
                investment: 100.0
            }),
            0.0,
        );
        assert_eq!(
            rule.fee(Order::Redemption {
                date: NaiveDate::from_ymd(2021, 1, 10),
                net_asset_value: 1.0,
                redemption: 10.0
            }),
            0.05,
        );
        assert_eq!(
            rule.fee(Order::Investment {
                date: NaiveDate::from_ymd(2021, 2, 1),
                net_asset_value: 1.0,
                investment: 100.0
            }),
            0.0,
        );
        assert_eq!(
            rule.fee(Order::Redemption {
                date: NaiveDate::from_ymd(2021, 2, 5),
                net_asset_value: 1.05,
                redemption: 190.0
            }),
            100.0 * 1.05 * 0.015,
        );
    }
}
