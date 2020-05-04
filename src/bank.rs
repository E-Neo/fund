use crate::{person, repository, world};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub enum Message {
    Step(Sender<world::Message>, NaiveDate),
    Balance(Sender<person::Message>),
    Deposit(Decimal),
    BankToRepository(Sender<repository::Message>, u64, Decimal),
    RepositoryToBank(Sender<repository::Message>, u64, Decimal),
    Stop,
}

pub struct Bank {
    date: NaiveDate,
    balance: Decimal,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Bank {
    pub fn new(date: NaiveDate, balance: Decimal) -> Self {
        let (tx, rx) = channel(8);
        Self {
            date,
            balance,
            tx,
            rx,
        }
    }

    pub fn channel(&self) -> Sender<Message> {
        self.tx.clone()
    }

    async fn process_step(&mut self, mut world_channel: Sender<world::Message>, date: NaiveDate) {
        self.date = date;
        let _ = world_channel.send(world::Message::BankStepped).await;
    }

    async fn process_balance(&mut self, mut person_channel: Sender<person::Message>) {
        let _ = person_channel
            .send(person::Message::Balance(self.balance))
            .await;
    }

    async fn process_deposit(&mut self, amount: Decimal) {
        self.balance += amount;
    }

    async fn process_bank_to_repository(
        &mut self,
        mut repository_channel: Sender<repository::Message>,
        id: u64,
        amount: Decimal,
    ) {
        if self.balance >= amount {
            let _ = repository_channel
                .send(repository::Message::BankToRepositoryOk(id))
                .await;
        } else {
            let _ = repository_channel
                .send(repository::Message::BankToRepositoryErr(id))
                .await;
        }
    }

    async fn process_repository_to_bank(
        &mut self,
        mut repository_channel: Sender<repository::Message>,
        id: u64,
        amount: Decimal,
    ) {
        self.balance += amount;
        let _ = repository_channel
            .send(repository::Message::RepositoryToBankOk(id))
            .await;
    }

    pub async fn process(&mut self) {
        loop {
            match self.rx.recv().await.unwrap() {
                Message::Step(world_channel, date) => self.process_step(world_channel, date).await,
                Message::Balance(person_channel) => self.process_balance(person_channel).await,
                Message::Deposit(amount) => self.process_deposit(amount).await,
                Message::BankToRepository(repository_channel, id, amount) => {
                    self.process_bank_to_repository(repository_channel, id, amount)
                        .await
                }
                Message::RepositoryToBank(repository_channel, id, amount) => {
                    self.process_repository_to_bank(repository_channel, id, amount)
                        .await
                }
                Message::Stop => break,
            }
        }
    }
}
