use crate::{bank, world};
use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub enum Message {
    Step {
        world_channel: Sender<world::Message>,
        date: NaiveDate,
        net_asset_value: Decimal,
        next_date: NaiveDate,
    },
    Buy {
        bank_channel: Sender<bank::Message>,
        amount: Decimal,
    },
    BankToRepositoryOk(u64),
    BankToRepositoryErr(u64),
    Sell {
        bank_channel: Sender<bank::Message>,
        share: Decimal,
    },
    RepositoryToBankOk(u64),
    RepositoryToBankErr(u64),
    Stop,
}

pub struct TransactionBuy {
    id: u64,
    finish_date: NaiveDate,
    amount: Decimal, // doesnot include fee
    share: Decimal,
}

pub struct TransactionSell {
    id: u64,
    finish_date: NaiveDate,
    amount: Decimal,
    bank_channel: Sender<bank::Message>,
}

struct QueueItem {
    date: NaiveDate,
    share: Decimal,
}

pub struct Repository {
    purchase_fee: Box<dyn Fn(&Repository, Decimal) -> Decimal>,
    redemption_fee: Box<dyn Fn(&Repository, Duration, Decimal) -> Decimal>,
    date: NaiveDate,
    net_asset_value: Decimal,
    next_date: NaiveDate,
    holding_share: Decimal,
    invested_money: Decimal,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    next_transaction_id: u64,
    purchase_queue: VecDeque<TransactionBuy>,
    redemption_queue: VecDeque<TransactionSell>,
    queue: VecDeque<QueueItem>,
}

impl Repository {
    pub fn new(
        purchase_fee: Box<dyn Fn(&Repository, Decimal) -> Decimal>,
        redemption_fee: Box<dyn Fn(&Repository, Duration, Decimal) -> Decimal>,
        date: NaiveDate,
        net_asset_value: Decimal,
        next_date: NaiveDate,
    ) -> Self {
        let (tx, rx) = channel(8);
        Self {
            purchase_fee,
            redemption_fee,
            date,
            net_asset_value,
            next_date,
            holding_share: Decimal::new(0, 0),
            invested_money: Decimal::new(0, 0),
            tx,
            rx,
            next_transaction_id: 0,
            purchase_queue: VecDeque::new(),
            redemption_queue: VecDeque::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn channel(&self) -> Sender<Message> {
        self.tx.clone()
    }

    fn get_transaction_id(&mut self) -> u64 {
        let id = self.next_transaction_id;
        self.next_transaction_id += 1;
        id
    }

    async fn process_step(
        &mut self,
        mut world_channel: Sender<world::Message>,
        date: NaiveDate,
        net_asset_value: Decimal,
        next_date: NaiveDate,
    ) {
        self.date = date;
        self.net_asset_value = net_asset_value;
        self.next_date = next_date;
        while let Some(ts) = self.redemption_queue.front() {
            if ts.finish_date > self.date {
                break;
            } else {
                let mut ts = self.redemption_queue.pop_front().unwrap();
                let _ = ts
                    .bank_channel
                    .send(bank::Message::RepositoryToBank(
                        self.channel(),
                        ts.id,
                        ts.amount,
                    ))
                    .await;
            }
        }
        let _ = world_channel.send(world::Message::RepositoryStepped).await;
    }

    async fn process_buy(&mut self, mut bank_channel: Sender<bank::Message>, amount: Decimal) {
        let id = self.get_transaction_id();
        let _ = bank_channel
            .send(bank::Message::BankToRepository(self.channel(), id, amount))
            .await;
        self.purchase_queue.push_back(TransactionBuy {
            id,
            finish_date: self.next_date,
            amount,
            share: (amount - (*self.purchase_fee)(self, amount)) / self.net_asset_value,
        });
    }

    async fn process_bank_to_repository_ok(&mut self, id: u64) {
        if let Some(idx) = self.purchase_queue.iter().position(|tb| tb.id == id) {
            let tb = self.purchase_queue.remove(idx).unwrap();
            self.holding_share += tb.share;
            self.invested_money += tb.amount;
            self.queue.push_back(QueueItem {
                date: tb.finish_date,
                share: tb.share,
            })
        }
    }

    async fn process_sell(&mut self, bank_channel: Sender<bank::Message>, share: Decimal) {
        if share <= self.holding_share {
            let mut tmp_share = share;
            let mut amount = Decimal::new(0, 0);
            let mut fee = Decimal::new(0, 0);
            while tmp_share != Decimal::new(0, 0) {
                let item = self.queue.pop_front().unwrap();
                if item.share <= tmp_share {
                    fee += (*self.redemption_fee)(self, self.next_date - item.date, item.share);
                    amount += item.share * self.net_asset_value - fee;
                    tmp_share -= item.share;
                } else {
                    self.queue.push_front(QueueItem {
                        date: item.date,
                        share: item.share - tmp_share,
                    });
                    fee += (*self.redemption_fee)(self, self.next_date - item.date, tmp_share);
                    amount += tmp_share * self.net_asset_value - fee;
                    tmp_share = Decimal::new(0, 0);
                }
            }
            let id = self.get_transaction_id();
            self.redemption_queue.push_back(TransactionSell {
                id,
                finish_date: self.next_date,
                amount,
                bank_channel,
            });
        }
    }

    pub async fn process(&mut self) {
        loop {
            match self.rx.recv().await.unwrap() {
                Message::Step {
                    world_channel,
                    date,
                    net_asset_value,
                    next_date,
                } => {
                    self.process_step(world_channel, date, net_asset_value, next_date)
                        .await
                }
                Message::Buy {
                    bank_channel,
                    amount,
                } => self.process_buy(bank_channel, amount).await,
                Message::BankToRepositoryOk(id) => self.process_bank_to_repository_ok(id).await,
                Message::BankToRepositoryErr(_) => (),
                Message::Sell {
                    bank_channel,
                    share,
                } => self.process_sell(bank_channel, share).await,
                Message::RepositoryToBankOk(_) => (),
                Message::RepositoryToBankErr(_) => (),
                Message::Stop => break,
            }
        }
    }
}
