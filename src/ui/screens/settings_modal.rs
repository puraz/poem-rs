use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::ui::components::{modal_frame, modal_header};
use crate::ui::message::Message;
use crate::ui::state::SettingsForm;

pub fn view<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let content = column![
        text(format!("当前模式：{}", form.mode_label)).size(14),
        status_block(form),
        field(
            "Base URL",
            text_input("https://api.openai.com/v1", &form.base_url)
                .on_input(Message::SettingsBaseUrlChanged),
        ),
        field(
            "Model",
            text_input("gpt-4.1-mini", &form.model).on_input(Message::SettingsModelChanged),
        ),
        field(
            "API Key",
            text_input("输入 API Key", &form.api_key).on_input(Message::SettingsApiKeyChanged),
        ),
        container(
            checkbox(form.allow_file_fallback)
                .label("允许文件回退存储")
                .on_toggle(Message::SettingsFallbackChanged),
        )
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box),
        row![
            button("清除 Key")
                .style(button::danger)
                .on_press(Message::ClearApiKey),
            Space::new().width(Length::Fill),
            button("保存设置")
                .style(button::primary)
                .on_press(Message::SaveSettings),
        ]
        .spacing(12),
    ]
    .spacing(16);

    modal_frame(
        modal_header("AI 设置", Some("管理模型、Base URL 与 API Key 持久化行为")),
        scrollable(content).height(Length::Shrink),
        Some(
            row![
                button("清除 Key")
                    .style(button::danger)
                    .on_press(Message::ClearApiKey),
                Space::new().width(Length::Fill),
                button("关闭")
                    .style(button::secondary)
                    .on_press(Message::CloseModal),
                button("保存设置")
                    .style(button::primary)
                    .on_press(Message::SaveSettings),
            ]
            .spacing(12)
            .into(),
        ),
    )
}

fn status_block<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let mut block = column![];

    if !form.warning.trim().is_empty() {
        block = block.push(text(form.warning.as_str()).size(13));
    } else {
        block = block.push(
            text("Base URL、模型与密钥会复用现有配置持久化逻辑。API Key 留空时不会覆盖已保存的凭据。")
                .size(14),
        );
    }

    container(block.spacing(8))
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(column![text(label).size(14), input.into()].spacing(8))
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}
