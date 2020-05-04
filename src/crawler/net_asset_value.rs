use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;
use soup::prelude::*;

#[derive(Deserialize)]
struct ApiData {
    content: String,
    records: u32,
    pages: u32,
    curpage: u32,
}

async fn get_page(
    code: &str,
    start: &str,
    end: &str,
    page: u32,
) -> Result<(Vec<(NaiveDate, Decimal)>, bool), Box<dyn std::error::Error>> {
    let body = reqwest::get(&format!(
        "http://fund.eastmoney.com/f10/F10DataApi.aspx?\
         type=lsjz&code={}&page={}&per=20&sdate={}&edate={}",
        code, page, start, end
    ))
    .await?
    .text()
    .await?;
    let data: ApiData = json5::from_str(&body[12..body.len() - 1])?;
    let soup = Soup::new(&data.content);
    let mut navs = Vec::with_capacity(data.records as usize);
    for tr in soup
        .tag("tbody")
        .find()
        .ok_or("No tbody")?
        .tag("tr")
        .find_all()
    {
        let mut tds = tr.tag("td").find_all();
        navs.push((
            tds.next().ok_or("No date")?.text().parse()?,
            tds.next().ok_or("No value")?.text().parse()?,
        ));
    }
    Ok((navs, data.curpage < data.pages))
}

pub async fn get_history(
    code: &str,
    start: &str,
    end: &str,
) -> Result<Vec<(NaiveDate, Decimal)>, Box<dyn std::error::Error>> {
    let end = &(end.parse::<NaiveDate>()? - Duration::days(1)).to_string();
    let mut history = vec![];
    let mut page = 1;
    let (mut navs, mut have_next) = get_page(code, start, end, page).await?;
    history.append(&mut navs);
    while have_next {
        page += 1;
        let (mut navs, next_flag) = get_page(code, start, end, page).await?;
        history.append(&mut navs);
        have_next = next_flag;
    }
    history.sort_by_key(|x| x.0);
    Ok(history)
}
