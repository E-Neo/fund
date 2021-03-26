use crate::{
    error::Result,
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
    ) -> Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let mut repository = Repository::new(rule, net_asset_value_history)?;
        listener.accept().and_then(|(stream, _)| {
            let reader = BufReader::new(stream.try_clone()?);
            let mut writer = BufWriter::new(stream);
            let (date, nav) = repository.check().unwrap();
            writeln!(&mut writer, "+{} {}", date, nav)?;
            writer.flush()?;
            for line in reader.lines() {
                let line = &line?;
                if let Some(res) = pass(&mut repository, line)
                    .or(invest(&mut repository, line))
                    .or(redeem(&mut repository, line))
                {
                    match res {
                        Ok(()) => match repository.check() {
                            Ok((date, nav)) => {
                                writeln!(&mut writer, "+{} {}", date, nav)?;
                                writer.flush()?;
                            }
                            Err(err) => {
                                writeln!(&mut writer, "-{}", err)?;
                                writer.flush()?;
                                break;
                            }
                        },
                        Err(err) => {
                            writeln!(&mut writer, "-{}", err)?;
                            writer.flush()?;
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

fn pass(repository: &mut Repository, line: &str) -> Option<Result<()>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^p$").unwrap();
    }
    RE.captures(line).map(|_| repository.pass())
}

fn invest(repository: &mut Repository, line: &str) -> Option<Result<()>> {
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

fn redeem(repository: &mut Repository, line: &str) -> Option<Result<()>> {
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
