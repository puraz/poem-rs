use iced::{
    Element, Length,
    widget::{self, TextInput},
};

use crate::ui::theme;

pub fn field_input<'a, Message: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
) -> TextInput<'a, Message> {
    widget::text_input(placeholder, value)
        .padding(theme::SPACE_4)
        .size(16)
        .width(Length::Fill)
        .style(theme::text_input_default)
}

pub fn search_input<'a, Message: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
) -> TextInput<'a, Message> {
    widget::text_input(placeholder, value)
        .padding(theme::SPACE_4)
        .size(16)
        .width(Length::Fill)
        .style(theme::text_input_search)
}

pub fn search_input_prominent<'a, Message: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
) -> TextInput<'a, Message> {
    widget::text_input(placeholder, value)
        .padding([theme::SPACE_5, theme::SPACE_5])
        .size(18)
        .width(Length::Fill)
        .style(theme::text_input_search_prominent)
}

pub fn search_field<'a, Message: Clone + 'a>(value: &'a str) -> TextInput<'a, Message> {
    search_input("搜索本地诗词: 标题、作者、内容", value)
}

pub fn input_block<'a, Message: 'a>(
    label: impl Into<String>,
    input: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let label = widget::container(widget::text(label.into()).size(13)).style(theme::subdued_text);

    widget::Column::new()
        .spacing(f32::from(theme::SPACE_2))
        .push(label)
        .push(input)
        .into()
}
