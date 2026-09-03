wit_bindgen::generate!({
    world: "fund-strategy",
    path: "../../wit/strategy.wit",
});

use exports::fund::strategy::strategy::{Event, Order};
use std::sync::Mutex;

enum StrategyKind {
    BuyHold {
        amount: f64,
        invested: bool,
    },
    Dca {
        amount: f64,
        interval: u64,
        day: u64,
    },
}

#[derive(Default)]
struct State {
    kind: Option<StrategyKind>,
}

static STATE: Mutex<State> = Mutex::new(State { kind: None });

fn as_f64(value: &toml::Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|i| i as f64))
}

struct FundStrategies;

impl exports::fund::strategy::strategy::Guest for FundStrategies {
    fn init(config: String) {
        let mut state = STATE.lock().unwrap();
        let table: toml::Table = match toml::from_str(&config) {
            Ok(table) => table,
            Err(_) => {
                state.kind = None;
                return;
            }
        };
        let strategy = table.get("strategy").and_then(|v| v.as_str());
        state.kind = match strategy {
            Some("buy_hold") => {
                let amount = table.get("amount").and_then(as_f64);
                amount.map(|amount| StrategyKind::BuyHold {
                    amount,
                    invested: false,
                })
            }
            Some("dca") => {
                let amount = table.get("amount").and_then(as_f64);
                let interval = table.get("interval").and_then(|v| v.as_integer());
                match (amount, interval) {
                    (Some(amount), Some(interval)) => Some(StrategyKind::Dca {
                        amount,
                        interval: interval as u64,
                        day: 0,
                    }),
                    _ => None,
                }
            }
            _ => None,
        };
    }

    fn on_event(event: Event) -> Vec<Order> {
        let mut state = STATE.lock().unwrap();
        match &mut state.kind {
            Some(StrategyKind::BuyHold { amount, invested }) => {
                if !*invested && let Event::NavUpdate(_) = event {
                    *invested = true;
                    return vec![Order::Invest(*amount)];
                }
                Vec::new()
            }
            Some(StrategyKind::Dca {
                amount,
                interval,
                day,
            }) => {
                if let Event::NavUpdate(_) = event {
                    let buy = *day == 0 || day.is_multiple_of(*interval);
                    *day += 1;
                    if buy {
                        return vec![Order::Invest(*amount)];
                    }
                }
                Vec::new()
            }
            None => Vec::new(),
        }
    }
}

export!(FundStrategies);
