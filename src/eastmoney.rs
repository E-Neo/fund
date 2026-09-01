use crate::error::{Error, Result};
use chrono::{FixedOffset, NaiveDate, TimeZone};
use serde::Deserialize;
use std::collections::HashMap;

const EASTMONEY_URL: &str = "https://fund.eastmoney.com/pingzhongdata/{code}.js";
const BEIJING_OFFSET: i32 = 8 * 3600;

#[derive(Debug, Clone)]
pub struct Nav {
    pub date: NaiveDate,
    pub unit_nav: f64,
    pub accum_nav: f64,
    pub daily_return: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Fund {
    pub code: String,
    pub name: String,
    pub navs: Vec<Nav>,
}

#[derive(Debug, Deserialize)]
struct NetWorthPoint {
    x: i64,
    y: f64,
    #[serde(rename = "equityReturn")]
    equity_return: f64,
}

#[derive(Debug, Deserialize)]
struct AccWorthPoint {
    x: i64,
    y: f64,
}

fn is_valid_code(code: &str) -> bool {
    code.len() == 6 && code.chars().all(|c| c.is_ascii_digit())
}

pub async fn fetch_fund(code: &str) -> Result<Fund> {
    if !is_valid_code(code) {
        return Err(Error::InvalidCode(code.to_string()));
    }
    let url = EASTMONEY_URL.replace("{code}", code);
    let body = reqwest::get(&url).await?.text().await?;
    parse_js(&body, code)
}

fn parse_js(js: &str, code: &str) -> Result<Fund> {
    let name = extract_string(js, "fS_name")?;
    let net_worth = extract_json::<Vec<NetWorthPoint>>(js, "Data_netWorthTrend")?;
    let acc_worth = extract_json::<Vec<AccWorthPoint>>(js, "Data_ACWorthTrend")?;

    let mut accum_by_date: HashMap<NaiveDate, f64> = HashMap::new();
    for point in acc_worth {
        accum_by_date.insert(epoch_to_date(point.x), point.y);
    }

    let mut navs = net_worth
        .into_iter()
        .map(|point| {
            let date = epoch_to_date(point.x);
            let accum_nav = accum_by_date.get(&date).copied().unwrap_or(point.y);
            Nav {
                date,
                unit_nav: point.y,
                accum_nav,
                daily_return: Some(point.equity_return),
            }
        })
        .collect::<Vec<_>>();
    navs.sort_by_key(|nav| nav.date);
    Ok(Fund {
        code: code.to_string(),
        name,
        navs,
    })
}

fn extract_string(js: &str, var: &str) -> Result<String> {
    let marker = format!("{var} = ");
    let start = js
        .find(&marker)
        .ok_or_else(|| Error::Parse(format!("missing {var}")))?;
    let rest = &js[start + marker.len()..];
    let quote_start = rest
        .find('"')
        .ok_or_else(|| Error::Parse(format!("missing value for {var}")))?;
    let value = &rest[quote_start + 1..];
    let quote_end = value
        .find('"')
        .ok_or_else(|| Error::Parse(format!("unterminated value for {var}")))?;
    Ok(value[..quote_end].to_string())
}

fn extract_json<'a, T: Deserialize<'a>>(js: &'a str, var: &str) -> Result<T> {
    let marker = format!("{var} = ");
    let start = js
        .find(&marker)
        .ok_or_else(|| Error::Parse(format!("missing {var}")))?;
    let value = &js[start + marker.len()..];
    let end = value
        .find(';')
        .ok_or_else(|| Error::Parse(format!("unterminated value for {var}")))?;
    serde_json::from_str(&value[..end]).map_err(|err| Error::Parse(err.to_string()))
}

fn epoch_to_date(ms: i64) -> NaiveDate {
    let offset = FixedOffset::east_opt(BEIJING_OFFSET).expect("valid fixed offset");
    offset
        .timestamp_millis_opt(ms)
        .single()
        .expect("valid timestamp")
        .date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"var fS_name = "Sample Fund";var fS_code = "000001";
Data_netWorthTrend = [{"x":1282233600000,"y":1.0,"equityReturn":0,"unitMoney":""},{"x":1282838400000,"y":1.001,"equityReturn":0.1,"unitMoney":""}];
Data_ACWorthTrend = [[1282233600000,1.0],[1282838400000,1.001]];"#;

    #[test]
    fn test_parse_js() {
        let fund = parse_js(SAMPLE, "000001").unwrap();
        assert_eq!(fund.name, "Sample Fund");
        assert_eq!(fund.code, "000001");
        assert_eq!(fund.navs.len(), 2);
        assert_eq!(
            fund.navs[0].date,
            NaiveDate::from_ymd_opt(2010, 8, 20).unwrap()
        );
        assert_eq!(fund.navs[0].unit_nav, 1.0);
        assert_eq!(
            fund.navs[1].date,
            NaiveDate::from_ymd_opt(2010, 8, 27).unwrap()
        );
        assert_eq!(fund.navs[1].unit_nav, 1.001);
    }

    #[test]
    fn test_invalid_code() {
        assert!(!is_valid_code("abc"));
        assert!(!is_valid_code("12345"));
        assert!(!is_valid_code("12345a"));
        assert!(is_valid_code("123456"));
    }
}
