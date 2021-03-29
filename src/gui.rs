use crate::repository::Repository;
use iced::{Application, Column, Command, Element, Text};

#[derive(Debug, Clone)]
pub enum Message {
    OffsetChanged(usize),
}

pub struct Gui {
    repository: Repository,
    offset: usize,
}

impl Application for Gui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Repository;

    fn new(repository: Self::Flags) -> (Self, Command<Self::Message>) {
        let offset = repository.len() - 1;
        (Self { repository, offset }, Command::none())
    }

    fn title(&self) -> String {
        String::from("基金模拟交易")
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let (date, nav) = self.repository.net_asset_value_history()[self.offset];
        Column::new()
            .push(Text::new(format!("日期: {}", date)))
            .push(Text::new(format!("净值: {}", nav)))
            .into()
    }
}
