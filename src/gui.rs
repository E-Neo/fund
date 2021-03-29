use crate::repository::Repository;
use iced::{slider, Application, Column, Command, Element, Font, Slider, Text};

const CHINESE_FONT: Font = Font::External {
    name: "null",
    bytes: include_bytes!(concat!(env!("OUT_DIR"), "/chinese_font")),
};

#[derive(Debug, Clone)]
pub enum Message {
    OffsetChanged(usize),
}

pub struct Gui {
    repository: Repository,
    offset: usize,
    slider_state: slider::State,
}

impl Gui {
    fn text<T: Into<String>>(label: T) -> Text {
        Text::new(label).font(CHINESE_FONT)
    }
}

impl Application for Gui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Repository;

    fn new(repository: Self::Flags) -> (Self, Command<Self::Message>) {
        let offset = repository.len() - 1;
        (
            Self {
                repository,
                offset,
                slider_state: slider::State::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("基金模拟交易")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::OffsetChanged(offset) => {
                self.offset = offset;
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Self::Message> {
        let slider = Slider::new(
            &mut self.slider_state,
            0.0..=self.repository.len() as f64 - 1.0,
            self.offset as f64,
            |x| Message::OffsetChanged(x as usize),
        );
        let (date, nav) = self.repository.net_asset_value_history()[self.offset];
        let info = &self.repository.daily_infos()[self.offset];
        Column::new()
            .push(Self::text(format!("日期: {}", date)))
            .push(Self::text(format!("净值: {:.4}", nav)))
            .push(Self::text(format!(
                "持仓成本价: {:.4}",
                info.holding_price()
            )))
            .push(Self::text(format!("持有份额: {:.2}", info.holding_share())))
            .push(Self::text(format!(
                "持有金额: {:.2}",
                info.holding_price() * info.holding_share()
            )))
            .push(Self::text(format!(
                "累计收益: {:.2}",
                info.holding_price() * info.holding_share() + info.cumulative_redemption()
                    - info.cumulative_investment()
            )))
            .push(slider)
            .into()
    }
}
