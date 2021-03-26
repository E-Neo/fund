use crate::{
    error::{Error, Result},
    repository::{Repository, Rule},
};
use chrono::NaiveDate;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    net::TcpListener,
};

pub struct Server;

impl Server {
    pub fn run(
        rule: Box<dyn Rule>,
        net_asset_value_history: Vec<(NaiveDate, f64)>,
        port: u16,
    ) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let (mut repository, (date, nav)) =
            Repository::start(rule, net_asset_value_history).unwrap();
        listener.accept().and_then(|(stream, _)| {
            let reader = BufReader::new(stream.try_clone()?);
            let mut writer = BufWriter::new(stream);
            writeln!(&mut writer, "+{} {}", date, nav)?;
            writer.flush()?;
            for line in reader.lines() {
                let line = &line?;
                if let Some(res) = pass(&mut repository, line)
                    .or(invest(&mut repository, line))
                    .or(redeem(&mut repository, line))
                {
                    match res {
                        Ok((date, nav)) => {
                            writeln!(&mut writer, "+{} {}", date, nav)?;
                            writer.flush()?;
                        }
                        Err(err) => {
                            writeln!(&mut writer, "-{}", err)?;
                            writer.flush()?;
                            if err == Error::Overflow {
                                break;
                            }
                        }
                    }
                } else {
                    writeln!(&mut writer, "-Invalid")?;
                    writer.flush()?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }
}

fn pass(repository: &mut Repository, line: &str) -> Option<Result<(NaiveDate, f64)>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^p$").unwrap();
    }
    RE.captures(line).map(|_| repository.pass())
}

fn invest(repository: &mut Repository, line: &str) -> Option<Result<(NaiveDate, f64)>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^i(\S+)$").unwrap();
    }
    if let Some(investment) = RE
        .captures(line)
        .and_then(|caps| caps.get(1).and_then(|x| x.as_str().parse().ok()))
    {
        Some(repository.invest(investment))
    } else {
        None
    }
}

fn redeem(repository: &mut Repository, line: &str) -> Option<Result<(NaiveDate, f64)>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^r(\S+)$").unwrap();
    }
    if let Some(redemption) = RE
        .captures(line)
        .and_then(|caps| caps.get(1).and_then(|x| x.as_str().parse().ok()))
    {
        Some(repository.redeem(redemption))
    } else {
        None
    }
}
