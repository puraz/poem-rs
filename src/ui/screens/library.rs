use iced::widget::{button, column, container, row, scrollable, svg, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

use crate::domain::Poem;
use crate::ui::theme;

use super::super::message::Message;

pub fn view<'a>(
    poems: Vec<Poem>,
    selected_poem_id: Option<&str>,
    search_query: &'a str,
    title: &'a str,
) -> Element<'a, Message> {
    let search = container(
        row![
            svg("assets/icons/search.svg")
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0))
                .style(|active_theme: &Theme, _status| svg::Style {
                    color: Some(theme::tokens(active_theme).text_muted),
                }),
            text_input("搜索本地诗词： 标题、 作者、 内容", search_query)
                .on_input(Message::SearchChanged)
                .padding(0)
                .size(16)
                .width(Length::Fill)
                .style(theme::text_input_search_embedded),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding([18, 20])
    .width(Length::Fill)
    .style(theme::library_search_shell);

    let list = poems
        .into_iter()
        .fold(column![].spacing(22), |column, poem| {
            let selected = selected_poem_id == Some(poem.id.as_str());
            let title = poem.title.clone();
            let meta = poem.metadata();
            let snippet = poem.snippet();
            let is_favorite = poem.is_favorite;

            let favorite_icon = if is_favorite {
                text("★").size(18)
            } else {
                text("☆").size(18)
            };

            let card = container(
                row![
                    column![
                        text(title).size(if selected { 24 } else { 22 }),
                        container(text(meta).size(14)).style(move |active_theme| {
                            if selected {
                                theme::library_item_meta_selected(active_theme)
                            } else {
                                theme::subdued_text(active_theme)
                            }
                        }),
                        container(text(snippet).size(15)).style(move |active_theme| {
                            if selected {
                                theme::library_item_snippet_selected(active_theme)
                            } else {
                                theme::quiet_text(active_theme)
                            }
                        }),
                    ]
                    .spacing(10)
                    .width(Length::Fill),
                    container(
                        row![favorite_icon, text("›").size(22),]
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                    )
                    .style(move |active_theme| {
                        if selected {
                            theme::library_item_meta_selected(active_theme)
                        } else {
                            theme::subdued_text(active_theme)
                        }
                    }),
                ]
                .spacing(16)
                .align_y(iced::Alignment::Center),
            )
            .padding([22, 28])
            .width(Length::Fill)
            .style(move |active_theme| {
                if selected {
                    theme::library_item_selected_panel(active_theme)
                } else {
                    theme::library_item_panel(active_theme)
                }
            });

            column.push(
                button(card)
                    .style(button::text)
                    .width(Fill)
                    .on_press(Message::SelectPoem(poem.id.clone())),
            )
        });

    container(
        column![
            container(text(title).size(14)).style(theme::title_text),
            search,
            scrollable(list).height(Length::Fill),
        ]
        .spacing(22),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([28, 28])
    .style(theme::library_stage)
    .into()
}
