use iced::widget::{column, container, scrollable, text};
use iced::{Element, Length};

use crate::ui::components::{modal_frame, modal_header_with_close};
use crate::ui::message::Message;
use crate::ui::theme;

pub fn view<'a>() -> Element<'a, Message> {
    let hero = container(
        column![
            text("poem-rs").size(28),
            text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(14),
            text("本地诗库与 AI 发现并存的桌面阅读器").size(16),
            text("Iced 重写版本优先服务日常阅读、检索、发现与导入流2程，保留安静克制的桌面质感，而不是追求旧界面的全量平移。").size(15),
        ]
        .spacing(8),
    )
    .padding(20)
    .width(Length::Fill)
    .style(container::rounded_box);

    let body = scrollable(
        column![
            hero,
            /*section(
                "这次重写关注什么",
                &[
                    "左中右三栏桌面结构，让导航、列表与阅读区分工更明确。",
                    "收藏、编辑、AI 赏析都回到主阅读路径，不再停留在半成品状态。",
                    "松烟笺与寒江雪双主题同时提供日读与夜读体验。",
                ],
            ),*/
        ]
        .spacing(16),
    )
    .direction(theme::scrollable_direction())
    .style(theme::scrollable_style)
    .height(Length::Fill);

    modal_frame(
        modal_header_with_close("关于", None, Message::CloseModal),
        body,
        None,
    )
}
