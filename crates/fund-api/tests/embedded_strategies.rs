use chrono::NaiveDate;
use fund_api::{
    eastmoney::Nav,
    error::Result,
    fees::FeeRule,
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

#[test]
fn embedded_dca_matches_native() -> Result<()> {
    let navs = navs(20);
    let mut fee_rule = Fifo::new(vec![], vec![]);

    let mut native = NativeDca {
        amount: 100.0,
        interval: 5,
        day: 0,
    };
    let native_result = engine::simulate(&navs, &mut fee_rule, &mut native, 10_000.0)?;
    assert_eq!(native_result.transactions.len(), 4);

    let config = serde_json::json!({"amount": 100, "interval": 5});
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &FeeRule::default(),
        capital: 10_000.0,
    };
    let mut wasm = strategy::load(
        &strategy::StrategyArg::Bundled("Dollar Cost Averaging".to_string()),
        &config,
        &ctx,
    )?;
    let mut fee_rule = Fifo::new(vec![], vec![]);
    let wasm_result = engine::simulate(&navs, &mut fee_rule, wasm.as_mut(), 10_000.0)?;

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
fn oracle_profits_from_uptrend() -> Result<()> {
    let navs: Vec<Nav> = (0..10)
        .map(|i| {
            let date =
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap() + chrono::Duration::days(i as i64);
            Nav {
                date,
                unit_nav: 1.0 + i as f64 * 0.1,
                accum_nav: 1.0 + i as f64 * 0.1,
                daily_return: None,
            }
        })
        .collect();
    let mut fee_rule = Fifo::new(vec![], vec![]);
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &FeeRule::default(), 1000.0);
    let result = engine::simulate(&navs, &mut fee_rule, &mut oracle, 1000.0)?;
    // One buy and one sell.
    assert_eq!(result.transactions.len(), 2);
    // Bought at nav[1] (1.1) and sold at nav[9] (1.9): a clear profit.
    assert!(result.final_state.cumulative_redemption > 1000.0);
    Ok(())
}

#[test]
fn oracle_captures_multiple_swings() -> Result<()> {
    // Uptrends from 1 -> 2 -> 1 -> 3 -> 1 -> 4 (effective series nav[1..]).
    let prices = [1.0, 2.0, 1.0, 3.0, 1.0, 4.0];
    let navs: Vec<Nav> = prices
        .iter()
        .enumerate()
        .map(|(i, p)| Nav {
            date: NaiveDate::from_ymd_opt(2021, 1, 1).unwrap() + chrono::Duration::days(i as i64),
            unit_nav: *p,
            accum_nav: *p,
            daily_return: None,
        })
        .collect();
    let mut fee_rule = Fifo::new(vec![], vec![]);
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &FeeRule::default(), 1000.0);
    let result = engine::simulate(&navs, &mut fee_rule, &mut oracle, 1000.0)?;
    // Two round trips (1->3 and 1->4), four transactions, compounding 3x then 4x.
    assert_eq!(result.transactions.len(), 4);
    assert_eq!(result.final_state.cash, 12_000.0);
    Ok(())
}

#[test]
fn oracle_skips_fee_eaten_swing() -> Result<()> {
    // A 2% subscription fee makes the 1 -> 1.01 swing a net loss.
    let rule = FeeRule {
        subscribe: vec![fund_api::rules::Tier::pct(0.0, 2.0)],
        redeem: vec![],
    };
    let prices = [1.0, 1.01, 1.0, 1.01];
    let navs: Vec<Nav> = prices
        .iter()
        .enumerate()
        .map(|(i, p)| Nav {
            date: NaiveDate::from_ymd_opt(2021, 1, 1).unwrap() + chrono::Duration::days(i as i64),
            unit_nav: *p,
            accum_nav: *p,
            daily_return: None,
        })
        .collect();
    let mut fee_rule = Fifo::new(rule.subscribe.clone(), rule.redeem.clone());
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &rule, 1000.0);
    let result = engine::simulate(&navs, &mut fee_rule, &mut oracle, 1000.0)?;
    assert_eq!(result.transactions.len(), 0);
    assert_eq!(result.final_state.cash, 1000.0);
    Ok(())
}
