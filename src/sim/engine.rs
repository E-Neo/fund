use crate::{
    eastmoney::Nav,
    error::{Error, Result},
    rules::{OrderForFee, Rule},
    sim::{
        event::{Event, Order, Transaction, TransactionKind},
        state::{DailySnapshot, PortfolioState},
        strategy::{SimContext, Strategy},
    },
};

pub struct SimulationResult {
    pub final_state: PortfolioState,
    pub transactions: Vec<Transaction>,
    pub snapshots: Vec<DailySnapshot>,
}

pub fn simulate(
    navs: &[Nav],
    fee_rule: &mut dyn Rule,
    strategy: &mut dyn Strategy,
) -> Result<SimulationResult> {
    let mut state = PortfolioState::default();
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut snapshots: Vec<DailySnapshot> = Vec::new();
    let mut pending: Vec<Order> = Vec::new();

    for nav in navs {
        let mut next_pending: Vec<Order> = Vec::new();

        dispatch(
            strategy,
            &Event::DayStart(nav.date),
            nav,
            &state,
            &transactions,
            &mut next_pending,
        );

        for order in pending {
            let transaction = execute(&mut state, order, nav, fee_rule)?;
            transactions.push(transaction.clone());
            dispatch(
                strategy,
                &Event::OrderExecuted {
                    date: nav.date,
                    transaction,
                },
                nav,
                &state,
                &transactions,
                &mut next_pending,
            );
        }

        dispatch(
            strategy,
            &Event::NavUpdate {
                date: nav.date,
                unit_nav: nav.unit_nav,
                accum_nav: nav.accum_nav,
            },
            nav,
            &state,
            &transactions,
            &mut next_pending,
        );

        snapshots.push(DailySnapshot {
            date: nav.date,
            unit_nav: nav.unit_nav,
            holding_price: state.holding_price,
            holding_share: state.holding_share,
            cumulative_investment: state.cumulative_investment,
            cumulative_redemption: state.cumulative_redemption,
        });

        dispatch(
            strategy,
            &Event::DayEnd(nav.date),
            nav,
            &state,
            &transactions,
            &mut next_pending,
        );

        pending = next_pending;
    }

    Ok(SimulationResult {
        final_state: state,
        transactions,
        snapshots,
    })
}

fn dispatch(
    strategy: &mut dyn Strategy,
    event: &Event,
    nav: &Nav,
    state: &PortfolioState,
    transactions: &[Transaction],
    out: &mut Vec<Order>,
) {
    let mut ctx = SimContext {
        date: nav.date,
        unit_nav: nav.unit_nav,
        accum_nav: nav.accum_nav,
        state,
        transactions,
    };
    out.extend(strategy.on_event(event, &mut ctx));
}

fn execute(
    state: &mut PortfolioState,
    order: Order,
    nav: &Nav,
    fee_rule: &mut dyn Rule,
) -> Result<Transaction> {
    match order {
        Order::Invest { amount } => {
            let fee = fee_rule.fee(OrderForFee::Invest {
                date: nav.date,
                unit_nav: nav.unit_nav,
                amount,
            });
            let shares = (amount - fee) / nav.unit_nav;
            state.invest(amount, shares);
            Ok(Transaction {
                date: nav.date,
                unit_nav: nav.unit_nav,
                kind: TransactionKind::Invest {
                    amount,
                    shares,
                    fee,
                },
            })
        }
        Order::Redeem { shares } => {
            if state.holding_share < shares {
                return Err(Error::Insufficient);
            }
            let fee = fee_rule.fee(OrderForFee::Redeem {
                date: nav.date,
                unit_nav: nav.unit_nav,
                shares,
            });
            let money = nav.unit_nav * shares - fee;
            state.redeem(shares, money);
            Ok(Transaction {
                date: nav.date,
                unit_nav: nav.unit_nav,
                kind: TransactionKind::Redeem { shares, money, fee },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Fifo;
    use chrono::NaiveDate;

    fn navs() -> Vec<Nav> {
        NaiveDate::from_ymd_opt(2021, 1, 1)
            .unwrap()
            .iter_days()
            .take(5)
            .enumerate()
            .map(|(i, date)| Nav {
                date,
                unit_nav: if i & 1 == 0 { 1.0 } else { 1.05 },
                accum_nav: if i & 1 == 0 { 1.0 } else { 1.05 },
                daily_return: None,
            })
            .collect()
    }

    #[test]
    fn test_buy_hold_zero_fee() {
        let mut fee_rule = Fifo::new(vec![], vec![]);
        let mut strategy = crate::sim::strategy::create("buy_hold", 100.0, 0.0, 0).unwrap();
        let result = simulate(&navs(), &mut fee_rule, strategy.as_mut()).unwrap();
        assert_eq!(result.transactions.len(), 1);
        // Order placed on day 1's NavUpdate executes at day 2's nav (1.05).
        assert_eq!(result.final_state.holding_share, 100.0 / 1.05);
        assert_eq!(result.snapshots.len(), 5);
    }

    #[test]
    fn test_invest_then_redeem_t_plus_one() {
        struct BuyThenSell {
            step: u32,
        }
        impl Strategy for BuyThenSell {
            fn name(&self) -> &str {
                "buy_then_sell"
            }
            fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
                match (self.step, event) {
                    (0, Event::NavUpdate { .. }) => {
                        self.step += 1;
                        vec![Order::Invest { amount: 100.0 }]
                    }
                    (1, Event::NavUpdate { .. }) => {
                        self.step += 1;
                        vec![Order::Redeem { shares: 50.0 }]
                    }
                    _ => Vec::new(),
                }
            }
        }

        let mut fee_rule = Fifo::new(vec![], vec![]);
        let mut strategy = BuyThenSell { step: 0 };
        let result = simulate(&navs(), &mut fee_rule, &mut strategy).unwrap();

        // Invest executes on day 2's nav (1.05) -> 100 / 1.05 shares.
        // Redeem executes on day 3's nav (1.0) -> 50 shares sold.
        assert_eq!(result.transactions.len(), 2);
        assert_eq!(result.final_state.holding_share, 100.0 / 1.05 - 50.0);
        assert_eq!(result.final_state.cumulative_investment, 100.0);
        assert_eq!(result.final_state.cumulative_redemption, 50.0 * 1.0);
    }

    #[test]
    fn test_redeem_more_than_held_fails() {
        struct BuyThenOversell {
            step: u32,
        }
        impl Strategy for BuyThenOversell {
            fn name(&self) -> &str {
                "buy_then_oversell"
            }
            fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
                match (self.step, event) {
                    (0, Event::NavUpdate { .. }) => {
                        self.step += 1;
                        vec![Order::Invest { amount: 100.0 }]
                    }
                    (1, Event::NavUpdate { .. }) => {
                        self.step += 1;
                        vec![Order::Redeem { shares: 9999.0 }]
                    }
                    _ => Vec::new(),
                }
            }
        }

        let mut fee_rule = Fifo::new(vec![], vec![]);
        let mut strategy = BuyThenOversell { step: 0 };
        let result = simulate(&navs(), &mut fee_rule, &mut strategy);
        assert!(matches!(result, Err(Error::Insufficient)));
    }
}
