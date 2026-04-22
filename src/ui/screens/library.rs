use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Fill, Length};

use crate::domain::Poem;
use crate::ui::components::{SurfaceKind, search_field, section_surface, surface};

use super::super::message::Message;

pub fn view<'a>(
    poems: Vec<Poem>,
    selected: Option<Poem>,
    search_query: &'a str,
) -> Element<'a, Message> {
    let search = search_field(search_query).on_input(Message::SearchChanged);

    let list = poems
        .into_iter()
        .fold(column![].spacing(12), |column, poem| {
            let card = surface(
                column![
                    text(poem.title.clone()).size(22),
                    text(poem.metadata()).size(14),
                    text(poem.snippet()).size(15),
                ]
                .spacing(8),
                SurfaceKind::Outline,
            )
            .padding(0);

            column.push(
                iced::widget::button(card)
                    .width(Fill)
                    .on_press(Message::SelectPoem(poem.id.clone())),
            )
        });

    let detail = if let Some(poem) = selected {
        column![
            text(poem.title.clone()).size(34),
            text(poem.metadata()).size(16),
            text(poem.tags_summary()).size(14),
            text(poem.content.clone()).size(22),
        ]
        .spacing(14)
    } else {
        column![
            text("暂无可展示诗词").size(24),
            text("当前筛选条件下没有可展示的诗词。").size(16)
        ]
        .spacing(10)
    };

    row![
        container(section_surface(
            "诗库",
            column![search, scrollable(list).height(Length::Fill)].spacing(20),
            SurfaceKind::Base,
        ))
        .width(Length::FillPortion(2))
        .height(Length::Fill),
        container(section_surface(
            "阅读",
            scrollable(detail),
            SurfaceKind::Raised,
        ))
        .width(Length::FillPortion(3))
        .height(Length::Fill),
    ]
    .spacing(24)
    .height(Length::Fill)
    .into()
}
