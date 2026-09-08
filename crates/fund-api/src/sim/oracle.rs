use crate::{
    eastmoney::Nav,
    fees::FeeRule,
    rules::{Fifo, OrderForFee, Rule},
    sim::{
        event::{Event, Order},
        strategy::{SimContext, Strategy},
    },
};
use chrono::NaiveDate;

/// A virtual strategy that can see the full history and the future of a fund.
///
/// It is not a real, implementable strategy: it knows every future NAV and
/// greedily buys at each local minimum and sells at the following local
/// maximum, capturing every profitable swing while taking transaction costs
/// into account. It is implemented natively (not as a wasm component) because
/// the guest interface cannot see the future.
pub struct Oracle {
    /// Ordered plan of orders to emit, keyed by the NavUpdate date on which
    /// the order must be placed (orders settle at the next day's nav, T+1).
    plan: Vec<(NaiveDate, Order)>,
    cursor: usize,
}

impl Oracle {
    pub fn new(navs: &[Nav], fee_rule: &FeeRule, capital: f64) -> Self {
        Self {
            plan: build_plan(navs, fee_rule, capital),
            cursor: 0,
        }
    }
}

impl Strategy for Oracle {
    fn name(&self) -> &str {
        "Oracle"
    }

    fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
        let Event::NavUpdate { date, .. } = event else {
            return Vec::new();
        };
        if self.cursor >= self.plan.len() {
            return Vec::new();
        }
        if self.plan[self.cursor].0 != *date {
            return Vec::new();
        }
        self.cursor += 1;
        vec![self.plan[self.cursor - 1].1.clone()]
    }
}

/// Build the optimal buy/sell plan with a single forward pass over the navs.
///
/// A buy is placed on the NavUpdate of day `b - 1` and settles at `navs[b]`
/// (T+1), so the effective tradeable series is `navs[1..]`. The oracle buys
/// all-in at each local minimum and sells everything at the following local
/// maximum. A segment is kept only if its net proceeds after subscription and
/// redemption fees exceed the cash it started with — transaction costs never
/// turn a winning trade into a losing one.
fn build_plan(navs: &[Nav], fee_rule: &FeeRule, capital: f64) -> Vec<(NaiveDate, Order)> {
    let n = navs.len();
    let mut plan = Vec::new();
    if n < 3 || capital <= 0.0 {
        return plan;
    }
    let mut cash = capital;
    let mut buy = 1usize;
    while buy + 1 < n {
        // Advance to a local minimum of the effective series (start of a rise).
        while buy + 1 < n && navs[buy + 1].unit_nav <= navs[buy].unit_nav {
            buy += 1;
        }
        if buy + 1 >= n {
            break;
        }
        // Advance to the following local maximum (end of the rise).
        let mut sell = buy + 1;
        while sell + 1 < n && navs[sell + 1].unit_nav >= navs[sell].unit_nav {
            sell += 1;
        }
        let (amount, shares, net) = net_cash(navs, fee_rule, buy, sell, cash);
        if net > cash {
            // Order placed on day buy-1 settles at navs[buy]; likewise for sell.
            plan.push((navs[buy - 1].date, Order::Invest { amount }));
            plan.push((navs[sell - 1].date, Order::Redeem { shares }));
            cash = net;
        }
        buy = sell + 1;
    }
    plan
}

/// Returns (amount, shares, net_cash) for a buy at `buy` and a sell at `sell`.
fn net_cash(
    navs: &[Nav],
    fee_rule: &FeeRule,
    buy: usize,
    sell: usize,
    cash: f64,
) -> (f64, f64, f64) {
    let mut fifo = Fifo::new(fee_rule.subscribe.clone(), fee_rule.redeem.clone());
    let sub_fee = fifo.fee(OrderForFee::Invest {
        date: navs[buy].date,
        unit_nav: navs[buy].unit_nav,
        amount: cash,
    });
    let shares = (cash - sub_fee) / navs[buy].unit_nav;
    let red_fee = fifo.fee(OrderForFee::Redeem {
        date: navs[sell].date,
        unit_nav: navs[sell].unit_nav,
        shares,
    });
    let net = shares * navs[sell].unit_nav - red_fee;
    (cash, shares, net)
}
