use iced::{
    Element, Length, Theme,
    widget::{self, mouse_area, opaque},
};

use crate::ui::assets;
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
    let close_icon = widget::svg(assets::svg_handle(assets::CLOSE))
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .style(|active_theme: &Theme, _status| widget::svg::Style {
            color: Some(theme::tokens(active_theme).title),
        });

    widget::Row::new()
        .align_y(iced::Alignment::Start)
        .push(modal_header(title, subtitle))
        .push(widget::Space::new().width(Length::Fill))
        .push(
            mouse_area(
                widget::container(close_icon)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .center_x(Length::Shrink)
                    .center_y(Length::Shrink),
            )
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
        .push(
            widget::container(header)
                .width(Length::Fill)
                .padding([theme::SPACE_6, theme::SPACE_6]),
        )
        .push(modal_divider())
        .push(
            widget::container(body)
                .width(Length::Fill)
                .padding([theme::SPACE_5, theme::SPACE_6]),
        );

    if let Some(footer) = footer {
        frame = frame.push(modal_divider()).push(
            widget::container(footer)
                .width(Length::Fill)
                .padding([theme::SPACE_5, theme::SPACE_6]),
        );
    }

    widget::container(frame)
        .width(Length::Fill)
        .max_width(860)
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

fn modal_divider<'a, Message: 'a>() -> Element<'a, Message> {
    widget::container(
        widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(1.0)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(1.0))
    .style(theme::content_divider)
    .into()
}
