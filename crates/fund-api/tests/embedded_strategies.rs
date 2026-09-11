use chrono::NaiveDate;
use fund_api::{
    eastmoney::Nav,
    error::Result,
    fees::FeeRule,
    rules::Fifo,
    sim::{
        engine,
        event::{Event, Order},
        fees::FeeService,
        strategy::{self, SimContext, Strategy},
    },
};
use std::sync::Arc;

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

fn price_navs(prices: &[f64]) -> Vec<Nav> {
    prices
        .iter()
        .enumerate()
        .map(|(i, p)| Nav {
            date: NaiveDate::from_ymd_opt(2021, 1, 1).unwrap() + chrono::Duration::days(i as i64),
            unit_nav: *p,
            accum_nav: *p,
            daily_return: None,
        })
        .collect()
}

fn service(navs: &[Nav], rule: &FeeRule) -> FeeService {
    FeeService::new(
        Box::new(Fifo::new(rule.subscribe.clone(), rule.redeem.clone())),
        navs[0].date,
    )
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

    let mut native = NativeDca {
        amount: 100.0,
        interval: 5,
        day: 0,
    };
    let native_result = engine::simulate(
        &navs,
        &service(&navs, &FeeRule::default()),
        &mut native,
        10_000.0,
    )?;
    assert_eq!(native_result.transactions.len(), 4);

    let config = serde_json::json!({"amount": 100, "interval": 5});
    let fee_rule = FeeRule::default();
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &fee_rule,
        fee_service: Arc::new(service(&navs, &fee_rule)),
        capital: 10_000.0,
    };
    let mut wasm = strategy::load(
        &strategy::StrategyArg::Bundled("Dollar Cost Averaging".to_string()),
        &config,
        &ctx,
    )?;
    let wasm_result = engine::simulate(&navs, &service(&navs, &fee_rule), wasm.as_mut(), 10_000.0)?;

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
    let fee_rule = FeeRule::default();
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &fee_rule, 1000.0);
    let result = engine::simulate(&navs, &service(&navs, &fee_rule), &mut oracle, 1000.0)?;
    // One buy (day 0, nav 1.0) and one sell (day 9, nav 1.9).
    assert_eq!(result.transactions.len(), 2);
    assert_eq!(result.final_state.cash, 1900.0);
    Ok(())
}

#[test]
fn oracle_captures_multiple_swings() -> Result<()> {
    // Same-day settlement: buy at each 1.0 trough and sell at each peak.
    let navs = price_navs(&[1.0, 2.0, 1.0, 3.0, 1.0, 4.0]);
    let fee_rule = FeeRule::default();
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &fee_rule, 1000.0);
    let result = engine::simulate(&navs, &service(&navs, &fee_rule), &mut oracle, 1000.0)?;
    // Three round trips (1->2, 1->3, 1->4): six transactions, 2x3x4 = 24x.
    assert_eq!(result.transactions.len(), 6);
    assert_eq!(result.final_state.cash, 24_000.0);
    Ok(())
}

#[test]
fn oracle_skips_fee_eaten_swing() -> Result<()> {
    // A 2% subscription fee makes the 1 -> 1.01 swing a net loss.
    let rule = FeeRule {
        subscribe: vec![fund_api::rules::Tier::pct(0.0, 2.0)],
        redeem: vec![],
    };
    let navs = price_navs(&[1.0, 1.01, 1.0, 1.01]);
    let mut oracle = fund_api::sim::oracle::Oracle::new(&navs, &rule, 1000.0);
    let result = engine::simulate(&navs, &service(&navs, &rule), &mut oracle, 1000.0)?;
    assert_eq!(result.transactions.len(), 0);
    assert_eq!(result.final_state.cash, 1000.0);
    Ok(())
}

#[test]
fn dip_take_profit_buys_dip_and_takes_profit() -> Result<()> {
    // Drop from 1.0 to 0.95 triggers a buy; rebound to 1.05 hits the target.
    let navs = price_navs(&[1.0, 0.95, 1.05]);
    let fee_rule = FeeRule::default();
    let fee_service = Arc::new(service(&navs, &fee_rule));
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &fee_rule,
        fee_service: Arc::clone(&fee_service),
        capital: 1000.0,
    };
    let params = serde_json::json!({"amount": 100, "drop_pct": 3, "take_profit_pct": 5});
    let mut strategy = strategy::load(
        &strategy::StrategyArg::Bundled("Dip Take Profit".to_string()),
        &params,
        &ctx,
    )?;
    let result = engine::simulate(&navs, &fee_service, strategy.as_mut(), 1000.0)?;
    // Buy 100 at 0.95 (100/0.95 shares), redeem at 1.05: net 110.5, so cash = 1010.5.
    assert_eq!(result.transactions.len(), 2);
    assert!((result.final_state.cash - 1010.526).abs() < 0.01);
    Ok(())
}

#[test]
fn dip_take_profit_handles_same_day_buy_and_redeem() -> Result<()> {
    // On day 5 both a take-profit (the older lot) and a dip buy fire. The
    // redeem must be executed before the buy, otherwise the FILO pop removes
    // the just-bought lot and a later redeem over-redeems (HTTP 500).
    let navs = price_navs(&[1.1, 1.0, 1.0, 0.95, 1.2, 1.15, 1.3]);
    let fee_rule = FeeRule::default();
    let fee_service = Arc::new(service(&navs, &fee_rule));
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &fee_rule,
        fee_service: Arc::clone(&fee_service),
        capital: 1000.0,
    };
    let params = serde_json::json!({"amount": 100, "drop_pct": 4, "take_profit_pct": 10});
    let mut strategy = strategy::load(
        &strategy::StrategyArg::Bundled("Dip Take Profit".to_string()),
        &params,
        &ctx,
    )?;
    let result = engine::simulate(&navs, &fee_service, strategy.as_mut(), 1000.0)?;
    assert!(result.transactions.len() >= 4);
    Ok(())
}

#[test]
fn dip_take_profit_requires_consecutive_days() -> Result<()> {
    // n=3: buys only on day 4, when the run has 3 consecutive down days and
    // its cumulative drop from the pre-streak peak (1.0) reaches 6%.
    let navs = price_navs(&[1.0, 0.98, 0.95, 0.94, 1.05]);
    let fee_rule = FeeRule::default();
    let fee_service = Arc::new(service(&navs, &fee_rule));
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &fee_rule,
        fee_service: Arc::clone(&fee_service),
        capital: 1000.0,
    };
    let params = serde_json::json!({
        "amount": 100,
        "consecutive_days": 3,
        "drop_pct": 5,
        "take_profit_pct": 10
    });
    let mut strategy = strategy::load(
        &strategy::StrategyArg::Bundled("Dip Take Profit".to_string()),
        &params,
        &ctx,
    )?;
    let result = engine::simulate(&navs, &fee_service, strategy.as_mut(), 1000.0)?;
    // Bought 100 at 0.94 (100/0.94 shares), redeemed at 1.05: net 111.70,
    // so cash = 1000 - 100 + 111.70 = 1011.70.
    assert_eq!(result.transactions.len(), 2);
    assert!((result.final_state.cash - 1011.702).abs() < 0.01);
    Ok(())
}

#[test]
fn dip_take_profit_breaks_streak_on_up_day() -> Result<()> {
    // n=2: day 2 drops 7% but is only a single down day; day 3 breaks the
    // streak; day 4's single down day from 0.95 (5.3% drop) never reaches
    // n=2, so no buy happens at all.
    let navs = price_navs(&[1.0, 0.93, 0.95, 0.90, 1.0]);
    let fee_rule = FeeRule::default();
    let fee_service = Arc::new(service(&navs, &fee_rule));
    let ctx = strategy::StrategyCtx {
        navs: &navs,
        fee_rule: &fee_rule,
        fee_service: Arc::clone(&fee_service),
        capital: 1000.0,
    };
    let params = serde_json::json!({
        "amount": 100,
        "consecutive_days": 2,
        "drop_pct": 5,
        "take_profit_pct": 10
    });
    let mut strategy = strategy::load(
        &strategy::StrategyArg::Bundled("Dip Take Profit".to_string()),
        &params,
        &ctx,
    )?;
    let result = engine::simulate(&navs, &fee_service, strategy.as_mut(), 1000.0)?;
    assert_eq!(result.transactions.len(), 0);
    assert_eq!(result.final_state.cash, 1000.0);
    Ok(())
}
