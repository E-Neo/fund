use chrono::NaiveDate;
use fund::server::Server;

fn main() -> std::io::Result<()> {
    Server::run(
        Box::new(|_| 0.0),
        NaiveDate::from_ymd(2021, 1, 1)
            .iter_days()
            .enumerate()
            .take(5)
            .map(|(i, date)| (date, if i & 1 == 0 { 1.0 } else { 1.05 }))
            .collect(),
        8000,
    )
}
