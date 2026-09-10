use crate::rules::{OrderForFee, Rule};
use chrono::NaiveDate;
use std::sync::Mutex;

/// The platform-facing fee authority. The engine owns one and shares it with
/// the wasm host so strategies can ask "what would this cost?" without seeing
/// the pluggable `Rule` implementation.
pub struct FeeService {
    rule: Mutex<Box<dyn Rule>>,
    today: Mutex<(NaiveDate, f64)>,
}

impl FeeService {
    pub fn new(rule: Box<dyn Rule>, today: NaiveDate) -> Self {
        Self {
            rule: Mutex::new(rule),
            today: Mutex::new((today, 0.0)),
        }
    }

    /// Update the current trading day and its unit nav.
    pub fn set_today(&self, date: NaiveDate, unit_nav: f64) {
        *self.today.lock().unwrap() = (date, unit_nav);
    }

    /// Apply an order's fee, mutating the rule's lot state.
    pub fn apply(&self, order: OrderForFee) -> f64 {
        self.rule.lock().unwrap().fee(order)
    }

    /// The redemption fee for redeeming `shares` today, without mutating.
    pub fn redeem_fee(&self, shares: f64) -> f64 {
        let (date, unit_nav) = { *self.today.lock().unwrap() };
        self.rule
            .lock()
            .unwrap()
            .preview_redeem_fee(shares, date, unit_nav)
    }

    /// The subscription fee for investing `amount`.
    pub fn subscribe_fee(&self, amount: f64) -> f64 {
        self.rule.lock().unwrap().subscribe_fee(amount)
    }
}
