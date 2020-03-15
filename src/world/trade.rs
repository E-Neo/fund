use chrono::NaiveDate;

pub enum Trade {
    Buy {
        date: NaiveDate,
        price: f64,
        cost: f64,
    },
    Sell {
        date: NaiveDate,
        price: f64,
        share: f64,
    },
}

impl Trade {
    pub fn buy(date: NaiveDate, price: f64, cost: f64) -> Self {
        Trade::Buy { date, price, cost }
    }

    pub fn sell(date: NaiveDate, price: f64, share: f64) -> Self {
        Trade::Sell { date, price, share }
    }
    pub fn date(&self) -> NaiveDate {
        match self {
            &Trade::Buy {
                date: d,
                price: _,
                cost: _,
            } => d,
            &Trade::Sell {
                date: d,
                price: _,
                share: _,
            } => d,
        }
    }

    pub fn price(&self) -> f64 {
        match self {
            &Trade::Buy {
                date: _,
                price: v,
                cost: _,
            } => v,
            &Trade::Sell {
                date: _,
                price: v,
                share: _,
            } => v,
        }
    }

    pub fn share(&self) -> f64 {
        match self {
            &Trade::Buy {
                date: _,
                price: v,
                cost: c,
            } => c / v,
            &Trade::Sell {
                date: _,
                price: _,
                share: s,
            } => -s,
        }
    }

    pub fn money(&self) -> f64 {
        match self {
            &Trade::Buy {
                date: _,
                price: _,
                cost: c,
            } => c,
            &Trade::Sell {
                date: _,
                price: v,
                share: s,
            } => -v * s,
        }
    }
}
