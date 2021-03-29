use chrono::NaiveDate;
use fund::{error::Result, gui::Gui, server::Server};
use iced::{Application, Settings};

fn main() -> Result<()> {
    Gui::run(Settings::with_flags(Server::run(
        Box::new(|_| 0.0),
        NaiveDate::from_ymd(2021, 1, 1)
            .iter_days()
            .enumerate()
            .take(5)
            .map(|(i, date)| (date, if i & 1 == 0 { 1.0 } else { 1.05 }))
            .collect(),
        8000,
    )?))
    .map_err(|err| err.into())
}
