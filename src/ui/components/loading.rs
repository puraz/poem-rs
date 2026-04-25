use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

pub fn loading_indicator<'a, Message: 'a>(
    label: impl Into<String>,
    frame: usize,
) -> Element<'a, Message> {
    const FRAMES: [&str; 4] = ["   ", ".  ", ".. ", "..."];

    row![
        text(label.into()).size(15),
        container(text(FRAMES[frame % FRAMES.len()]).size(15))
            .width(Length::Fixed(22.0))
            .align_x(Alignment::Start),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .into()
}
