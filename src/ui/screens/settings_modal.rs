use iced::widget::{Space, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::ui::components::{
    ButtonKind, compact_button, field_input, input_block, modal_frame, modal_header_with_close,
    secure_field_input,
};
use crate::ui::message::Message;
use crate::ui::state::SettingsForm;
use crate::ui::theme;

pub fn view<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let mut content = column![
        status_row(form.mode_label.as_str()),
        input_block(
            "Base URL",
            field_input("https://api.openai.com/v1", &form.base_url)
                .on_input(Message::SettingsBaseUrlChanged),
        ),
        input_block(
            "Model",
            field_input("gpt-4.1-mini", &form.model).on_input(Message::SettingsModelChanged),
        ),
        input_block(
            "API Key",
            secure_field_input("输入 API Key", form.api_key_input_value())
                .on_input(Message::SettingsApiKeyChanged),
        ),
        fallback_section(form.allow_file_fallback),
    ]
    .spacing(20);

    if !form.warning.trim().is_empty() {
        content = content.push(warning_block(form.warning.as_str()));
    }

    modal_frame(
        modal_header_with_close("AI 设置", Some(""), Message::CloseModal),
        scrollable(content)
            .direction(theme::scrollable_direction())
            .style(theme::scrollable_style)
            .height(Length::Shrink),
        Some(
            row![
                Space::new().width(Length::Fill),
                compact_button("清除 Key", ButtonKind::DangerGhost)
                    .width(116)
                    .on_press(Message::ClearApiKey),
                compact_button("保存设置", ButtonKind::Primary)
                    .width(132)
                    .on_press(Message::SaveSettings),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
        ),
    )
}

fn status_row<'a>(mode_label: &'a str) -> Element<'a, Message> {
    let accent = status_accent(mode_label);

    row![
        container(text("●").size(14)).style(move |_theme: &Theme| iced::widget::container::Style {
            text_color: Some(accent),
            ..Default::default()
        }),
        text("当前模式：").size(15),
        container(text(mode_label).size(15)).style(move |_theme: &Theme| {
            iced::widget::container::Style {
                text_color: Some(accent),
                ..Default::default()
            }
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn fallback_section<'a>(allow_file_fallback: bool) -> Element<'a, Message> {
    container(
        column![
            checkbox(allow_file_fallback)
                .label("允许文件回退存储")
                .size(20)
                .text_size(16)
                .spacing(10)
                .on_toggle(Message::SettingsFallbackChanged),
            row![
                Space::new().width(Length::Fixed(34.0)),
                container(text("当 API 调用失败时，将赏析结果回退存储到本地文件。").size(13))
                    .style(theme::quiet_text),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .into()
}

fn warning_block<'a>(warning: &'a str) -> Element<'a, Message> {
    container(text(warning).size(13))
        .padding([12, 14])
        .width(Length::Fill)
        .style(theme::outline_panel)
        .into()
}

fn status_accent(mode_label: &str) -> Color {
    match mode_label {
        "已配置" => Color::from_rgb8(0x67, 0xB2, 0x79),
        "回退存储" => Color::from_rgb8(0xC4, 0x8A, 0x4A),
        "未配置" | "不可用" => Color::from_rgb8(0xC4, 0x5A, 0x4A),
        _ => Color::from_rgb8(0x67, 0xB2, 0x79),
    }
}
