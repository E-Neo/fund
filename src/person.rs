use chrono::NaiveDate;
use rust_decimal::Decimal;

pub enum Message {
    Balance(Decimal),
    NetAssetValue(Decimal),
    NextTradeDate(NaiveDate),
}

pub struct Person {}
