use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::ui::components::{
    ButtonKind, action_button, compact_button, field_input, loading_indicator, modal_frame,
    modal_header_with_close, surface,
};
use crate::ui::message::Message;
use crate::ui::state::DiscoveryListItem;
use crate::ui::{components::SurfaceKind, theme};

const DISCOVERY_BODY_HEIGHT: f32 = 420.0;

pub fn view<'a>(
    query: &'a str,
    loading: bool,
    loading_frame: usize,
    status: &'a str,
    items: Vec<DiscoveryListItem>,
) -> Element<'a, Message> {
    let search_label = if loading { "搜索中..." } else { "搜索" };
    let result_count = items.len();
    let mut search_button = compact_button(search_label, ButtonKind::Primary).width(112);
    if !loading {
        search_button = search_button.on_press(Message::SubmitDiscovery);
    }

    let search_row = row![
        field_input("例如：春江花月夜 / 西窗烛 / 山雨欲来", query)
            .on_input(Message::DiscoveryQueryChanged)
            .width(Length::Fill),
        search_button,
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);

    let results: Option<Element<'a, Message>> = if loading {
        Some(
            container(loading_indicator("推荐诗词中", loading_frame))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        )
    } else if result_count > 0 {
        let cards = items
            .into_iter()
            .enumerate()
            .fold(column![].spacing(14), |column, (index, item)| {
                column.push(result_card(index, item))
            });

        Some(
            column![
                if !status.trim().is_empty() {
                    container(text(status).size(14)).style(theme::subdued_text)
                } else {
                    container(text(format!("已生成 {result_count} 条候选。")).size(14))
                        .style(theme::subdued_text)
                },
                scrollable(cards)
                    .direction(theme::scrollable_direction())
                    .style(theme::scrollable_style)
                    .height(Length::Fill),
            ]
            .height(Length::Fill)
            .spacing(14)
            .into(),
        )
    } else {
        Some(empty_state(query, loading, status))
    };

    let mut body = column![search_row].spacing(14).height(Length::Fill);
    if let Some(results) = results {
        body = body.push(results);
    }

    modal_frame(
        modal_header_with_close("发现新诗词", None, Message::CloseModal),
        container(body)
            .width(Length::Fill)
            .height(Length::Fixed(DISCOVERY_BODY_HEIGHT)),
        None,
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
            column![
                text(item.title).size(20),
                container(text(metadata).size(13)).style(theme::subdued_text),
            ]
            .spacing(6),
            Space::new().width(Length::Fill),
            container(text(item.relevance).size(13)).style(theme::eyebrow_text),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Start),
        text(item.excerpt.text).size(excerpt_size),
        container(text(format!("推荐理由: {}", item.reason)).size(14)).style(theme::quiet_text),
        row![
            Space::new().width(Length::Fill),
            action_button("导入到诗库", ButtonKind::Primary)
                .on_press(Message::ImportDiscovery(index)),
        ]
        .spacing(12),
    ]
    .spacing(14);

    surface(body, SurfaceKind::Raised).into()
}

fn empty_state<'a>(query: &'a str, loading: bool, status: &'a str) -> Element<'a, Message> {
    let title = empty_state_copy(query, loading, status);

    container(text(title).size(22))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::subdued_text)
        .into()
}

fn empty_state_copy<'a>(query: &'a str, loading: bool, status: &'a str) -> &'a str {
    if loading {
        "正在搜索中..."
    } else if query.trim().is_empty() {
        "输入关键词开始搜索"
    } else if !status.trim().is_empty() {
        status
    } else {
        "当前没有可展示结果"
    }
}
