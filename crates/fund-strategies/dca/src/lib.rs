wit_bindgen::generate!({
    world: "fund-strategy",
    path: "../../../wit/strategy.wit",
});

use exports::fund::strategy::strategy::{Event, Order};
use serde::Deserialize;
use std::sync::Mutex;

/// JSON Schema describing the DCA hyperparameters.
const CONFIG_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "amount": {
      "type": "number",
      "minimum": 0.0,
      "default": 100.0,
      "title": "Amount",
      "description": "Amount to invest on each buy day."
    },
    "interval": {
      "type": "integer",
      "minimum": 1,
      "default": 7,
      "title": "Interval (days)",
      "description": "Buy every N trading days."
    }
  },
  "required": ["amount", "interval"]
}"#;

#[derive(Deserialize)]
struct DcaParams {
    amount: f64,
    interval: u64,
}

#[derive(Default)]
struct State {
    params: Option<DcaParams>,
    day: u64,
}

static STATE: Mutex<State> = Mutex::new(State { params: None, day: 0 });

struct FundStrategies;

impl exports::fund::strategy::strategy::Guest for FundStrategies {
    fn description() -> String {
        "invest a fixed amount on a regular schedule".to_string()
    }

    fn config_schema() -> String {
        CONFIG_SCHEMA.to_string()
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
