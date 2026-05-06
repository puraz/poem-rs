use iced::{
    Element, Length,
    widget::{self, Container},
};

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Base,
    Raised,
    Accent,
    Outline,
    Appreciation,
}

pub fn page_shell<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    widget::container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::page_background)
}

pub fn shell_surface<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    surface(content, SurfaceKind::Raised)
}

pub fn nav_surface<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    widget::container(content)
        .width(Length::Fill)
        .padding(theme::SPACE_5)
        .style(theme::sidebar_panel)
}

pub fn surface<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    kind: SurfaceKind,
) -> Container<'a, Message> {
    widget::container(content)
        .width(Length::Fill)
        .padding(theme::SPACE_5)
        .style(style_for(kind))
}

pub fn section_surface<'a, Message: 'a>(
    label: impl Into<String>,
    content: impl Into<Element<'a, Message>>,
    kind: SurfaceKind,
) -> Element<'a, Message> {
    let label = widget::container(widget::text(label.into()).size(12)).style(theme::eyebrow_text);

    widget::Column::new()
        .spacing(f32::from(theme::SPACE_3))
        .push(label)
        .push(surface(content, kind))
        .into()
}

fn style_for(kind: SurfaceKind) -> fn(&iced::Theme) -> widget::container::Style {
    match kind {
        SurfaceKind::Base => theme::panel,
        SurfaceKind::Raised => theme::raised_panel,
        SurfaceKind::Accent => theme::accent_panel,
        SurfaceKind::Outline => theme::outline_panel,
        SurfaceKind::Appreciation => theme::appreciation_panel,
    }
}
