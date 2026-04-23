use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Length};

use crate::domain::Poem;
use crate::ui::components::{SurfaceKind, search_field, section_surface};
use crate::ui::theme;

use super::super::message::Message;

pub fn view<'a>(
    poems: Vec<Poem>,
    selected_poem_id: Option<&str>,
    search_query: &'a str,
    title: &'a str,
) -> Element<'a, Message> {
    let search = search_field(search_query).on_input(Message::SearchChanged);

    let list = poems
        .into_iter()
        .fold(column![].spacing(12), |column, poem| {
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
                        text(title).size(if selected { 26 } else { 23 }),
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
                .spacing(12)
                .align_y(iced::Alignment::Center),
            )
            .padding([18, 22])
            .width(Length::Fill)
            .style(move |active_theme| {
                if selected {
                    theme::library_item_selected_panel(active_theme)
                } else {
                    theme::raised_panel(active_theme)
                }
            });

            column.push(
                button(card)
                    .style(button::text)
                    .width(Fill)
                    .on_press(Message::SelectPoem(poem.id.clone())),
            )
        });

    section_surface(
        title,
        column![search, scrollable(list).height(Length::Fill)].spacing(20),
        SurfaceKind::Base,
    )
}
