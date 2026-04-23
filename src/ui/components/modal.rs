use iced::{
    Element, Length,
    widget::{self, mouse_area, opaque},
};

use super::button::{ButtonKind, compact_button};
use crate::ui::theme;

pub fn modal_header<'a, Message: 'a>(
    title: impl Into<String>,
    subtitle: Option<&'a str>,
) -> Element<'a, Message> {
    let mut text_stack = widget::Column::new()
        .spacing(f32::from(theme::SPACE_2))
        .push(widget::text(title.into()).size(28));

    if let Some(subtitle) = subtitle {
        text_stack = text_stack
            .push(widget::container(widget::text(subtitle).size(15)).style(theme::subdued_text));
    }

    text_stack.into()
}

pub fn modal_header_with_close<'a, Message: Clone + 'a>(
    title: impl Into<String>,
    subtitle: Option<&'a str>,
    close_message: Message,
) -> Element<'a, Message> {
    widget::Row::new()
        .align_y(iced::Alignment::Start)
        .push(modal_header(title, subtitle))
        .push(widget::Space::new().width(Length::Fill))
        .push(
            compact_button("关闭", ButtonKind::Secondary)
                .width(112)
                .on_press(close_message),
        )
        .into()
}

pub fn modal_frame<'a, Message: 'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut frame = widget::Column::new()
        .spacing(f32::from(theme::SPACE_5))
        .push(header)
        .push(body);

    if let Some(footer) = footer {
        frame = frame.push(widget::container(footer).padding(theme::SPACE_1));
    }

    widget::container(frame)
        .width(Length::Fill)
        .max_width(820)
        .padding(theme::SPACE_6)
        .style(theme::modal_frame)
        .into()
}

pub fn modal_overlay<'a, Message: Clone + 'a>(
    base: impl Into<Element<'a, Message>>,
    modal: impl Into<Element<'a, Message>>,
    on_backdrop_press: Option<Message>,
) -> Element<'a, Message> {
    let modal = widget::container(opaque(modal))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::SPACE_6);

    let overlay = match on_backdrop_press {
        Some(message) => mouse_area(
            widget::container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::modal_backdrop),
        )
        .on_press(message)
        .into(),
        None => widget::container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::modal_backdrop)
            .into(),
    };

    widget::Stack::with_children(vec![base.into(), overlay]).into()
}
