use iced::widget::{Space, column, row, scrollable, text_input};
use iced::{Element, Length};

use crate::ui::components::{
    ButtonKind, action_button, field_input, input_block, modal_frame, modal_header_with_close,
};
use crate::ui::message::Message;
use crate::ui::state::EditForm;

pub fn view(form: &'_ EditForm) -> Element<'_, Message> {
    let author_meta = row![
        input_block(
            "作者",
            field_input("作者", &form.author).on_input(Message::EditAuthorChanged),
        ),
        input_block(
            "朝代",
            field_input("朝代", &form.dynasty).on_input(Message::EditDynastyChanged),
        ),
    ]
    .spacing(16);

    let content = column![
        input_block(
            "标题",
            field_input("诗词标题", &form.title).on_input(Message::EditTitleChanged),
        ),
        author_meta,
        input_block(
            "正文",
            text_input("诗词正文", &form.content)
                .on_input(Message::EditContentChanged)
                .padding(16)
                .size(18)
                .width(Length::Fill),
        ),
    ]
    .spacing(20);

    modal_frame(
        modal_header_with_close("编辑诗词", None, Message::CloseModal),
        scrollable(content).height(Length::Shrink),
        Some(
            row![
                Space::new().width(Length::Fill),
                action_button("保存", ButtonKind::Primary)
                    .width(128)
                    .on_press(Message::SaveEdit),
                action_button("取消", ButtonKind::Secondary)
                    .width(112)
                    .on_press(Message::CloseModal),
            ]
            .spacing(12)
            .into(),
        ),
    )
}
