use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::ui::components::{
    ButtonKind, action_button, modal_frame, modal_header_with_close, search_input_prominent,
    surface,
};
use crate::ui::message::Message;
use crate::ui::state::DiscoveryListItem;
use crate::ui::{components::SurfaceKind, theme};

pub fn view<'a>(
    query: &'a str,
    loading: bool,
    status: &'a str,
    items: Vec<DiscoveryListItem>,
) -> Element<'a, Message> {
    let search_label = if loading { "搜索中..." } else { "搜索" };
    let result_count = items.len();

    let search_panel = surface(
        column![
            row![
                search_input_prominent("例如：春江花月夜 / 西窗烛 / 山雨欲来", query)
                    .on_input(Message::DiscoveryQueryChanged)
                    .width(Length::Fill),
                action_button(search_label, ButtonKind::Primary)
                    .width(140)
                    .on_press(Message::SubmitDiscovery),
            ]
            .spacing(14)
            .align_y(iced::Alignment::Center),
            if !loading && !status.trim().is_empty() {
                container(text(status).size(14)).style(theme::subdued_text)
            } else if result_count > 0 {
                container(text(format!("已生成 {result_count} 条候选。")).size(14))
                    .style(theme::subdued_text)
            } else {
                container(Space::new())
            },
        ]
        .spacing(16),
        SurfaceKind::Raised,
    );

    let results: Option<Element<'a, Message>> = if result_count > 0 {
        let cards = items
            .into_iter()
            .enumerate()
            .fold(column![].spacing(14), |column, (index, item)| {
                column.push(result_card(index, item))
            });

        Some(
            column![
                text("候选结果").size(20),
                scrollable(cards).height(Length::Fill),
            ]
            .spacing(14)
            .into(),
        )
    } else if loading || !status.trim().is_empty() {
        Some(empty_state(loading, status))
    } else {
        None
    };

    let mut body = column![search_panel].spacing(18);
    if let Some(results) = results {
        body = body.push(results);
    }

    modal_frame(
        modal_header_with_close("发现新诗词", None, Message::CloseModal),
        scrollable(body).height(Length::Shrink),
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

fn empty_state<'a>(loading: bool, status: &'a str) -> Element<'a, Message> {
    let (title, detail) = empty_state_copy(loading, status);

    let content = if let Some(detail) = detail {
        column![
            text(title).size(22),
            container(text(detail).size(16)).style(theme::subdued_text),
        ]
        .spacing(14)
    } else {
        column![text(title).size(22)].spacing(0)
    };

    surface(content, SurfaceKind::Raised).into()
}

fn empty_state_copy<'a>(loading: bool, status: &'a str) -> (&'a str, Option<&'a str>) {
    if loading {
        ("正在整理结果...", None)
    } else {
        ("当前没有可展示结果", Some(status))
    }
}
