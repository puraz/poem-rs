use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::ui::components::{modal_frame, modal_header};
use crate::ui::message::Message;
use crate::ui::state::DiscoveryListItem;

pub fn view(
    query: String,
    loading: bool,
    status: String,
    items: Vec<DiscoveryListItem>,
) -> Element<'static, Message> {
    let search_label = if loading { "搜索中" } else { "搜索" };

    let results: Element<'static, Message> = if items.is_empty() {
        container(text(empty_state_text(&query, loading, &status)).size(15))
            .padding(24)
            .width(Length::Fill)
            .style(container::rounded_box)
            .into()
    } else {
        let cards = items
            .into_iter()
            .enumerate()
            .fold(column![].spacing(12), |column, (index, item)| {
                column.push(result_card(index, item))
            });

        scrollable(cards).height(Length::Fill).into()
    };

    let mut content = column![
        text("输入关键词、片段或意境，调用现有 AI 发现流程生成可导入的诗词候选。").size(14),
        row![
            text_input("输入关键词、片段或意境", &query).on_input(Message::DiscoveryQueryChanged),
            button(search_label)
                .style(button::primary)
                .on_press(Message::SubmitDiscovery),
        ]
        .spacing(12),
    ]
    .spacing(16);

    if loading || !status.trim().is_empty() {
        content = content.push(
            container(text(status_line(loading, &status)).size(14))
                .padding(14)
                .width(Length::Fill)
                .style(container::rounded_box),
        );
    }

    content = content.push(results);

    modal_frame(
        modal_header("发现新诗词", Some("从关键词、诗句片段或意境生成可导入候选")),
        scrollable(content).height(Length::Shrink),
        Some(
            row![
                Space::new().width(Length::Fill),
                button("关闭")
                    .style(button::secondary)
                    .on_press(Message::CloseModal),
            ]
            .spacing(12)
            .into(),
        ),
    )
}

fn result_card(index: usize, item: DiscoveryListItem) -> Element<'static, Message> {
    let metadata = if item.dynasty.trim().is_empty() {
        item.author.clone()
    } else {
        format!("{} · {}", item.author, item.dynasty)
    };
    let excerpt_size = if item.excerpt.centered { 16 } else { 15 };

    let body = column![
        row![
            column![text(item.title).size(18), text(metadata).size(13),].spacing(6),
            Space::new().width(Length::Fill),
            text(item.relevance).size(13),
        ]
        .spacing(12),
        text(item.excerpt.text).size(excerpt_size),
        text(item.reason).size(14),
        row![
            Space::new().width(Length::Fill),
            button("导入到诗库")
                .style(button::primary)
                .on_press(Message::ImportDiscovery(index)),
        ]
        .spacing(12),
    ]
    .spacing(10);

    container(body)
        .padding(18)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn status_line(loading: bool, status: &str) -> String {
    if loading {
        "AI 正在搜索最匹配的诗词…".to_string()
    } else {
        status.to_string()
    }
}

fn empty_state_text(query: &str, loading: bool, status: &str) -> String {
    if loading {
        "正在整理结果…".to_string()
    } else if query.trim().is_empty() {
        "输入关键词、诗句片段或意境，再开始一次 AI 搜索。".to_string()
    } else if status.trim().is_empty() {
        "没有可显示的结果，换个关键词再试。".to_string()
    } else {
        status.to_string()
    }
}
