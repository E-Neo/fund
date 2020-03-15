use crate::world::{RepositoryError, Trade, TradingRule};
use chrono::{Duration, NaiveDate};
use std::collections::LinkedList;

pub struct WeekRule {
    holding_shares: LinkedList<(NaiveDate, f64)>,
}

impl WeekRule {
    pub fn new() -> Self {
        WeekRule {
            holding_shares: LinkedList::new(),
        }
    }
}

impl TradingRule for WeekRule {
    fn buy(&mut self, date: NaiveDate, nav: f64, cost: f64) -> Result<Trade, RepositoryError> {
        if cost >= 10. {
            self.holding_shares
                .push_back((date + Duration::days(1), cost / nav));
            Ok(Trade::buy(date, nav, cost))
        } else {
            Err(RepositoryError::BuyError)
        }
    }

    fn sell(&mut self, date: NaiveDate, nav: f64, share: f64) -> Result<Trade, RepositoryError> {
        let mut feed_share = 0.;
        let mut share_left = share;
        while share_left > 0. {
            let (brought_date, brought_share) = self
                .holding_shares
                .pop_front()
                .ok_or(RepositoryError::SellError)?;
            if date - brought_date >= Duration::days(7) {
                if brought_share < share_left {
                    share_left -= brought_share;
                } else {
                    self.holding_shares
                        .push_front((brought_date, brought_share - share_left));
                    share_left = 0.;
                }
            } else {
                if brought_share < share_left {
                    feed_share += brought_share;
                    share_left -= brought_share;
                } else {
                    feed_share += share_left;
                    self.holding_shares
                        .push_front((brought_date, brought_share - share_left));
                    share_left = 0.;
                }
            }
        }
        Ok(Trade::sell(
            date,
            nav * (1. - feed_share / share * 0.015),
            share,
        ))
    }
}
