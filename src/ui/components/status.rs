use iced::{Element, widget};

use crate::ui::theme::{self, Tone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

pub fn status_chip<'a, Message: 'a>(
    label: impl Into<String>,
    tone: StatusTone,
) -> Element<'a, Message> {
    widget::container(widget::text(label.into()).size(13))
        .padding([theme::SPACE_2, theme::SPACE_3])
        .style(move |active_theme| theme::chip_style(active_theme, theme_tone(tone)))
        .into()
}

fn theme_tone(tone: StatusTone) -> Tone {
    match tone {
        StatusTone::Neutral => Tone::Neutral,
        StatusTone::Info => Tone::Primary,
        StatusTone::Success => Tone::Success,
        StatusTone::Warning => Tone::Warning,
        StatusTone::Danger => Tone::Danger,
    }
}
