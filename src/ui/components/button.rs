use iced::{
    Theme,
    widget::{self, Button, button},
};

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
    Nav,
    NavActive,
}

pub fn action_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
) -> Button<'a, Message> {
    widget::button(widget::text(label.into()).size(15))
        .padding(theme::SPACE_4)
        .style(style_for(kind))
}

pub fn compact_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
) -> Button<'a, Message> {
    widget::button(widget::text(label.into()).size(14))
        .padding(theme::SPACE_3)
        .style(style_for(kind))
}

pub fn nav_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    active: bool,
) -> Button<'a, Message> {
    let kind = if active {
        ButtonKind::NavActive
    } else {
        ButtonKind::Nav
    };

    widget::button(widget::text(label.into()).size(15))
        .padding([theme::SPACE_3, theme::SPACE_4])
        .width(iced::Length::Fill)
        .style(style_for(kind))
}

fn style_for(kind: ButtonKind) -> fn(&Theme, button::Status) -> button::Style {
    match kind {
        ButtonKind::Primary => theme::button_primary,
        ButtonKind::Secondary => theme::button_secondary,
        ButtonKind::Ghost => theme::button_ghost,
        ButtonKind::Danger => theme::button_danger,
        ButtonKind::Nav => theme::button_nav,
        ButtonKind::NavActive => theme::button_nav_active,
    }
}
