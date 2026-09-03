use chrono::NaiveDate;
use fund_api::{
    eastmoney::Nav,
    error::Result,
    rules::Fifo,
    sim::{
        engine,
        event::{Event, Order},
        strategy::{self, SimContext, Strategy},
    },
};

fn navs(days: usize) -> Vec<Nav> {
    NaiveDate::from_ymd_opt(2021, 1, 1)
        .unwrap()
        .iter_days()
        .take(days)
        .enumerate()
        .map(|(i, date)| Nav {
            date,
            unit_nav: if i & 1 == 0 { 1.0 } else { 1.05 },
            accum_nav: if i & 1 == 0 { 1.0 } else { 1.05 },
            daily_return: None,
        })
        .collect()
}

struct NativeDca {
    amount: f64,
    interval: u64,
    day: u64,
}

impl Strategy for NativeDca {
    fn name(&self) -> &str {
        "native_dca"
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

struct NativeBuyHold {
    amount: f64,
    invested: bool,
}

impl Strategy for NativeBuyHold {
    fn name(&self) -> &str {
        "native_buy_hold"
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

#[test]
fn embedded_dca_matches_native() -> Result<()> {
    let navs = navs(20);
    let mut fee_rule = Fifo::new(vec![], vec![]);

    let mut native = NativeDca {
        amount: 100.0,
        interval: 5,
        day: 0,
    };
    let native_result = engine::simulate(&navs, &mut fee_rule, &mut native)?;
    assert_eq!(native_result.transactions.len(), 4);

    let config = "strategy = \"dca\"\namount = 100\ninterval = 5";
    let mut wasm = strategy::embedded("dca", config)?;
    let mut fee_rule = Fifo::new(vec![], vec![]);
    let wasm_result = engine::simulate(&navs, &mut fee_rule, wasm.as_mut())?;

    assert_eq!(
        wasm_result.transactions.len(),
        native_result.transactions.len()
    );
    assert_eq!(
        wasm_result.final_state.holding_share,
        native_result.final_state.holding_share
    );
    assert_eq!(
        wasm_result.final_state.cumulative_investment,
        native_result.final_state.cumulative_investment
    );
    Ok(())
}

#[test]
fn embedded_buy_hold_matches_native() -> Result<()> {
    let navs = navs(20);
    let mut fee_rule = Fifo::new(vec![], vec![]);

    let mut native = NativeBuyHold {
        amount: 100.0,
        invested: false,
    };
    let native_result = engine::simulate(&navs, &mut fee_rule, &mut native)?;
    assert_eq!(native_result.transactions.len(), 1);

    let config = "strategy = \"buy_hold\"\namount = 100";
    let mut wasm = strategy::embedded("buy_hold", config)?;
    let mut fee_rule = Fifo::new(vec![], vec![]);
    let wasm_result = engine::simulate(&navs, &mut fee_rule, wasm.as_mut())?;

    assert_eq!(
        wasm_result.transactions.len(),
        native_result.transactions.len()
    );
    assert_eq!(
        wasm_result.final_state.holding_share,
        native_result.final_state.holding_share
    );
    assert_eq!(
        wasm_result.final_state.cumulative_investment,
        native_result.final_state.cumulative_investment
    );
    Ok(())
}
