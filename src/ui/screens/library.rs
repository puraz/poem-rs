use iced::widget::{button, column, container, pick_list, row, scrollable, svg, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

use crate::domain::Poem;
use crate::ui::{assets, theme};

use super::super::message::Message;
const LIBRARY_STAGE_PADDING: [u16; 2] = [22, 20];
const LIBRARY_SEARCH_PADDING: [u16; 2] = [14, 16];
const LIBRARY_ITEM_PADDING: [u16; 2] = [16, 18];

pub fn view<'a>(
    poems: Vec<Poem>,
    selected_poem_id: Option<&str>,
    search_query: &'a str,
    title: &'a str,
    poet_filter: &'a str,
    poets: &'a [String],
) -> Element<'a, Message> {
    let search = container(
        row![
            svg(assets::svg_handle(assets::SEARCH))
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
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(LIBRARY_SEARCH_PADDING)
    .width(Length::Fill)
    .style(theme::library_search_shell);

    let poet_filter_dropdown = {
        let options: Vec<String> = poets.to_vec();
        let selected: Option<&String> = if poet_filter.is_empty() || poet_filter == "全部诗人" {
            poets.first()
        } else {
            poets.iter().find(|s| s.as_str() == poet_filter)
        };

        container(
            row![
                text("按诗人：").size(14),
                pick_list(
                    options,
                    selected,
                    |selected| {
                        let filter = if selected == "全部诗人" {
                            String::new()
                        } else {
                            selected.to_string()
                        };
                        Message::PoetFilterChanged(filter)
                    },
                )
                .padding([4, 8])
                .text_size(14),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([0, LIBRARY_SEARCH_PADDING[0]])
        .width(Length::Fill)
    };

    let list = poems
        .into_iter()
        .fold(column![].spacing(13), |column, poem| {
            let selected = selected_poem_id == Some(poem.id.as_str());
            let title = poem.title.clone();
            let snippet = poem.snippet();
            let is_favorite = poem.is_favorite;

            let card = container(
                column![
                    row![
                        text(title).size(22),
                        iced::widget::Space::new().width(Length::Fill),
                        favorite_icon(is_favorite),
                    ]
                    .align_y(Alignment::Center),
                    column![
                        {
                            let poet_name = poem.author.clone();
                            let poet_dynasty = poem.dynasty.clone();
                            button(text(format!("{} · {}", poet_name, poet_dynasty)).size(14))
                                .style(button::text)
                                .on_press(Message::PoetNameClicked(poet_name))
                        },
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
                ]
                .spacing(10),
            )
            .padding(LIBRARY_ITEM_PADDING)
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
            poet_filter_dropdown,
            scrollable(list)
                .direction(theme::scrollable_direction())
                .style(theme::scrollable_style)
                .height(Length::Fill),
        ]
        .spacing(20),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(LIBRARY_STAGE_PADDING)
    .style(theme::library_stage)
    .into()
}

fn favorite_icon<'a>(is_favorite: bool) -> Element<'a, Message> {
    let icon_path = if is_favorite {
        assets::FAVORITE_FILLED
    } else {
        assets::FAVORITE
    };

    svg(assets::svg_handle(icon_path))
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0))
        .style(move |active_theme: &Theme, _status| svg::Style {
            color: Some(if is_favorite {
                theme::tokens(active_theme).primary
            } else {
                theme::tokens(active_theme).text_muted
            }),
        })
        .into()
}
