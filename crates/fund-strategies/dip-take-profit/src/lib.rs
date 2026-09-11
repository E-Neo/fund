wit_bindgen::generate!({
    world: "fund-strategy",
    path: "../../../wit/strategy.wit",
});

use exports::fund::strategy::trader::{Event, Order, TransactionKind};
use fund::strategy::fees;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Mutex;

fn default_amount() -> f64 {
    1000.0
}

fn default_drop_pct() -> f64 {
    3.0
}

fn default_take_profit_pct() -> f64 {
    5.0
}

fn default_consecutive_days() -> u32 {
    1
}

#[derive(Deserialize, JsonSchema)]
struct DipParams {
    /// Fixed amount invested on each buy.
    #[serde(default = "default_amount")]
    #[schemars(range(min = 0.0), default = "default_amount", title = "Amount")]
    amount: f64,
    /// Number of consecutive down days required before a buy.
    #[serde(default = "default_consecutive_days")]
    #[schemars(
        range(min = 1),
        default = "default_consecutive_days",
        title = "Consecutive down days"
    )]
    consecutive_days: u32,
    /// Buy when the cumulative drop over the consecutive down days reaches
    /// this percent.
    #[serde(default = "default_drop_pct")]
    #[schemars(range(min = 0.0), default = "default_drop_pct", title = "Drop %")]
    drop_pct: f64,
    /// Redeem a lot once its net gain reaches this percent.
    #[serde(default = "default_take_profit_pct")]
    #[schemars(
        range(min = 0.0),
        default = "default_take_profit_pct",
        title = "Take profit %"
    )]
    take_profit_pct: f64,
}

struct Lot {
    shares: f64,
    cost: f64,
}

struct State {
    params: Option<DipParams>,
    prev_nav: Option<f64>,
    stack: Vec<Lot>,
    down_streak: u32,
    run_navs: VecDeque<f64>,
}

static STATE: Mutex<State> = Mutex::new(State {
    params: None,
    prev_nav: None,
    stack: Vec::new(),
    down_streak: 0,
    run_navs: VecDeque::new(),
});

struct FundStrategies;

impl exports::fund::strategy::trader::Guest for FundStrategies {
    fn name() -> String {
        "Dip Take Profit".to_string()
    }

    fn description() -> String {
        "buy on consecutive down days, take profit per lot (FILO)".to_string()
    }

    fn config_schema() -> String {
        serde_json::to_string(&schemars::schema_for!(DipParams)).unwrap()
    }

    fn init(config: String) -> Result<(), String> {
        let mut state = STATE.lock().unwrap();
        state.params = serde_json::from_str(&config).map_err(|e| e.to_string())?;
        if state.params.as_ref().is_some_and(|p| p.consecutive_days == 0) {
            return Err("consecutive_days must be >= 1".to_string());
        }
        state.prev_nav = None;
        state.stack.clear();
        state.down_streak = 0;
        state.run_navs.clear();
        Ok(())
    }

    fn on_event(event: Event) -> Vec<Order> {
        let mut state = STATE.lock().unwrap();
        let (amount, consecutive_days, drop_pct, take_profit_pct) = match &state.params {
            Some(params) => (
                params.amount,
                params.consecutive_days,
                params.drop_pct,
                params.take_profit_pct,
            ),
            None => return Vec::new(),
        };
        match event {
            Event::NavUpdate(nav) => {
                let mut orders = Vec::new();
                // Redeem the top (most recent) lot once its net gain hits the
                // target. Emitted before any buy so the engine pops this lot
                // before a new one is pushed (settlement is same-day).
                if let Some(top) = state.stack.last() {
                    let fee = fees::redeem_fee(top.shares);
                    let net = top.shares * nav.unit_nav - fee;
                    if net >= top.cost * (1.0 + take_profit_pct / 100.0) {
                        orders.push(Order::Redeem(top.shares));
                    }
                }
                // Buy once the down-run has n consecutive down days and the
                // cumulative drop over the last n days (from the nav n
                // sessions back to today) reaches the target. The reference
                // window slides forward as the run lengthens, so n=1 is the
                // plain "drop versus the previous session" rule.
                if let Some(prev) = state.prev_nav {
                    if nav.unit_nav < prev {
                        if state.down_streak == 0 {
                            state.run_navs.clear();
                            state.run_navs.push_back(prev);
                        }
                        state.down_streak += 1;
                        state.run_navs.push_back(nav.unit_nav);
                        let window = consecutive_days as usize + 1;
                        while state.run_navs.len() > window {
                            state.run_navs.pop_front();
                        }
                        if state.down_streak >= consecutive_days
                            && let Some(ref_nav) = state.run_navs.front()
                            && nav.unit_nav <= *ref_nav * (1.0 - drop_pct / 100.0)
                        {
                            orders.push(Order::Invest(amount));
                            state.down_streak = 0;
                            state.run_navs.clear();
                        }
                    } else {
                        state.down_streak = 0;
                        state.run_navs.clear();
                    }
                }
                state.prev_nav = Some(nav.unit_nav);
                orders
            }
            Event::OrderExecuted(executed) => {
                match executed.kind {
                    TransactionKind::Invest(invested) => {
                        state.stack.push(Lot {
                            shares: invested.shares,
                            cost: invested.amount,
                        });
                    }
                    TransactionKind::Redeem(_) => {
                        state.stack.pop();
                    }
                }
                Vec::new()
            }
        }
    }
}

export!(FundStrategies);
