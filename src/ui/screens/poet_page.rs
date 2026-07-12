use iced::widget::{button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Fill, Length, Theme};

use crate::domain::Poem;
use crate::ui::{assets, theme};

use super::super::message::Message;

const PAGE_PADDING: [u16; 2] = [22, 20];
const PROFILE_PADDING: [u16; 2] = [16, 18];
const ITEM_PADDING: [u16; 2] = [16, 18];

pub fn view<'a>(
    poet_name: &'a str,
    profile_content: Option<&'a str>,
    is_loading: bool,
    poems: Vec<Poem>,
    selected_poem_id: Option<&str>,
) -> Element<'a, Message> {
    let back_button = button(
        row![
            svg(assets::svg_handle(assets::CHEVRON_LEFT))
                .width(Length::Fixed(18.0))
                .height(Length::Fixed(18.0))
                .style(|active_theme: &Theme, _status| svg::Style {
                    color: Some(theme::tokens(active_theme).text),
                }),
            text("返回").size(16),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(button::text)
    .on_press(Message::SwitchContentMode(super::super::message::ContentMode::Library));

    let header = row![
        back_button,
        iced::widget::Space::new().width(Length::Fill),
        text(poet_name).size(22),
        iced::widget::Space::new().width(Length::Fill),
        button(
            row![
                svg(assets::svg_handle(assets::REFRESH))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|active_theme: &Theme, _status| svg::Style {
                        color: Some(theme::tokens(active_theme).text),
                    }),
                text("刷新档案").size(14),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(button::text)
        .on_press(Message::RefreshPoetProfile(poet_name.to_string())),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let profile_section = if is_loading {
        container(
            text("正在获取诗人档案…")
                .size(15)
                .style(|active_theme: &Theme| text::Style {
                    color: Some(theme::tokens(active_theme).text_muted),
                }),
        )
        .padding(PROFILE_PADDING)
        .width(Length::Fill)
        .style(theme::library_item_panel)
    } else if let Some(content) = profile_content {
        container(
            scrollable(text(content).size(15))
                .direction(theme::scrollable_direction())
                .style(theme::scrollable_style)
                .height(Length::Fixed(300.0)),
        )
        .padding(PROFILE_PADDING)
        .width(Length::Fill)
        .style(theme::library_item_panel)
    } else {
        container(
            text("暂无诗人档案信息。点击「刷新档案」通过 AI 获取。")
                .size(15)
                .style(|active_theme: &Theme| text::Style {
                    color: Some(theme::tokens(active_theme).text_muted),
                }),
        )
        .padding(PROFILE_PADDING)
        .width(Length::Fill)
        .style(theme::library_item_panel)
    };

    let poems_title = container(
        text(format!("{} 的诗词 (共 {} 首)", poet_name, poems.len())).size(14),
    )
    .style(theme::title_text);

    let list = poems
        .into_iter()
        .fold(column![].spacing(13), |column, poem| {
            let selected = selected_poem_id == Some(poem.id.as_str());
            let title = poem.title.clone();
            let meta = poem.metadata();
            let snippet = poem.snippet();

            let card = container(
                column![
                    text(title).size(22),
                    column![
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
                ]
                .spacing(10),
            )
            .padding(ITEM_PADDING)
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
            header,
            profile_section,
            poems_title,
            scrollable(list)
                .direction(theme::scrollable_direction())
                .style(theme::scrollable_style)
                .height(Length::Fill),
        ]
        .spacing(20),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(PAGE_PADDING)
    .into()
}
