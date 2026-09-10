use crate::{
    eastmoney::Nav,
    error::{Error, Result},
    rules::OrderForFee,
    sim::{
        event::{Event, Order, Transaction, TransactionKind},
        fees::FeeService,
        state::{DailySnapshot, PortfolioState},
        strategy::{SimContext, Strategy},
    },
};
use std::collections::VecDeque;

/// Guard against a strategy that keeps returning orders on `OrderExecuted`.
const MAX_ORDERS_PER_DAY: usize = 10_000;

pub struct SimulationResult {
    pub final_state: PortfolioState,
    pub transactions: Vec<Transaction>,
    pub snapshots: Vec<DailySnapshot>,
}

pub fn simulate(
    navs: &[Nav],
    fee_service: &FeeService,
    strategy: &mut dyn Strategy,
    capital: f64,
) -> Result<SimulationResult> {
    let mut state = PortfolioState::with_capital(capital);
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut snapshots: Vec<DailySnapshot> = Vec::new();

    for nav in navs {
        // Same-day settlement: decisions and executions both use today's nav.
        fee_service.set_today(nav.date, nav.unit_nav);
        let mut queue: VecDeque<Order> = VecDeque::new();

        dispatch(
            strategy,
            &Event::DayStart(nav.date),
            nav,
            &state,
            &transactions,
            &mut queue,
        );
        drain(
            &mut queue,
            &mut state,
            nav,
            fee_service,
            strategy,
            &mut transactions,
        )?;

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
            &mut queue,
        );
        drain(
            &mut queue,
            &mut state,
            nav,
            fee_service,
            strategy,
            &mut transactions,
        )?;

        snapshots.push(DailySnapshot {
            date: nav.date,
            unit_nav: nav.unit_nav,
            holding_price: state.holding_price,
            holding_share: state.holding_share,
            cumulative_investment: state.cumulative_investment,
            cumulative_redemption: state.cumulative_redemption,
            cash: state.cash,
            capital: state.capital,
        });

        dispatch(
            strategy,
            &Event::DayEnd(nav.date),
            nav,
            &state,
            &transactions,
            &mut queue,
        );
        drain(
            &mut queue,
            &mut state,
            nav,
            fee_service,
            strategy,
            &mut transactions,
        )?;
    }

    Ok(SimulationResult {
        final_state: state,
        transactions,
        snapshots,
    })
}

/// Execute every queued order at the current day's nav, feeding each execution
/// back to the strategy (which may enqueue more orders, also settled today).
fn drain(
    queue: &mut VecDeque<Order>,
    state: &mut PortfolioState,
    nav: &Nav,
    fee_service: &FeeService,
    strategy: &mut dyn Strategy,
    transactions: &mut Vec<Transaction>,
) -> Result<()> {
    let mut processed = 0;
    while let Some(order) = queue.pop_front() {
        processed += 1;
        if processed > MAX_ORDERS_PER_DAY {
            break;
        }
        let Some(transaction) = execute(state, order, nav, fee_service)? else {
            continue;
        };
        transactions.push(transaction.clone());
        dispatch(
            strategy,
            &Event::OrderExecuted {
                date: nav.date,
                transaction,
            },
            nav,
            state,
            transactions,
            queue,
        );
    }
    Ok(())
}

fn dispatch(
    strategy: &mut dyn Strategy,
    event: &Event,
    nav: &Nav,
    state: &PortfolioState,
    transactions: &[Transaction],
    out: &mut VecDeque<Order>,
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
    fee_service: &FeeService,
) -> Result<Option<Transaction>> {
    match order {
        Order::Invest { amount } => {
            // Clamp the investment to the available cash; skip when none is left.
            let amount = amount.min(state.cash);
            if amount <= 0.0 {
                return Ok(None);
            }
            let fee = fee_service.apply(OrderForFee::Invest {
                date: nav.date,
                unit_nav: nav.unit_nav,
                amount,
            });
            let shares = (amount - fee) / nav.unit_nav;
            state.invest(amount, shares);
            Ok(Some(Transaction {
                date: nav.date,
                unit_nav: nav.unit_nav,
                kind: TransactionKind::Invest {
                    amount,
                    shares,
                    fee,
                },
            }))
        }
        Order::Redeem { shares } => {
            // Tolerate IEEE-754 rounding between a strategy's own lot
            // bookkeeping and the engine's running share total; a genuine
            // over-redeem (beyond a relative epsilon) is still an error.
            let tolerance = state.holding_share.abs() * 1e-9 + 1e-9;
            if shares > state.holding_share + tolerance {
                return Err(Error::Insufficient);
            }
            let shares = shares.min(state.holding_share);
            let fee = fee_service.apply(OrderForFee::Redeem {
                date: nav.date,
                unit_nav: nav.unit_nav,
                shares,
            });
            let money = nav.unit_nav * shares - fee;
            state.redeem(shares, money);
            Ok(Some(Transaction {
                date: nav.date,
                unit_nav: nav.unit_nav,
                kind: TransactionKind::Redeem { shares, money, fee },
            }))
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

    fn run(strategy: &mut dyn Strategy, capital: f64) -> Result<SimulationResult> {
        let navs = navs();
        let fee = FeeService::new(Box::new(Fifo::new(vec![], vec![])), navs[0].date);
        simulate(&navs, &fee, strategy, capital)
    }

    #[test]
    fn test_invest_then_redeem_same_day() {
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

        let mut strategy = BuyThenSell { step: 0 };
        let result = run(&mut strategy, 1000.0).unwrap();

        // Same-day: invest on day 0's nav (1.0) -> 100 shares; redeem 50 on
        // day 1's nav (1.05).
        assert_eq!(result.transactions.len(), 2);
        assert_eq!(result.final_state.holding_share, 100.0 - 50.0);
        assert_eq!(result.final_state.cumulative_investment, 100.0);
        assert_eq!(result.final_state.cumulative_redemption, 50.0 * 1.05);
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

        let mut strategy = BuyThenOversell { step: 0 };
        let result = run(&mut strategy, 1000.0);
        assert!(matches!(result, Err(Error::Insufficient)));
    }

    #[test]
    fn test_invest_clamped_to_capital() {
        struct Clamp {
            done: bool,
        }
        impl Strategy for Clamp {
            fn name(&self) -> &str {
                "clamp"
            }
            fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
                if !self.done
                    && let Event::NavUpdate { .. } = event
                {
                    self.done = true;
                    return vec![Order::Invest { amount: 500.0 }];
                }
                Vec::new()
            }
        }

        let mut strategy = Clamp { done: false };
        let result = run(&mut strategy, 200.0).unwrap();
        // Clamped to the available capital of 200, invested on day 0's nav (1.0).
        assert_eq!(result.transactions.len(), 1);
        assert_eq!(result.final_state.cumulative_investment, 200.0);
        assert_eq!(result.final_state.holding_share, 200.0 / 1.0);
        assert_eq!(result.final_state.cash, 0.0);
    }

    #[test]
    fn test_invest_skipped_when_no_cash() {
        struct Skip {
            done: bool,
        }
        impl Strategy for Skip {
            fn name(&self) -> &str {
                "skip"
            }
            fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
                if !self.done
                    && let Event::NavUpdate { .. } = event
                {
                    self.done = true;
                    return vec![Order::Invest { amount: 500.0 }];
                }
                Vec::new()
            }
        }

        let mut strategy = Skip { done: false };
        let result = run(&mut strategy, 0.0).unwrap();
        // No cash to invest: the order is skipped entirely.
        assert_eq!(result.transactions.len(), 0);
        assert_eq!(result.final_state.cumulative_investment, 0.0);
        assert_eq!(result.final_state.cash, 0.0);
    }
}
