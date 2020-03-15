use chrono::NaiveDate;

pub struct Market {
    nav_history: Vec<(NaiveDate, f64)>,
    step: usize,
}

impl Market {
    pub fn new(nav_history: Vec<(NaiveDate, f64)>) -> Self {
        Market {
            nav_history,
            step: 0,
        }
    }

    pub fn nav_history(&self) -> &[(NaiveDate, f64)] {
        &self.nav_history.as_slice()[..self.step]
    }
}

impl Iterator for Market {
    type Item = (NaiveDate, f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.step == self.nav_history.len() {
            None
        } else {
            let item = self.nav_history[self.step];
            self.step += 1;
            Some(item)
        }
    }
}
