use iced::{
    Element, Length,
    widget::{self, container},
};

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    Info,
    Success,
    Warning,
    Danger,
}

pub fn toast<'a, Message: 'a>(
    title: Option<&'a str>,
    message: impl Into<String>,
    tone: ToastTone,
) -> Element<'a, Message> {
    let mut body = widget::Column::new().spacing(f32::from(theme::SPACE_1));

    if let Some(title) = title {
        body = body.push(widget::text(title).size(14));
    }

    body = body.push(
        container(widget::text(message.into()).size(14))
            .style(theme::subdued_text)
            .width(Length::Shrink),
    );

    let accent = widget::container(widget::Space::new().width(4))
        .height(Length::Fill)
        .style(accent_style_for(tone));

    widget::container(
        widget::Row::new()
            .spacing(f32::from(theme::SPACE_3))
            .push(accent)
            .push(widget::container(body).width(Length::Fill)),
    )
    .width(Length::Shrink)
    .max_width(420)
    .padding(theme::SPACE_4)
    .style(theme::toast_surface)
    .into()
}

pub fn toast_host<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    toast: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let Some(toast) = toast else {
        return content.into();
    };

    let overlay = widget::container(
        widget::Column::new()
            .width(Length::Fill)
            .align_x(iced::Alignment::End)
            .push(toast),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::SPACE_5);

    widget::Stack::with_children(vec![content.into(), overlay.into()]).into()
}

fn accent_style_for(tone: ToastTone) -> fn(&iced::Theme) -> container::Style {
    match tone {
        ToastTone::Info => accent_info,
        ToastTone::Success => accent_success,
        ToastTone::Warning => accent_warning,
        ToastTone::Danger => accent_danger,
    }
}

fn accent_info(_theme: &iced::Theme) -> container::Style {
    accent_style(theme::ACCENT)
}

fn accent_success(_theme: &iced::Theme) -> container::Style {
    accent_style(theme::SUCCESS)
}

fn accent_warning(_theme: &iced::Theme) -> container::Style {
    accent_style(theme::WARNING)
}

fn accent_danger(_theme: &iced::Theme) -> container::Style {
    accent_style(theme::DANGER)
}

fn accent_style(color: iced::Color) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(color.into()),
        border: iced::Border {
            color,
            width: 0.0,
            radius: theme::RADIUS_SMALL.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}
