wit_bindgen::generate!({
    world: "fund-strategy",
    path: "../../../wit/strategy.wit",
});

use exports::fund::strategy::trader::{Event, Order};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Mutex;

fn default_amount() -> f64 {
    100.0
}

fn default_interval() -> u64 {
    7
}

#[derive(Deserialize, JsonSchema)]
struct DcaParams {
    /// Amount to invest on each buy day.
    #[serde(default = "default_amount")]
    #[schemars(range(min = 0.0), default = "default_amount", title = "Amount")]
    amount: f64,
    /// Buy every N trading days.
    #[serde(default = "default_interval")]
    #[schemars(range(min = 1), default = "default_interval", title = "Interval (days)")]
    interval: u64,
}

#[derive(Default)]
struct State {
    params: Option<DcaParams>,
    day: u64,
}

static STATE: Mutex<State> = Mutex::new(State { params: None, day: 0 });

struct FundStrategies;

impl exports::fund::strategy::trader::Guest for FundStrategies {
    fn name() -> String {
        "Dollar Cost Averaging".to_string()
    }

    fn description() -> String {
        "invest a fixed amount on a regular schedule".to_string()
    }

    fn config_schema() -> String {
        serde_json::to_string(&schemars::schema_for!(DcaParams)).unwrap()
    }

    fn init(config: String) -> Result<(), String> {
        let mut state = STATE.lock().unwrap();
        state.params = serde_json::from_str(&config).map_err(|e| e.to_string())?;
        state.day = 0;
        Ok(())
    }

    fn on_event(event: Event) -> Vec<Order> {
        let mut state = STATE.lock().unwrap();
        let (amount, interval) = match &state.params {
            Some(params) => (params.amount, params.interval),
            None => return Vec::new(),
        };
        if let Event::NavUpdate(_) = event {
            let buy = state.day == 0 || state.day.is_multiple_of(interval);
            state.day += 1;
            if buy {
                return vec![Order::Invest(amount)];
            }
        }
        Vec::new()
    }
}

export!(FundStrategies);