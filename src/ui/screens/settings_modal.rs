use iced::widget::{Space, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::ui::components::{ButtonKind, action_button, modal_frame, modal_header_with_close};
use crate::ui::message::Message;
use crate::ui::state::SettingsForm;

pub fn view<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let mut content = column![
        text(format!("当前模式：{}", form.mode_label)).size(14),
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
    ]
    .spacing(16);

    if !form.warning.trim().is_empty() {
        content = content.push(
            container(text(form.warning.as_str()).size(13))
                .padding(16)
                .width(Length::Fill)
                .style(container::rounded_box),
        );
    }

    modal_frame(
        modal_header_with_close("AI 设置", None, Message::CloseModal),
        scrollable(content).height(Length::Shrink),
        Some(
            row![
                action_button("清除 Key", ButtonKind::Danger)
                    .width(128)
                    .on_press(Message::ClearApiKey),
                Space::new().width(Length::Fill),
                action_button("保存设置", ButtonKind::Primary)
                    .width(140)
                    .on_press(Message::SaveSettings),
            ]
            .spacing(12)
            .into(),
        ),
    )
}

fn field<'a>(label: &'a str, input: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(column![text(label).size(14), input.into()].spacing(8))
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}
