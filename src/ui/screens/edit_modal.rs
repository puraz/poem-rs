use iced::widget::{Space, column, row, scrollable, text_editor};
use iced::{Element, Length};

use crate::ui::components::{
    ButtonKind, compact_button, field_input, input_block, modal_frame, modal_header_with_close,
};
use crate::ui::message::Message;
use crate::ui::state::EditForm;
use crate::ui::theme;

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
            text_editor(&form.content_editor)
                .placeholder("诗词正文")
                .on_action(Message::EditContentChanged)
                .padding(16)
                .size(18)
                .height(Length::Fixed(260.0))
                .style(theme::text_editor_default),
        ),
    ]
    .spacing(20);

    modal_frame(
        modal_header_with_close("编辑诗词", None, Message::CloseModal),
        scrollable(content).height(Length::Shrink),
        Some(
            row![
                Space::new().width(Length::Fill),
                compact_button("取消", ButtonKind::Secondary)
                    .width(108)
                    .on_press(Message::CloseModal),
                compact_button("保存", ButtonKind::Primary)
                    .width(116)
                    .on_press(Message::SaveEdit),
            ]
            .spacing(12)
            .into(),
        ),
    )
}
