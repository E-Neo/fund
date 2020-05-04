use crate::{person, world};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub enum Message {
    Step(Sender<world::Message>),
    NetAssetValue(Sender<person::Message>),
    NextTradeDate(Sender<person::Message>),
}

pub struct Market {
    net_asset_values: Vec<(NaiveDate, Decimal)>,
    idx: usize,
    final_trade_date: NaiveDate,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Market {
    pub fn new(net_asset_values: Vec<(NaiveDate, Decimal)>, final_date: NaiveDate) -> Self {
        let (tx, rx) = channel(8);
        Self {
            net_asset_values,
            idx: 0,
            final_trade_date: final_date,
            tx,
            rx,
        }
    }

    pub fn channel(&self) -> Sender<Message> {
        self.tx.clone()
    }

    fn next_trade_date(&self) -> NaiveDate {
        if self.idx == self.net_asset_values.len() {
            self.net_asset_values[self.idx].0
        } else {
            self.final_trade_date
        }
    }

    fn net_asset_value(&self) -> Decimal {
        self.net_asset_values[self.idx - 1].1
    }

    async fn process_step(&mut self, mut world_channel: Sender<world::Message>) {
        let date = self.next_trade_date();
        self.idx += 1;
        if self.idx == self.net_asset_values.len() {
            let _ = world_channel.send(world::Message::MarketTerminated).await;
        } else {
            let _ = world_channel
                .send(world::Message::MarketStepped {
                    date,
                    net_asset_value: self.net_asset_value(),
                    next_date: self.next_trade_date(),
                })
                .await;
        }
    }

    async fn process_net_asset_value(&mut self, mut tx: Sender<person::Message>) {
        let _ = tx
            .send(person::Message::NetAssetValue(self.net_asset_value()))
            .await;
    }

    async fn process_next_trade_date(&mut self, mut person_channel: Sender<person::Message>) {
        let _ = person_channel
            .send(person::Message::NextTradeDate(self.next_trade_date()))
            .await;
    }

    pub async fn process(&mut self) {
        loop {
            match self.rx.recv().await.unwrap() {
                Message::Step(world_channel) => self.process_step(world_channel).await,
                Message::NetAssetValue(tx) => self.process_net_asset_value(tx).await,
                Message::NextTradeDate(person_channel) => {
                    self.process_next_trade_date(person_channel).await
                }
            }
        }
    }
}
