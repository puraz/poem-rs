use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::ui::components::{
    ButtonKind, SurfaceKind, compact_button, field_input, input_block, modal_frame,
    modal_header_with_close, section_surface, secure_field_input,
};
use crate::ui::message::Message;
use crate::ui::state::SettingsForm;
use crate::ui::theme;

pub fn view<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let ai_fields = column![
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
    ]
    .spacing(20);

    let content = column![
        section_surface("AI 配置", ai_fields, SurfaceKind::Raised),
        Space::new().height(theme::SPACE_5 as f32),
        data_section(),
    ]
    .spacing(0);

    modal_frame(
        modal_header_with_close("设置", None, Message::CloseModal),
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

fn status_accent(mode_label: &str) -> Color {
    match mode_label {
        "已配置" => Color::from_rgb8(0x67, 0xB2, 0x79),
        "未配置" | "不可用" => Color::from_rgb8(0xC4, 0x5A, 0x4A),
        _ => Color::from_rgb8(0x67, 0xB2, 0x79),
    }
}

fn data_section<'a>() -> Element<'a, Message> {
    section_surface(
        "数据管理",
        column![
            section_surface(
                "备份与恢复",
                column![
                    container(
                        text("导出全部诗词为 JSON 文件，以便备份或迁移到其他设备。导入时自动跳过重复内容。")
                            .size(13)
                    )
                    .style(theme::quiet_text),
                    Space::new().height(theme::SPACE_4 as f32),
                    row![
                        compact_button("导出全部诗词", ButtonKind::Secondary)
                            .width(Length::Fill)
                            .on_press(Message::ExportPoems),
                        Space::new().width(theme::SPACE_3 as f32),
                        compact_button("导入诗词", ButtonKind::Secondary)
                            .width(Length::Fill)
                            .on_press(Message::ImportPoems),
                    ]
                    .spacing(0)
                    .align_y(Alignment::Center),
                ]
                .spacing(0),
                SurfaceKind::Outline,
            ),
        ]
        .spacing(0),
        SurfaceKind::Raised,
    )
}
