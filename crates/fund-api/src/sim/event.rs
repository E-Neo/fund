use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub enum Order {
    Invest { amount: f64 },
    Redeem { shares: f64 },
}

#[derive(Debug, Clone)]
pub enum TransactionKind {
    Invest { amount: f64, shares: f64, fee: f64 },
    Redeem { shares: f64, money: f64, fee: f64 },
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub date: NaiveDate,
    pub unit_nav: f64,
    pub kind: TransactionKind,
}

#[derive(Debug, Clone)]
pub enum Event {
    DayStart(NaiveDate),
    OrderExecuted {
        date: NaiveDate,
        transaction: Transaction,
    },
    NavUpdate {
        date: NaiveDate,
        unit_nav: f64,
        accum_nav: f64,
    },
    DayEnd(NaiveDate),
}
