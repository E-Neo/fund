use crate::{bank, market, repository};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::sync::mpsc::{channel, Receiver, Sender};

const COUNT_TO_STEP: usize = 3;

pub enum Message {
    MarketStepped {
        date: NaiveDate,
        net_asset_value: Decimal,
        next_date: NaiveDate,
    },
    BankStepped,
    RepositoryStepped,
    MarketTerminated,
}

pub struct World {
    count: usize,
    bank_channel: Sender<bank::Message>,
    market_channel: Sender<market::Message>,
    repository_channel: Sender<repository::Message>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl World {
    pub fn new(
        bank_channel: Sender<bank::Message>,
        market_channel: Sender<market::Message>,
        repository_channel: Sender<repository::Message>,
    ) -> Self {
        let (tx, rx) = channel(8);
        Self {
            count: 0,
            bank_channel,
            market_channel,
            repository_channel,
            tx,
            rx,
        }
    }

    async fn step_market(&mut self) {
        let _ = self
            .market_channel
            .send(market::Message::Step(self.tx.clone()))
            .await;
    }

    async fn process_market_stepped(
        &mut self,
        date: NaiveDate,
        net_asset_value: Decimal,
        next_date: NaiveDate,
    ) {
        self.count += 1;
        let _ = self
            .bank_channel
            .send(bank::Message::Step(self.tx.clone(), date))
            .await;
        let _ = self
            .repository_channel
            .send(repository::Message::Step {
                world_channel: self.tx.clone(),
                date,
                net_asset_value,
                next_date,
            })
            .await;
    }

    async fn preprocess_market_terminated(&mut self) {
        let _ = self.bank_channel.send(bank::Message::Stop).await;
        let _ = self
            .repository_channel
            .send(repository::Message::Stop)
            .await;
    }

    pub async fn process(&mut self) {
        self.step_market().await;
        loop {
            match self.rx.recv().await.unwrap() {
                Message::MarketStepped {
                    date,
                    net_asset_value,
                    next_date,
                } => {
                    self.process_market_stepped(date, net_asset_value, next_date)
                        .await
                }
                Message::BankStepped => self.count += 1,
                Message::RepositoryStepped => self.count += 1,
                Message::MarketTerminated => {
                    self.preprocess_market_terminated().await;
                    break;
                }
            }
            if self.count == COUNT_TO_STEP {
                self.step_market().await;
            }
        }
    }
}
