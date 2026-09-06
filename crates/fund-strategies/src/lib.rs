wit_bindgen::generate!({
    world: "fund-strategy",
    path: "../../wit/strategy.wit",
});

use exports::fund::strategy::strategy::{Event, Order};
use serde::Deserialize;
use std::sync::Mutex;

#[derive(Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
enum StrategyKind {
    Dca { amount: f64, interval: u64 },
}

#[derive(Default)]
struct State {
    kind: Option<StrategyKind>,
    day: u64,
}

static STATE: Mutex<State> = Mutex::new(State { kind: None, day: 0 });

struct FundStrategies;

impl exports::fund::strategy::strategy::Guest for FundStrategies {
    fn init(config: String) {
        let mut state = STATE.lock().unwrap();
        state.kind = serde_json::from_str(&config).ok();
        state.day = 0;
    }

    fn on_event(event: Event) -> Vec<Order> {
        let mut state = STATE.lock().unwrap();
        let (amount, interval) = match &state.kind {
            Some(StrategyKind::Dca { amount, interval }) => (*amount, *interval),
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
