use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::ui::components::{modal_frame, modal_header};
use crate::ui::message::Message;

pub fn view<'a>() -> Element<'a, Message> {
    let hero = container(
        column![
            text("poem-rs").size(28),
            text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(14),
            text("本地诗库与 AI 发现并存的桌面阅读器").size(16),
            text("Iced 重写版本优先服务日常阅读、检索、发现与导入流程，保留安静克制的桌面质感，而不是追求旧界面的全量平移。").size(15),
        ]
        .spacing(8),
    )
    .padding(20)
    .width(Length::Fill)
    .style(container::rounded_box);

    let body = scrollable(
        column![
            hero,
            section(
                "这次重写关注什么",
                &[
                    "本地搜索优先，保持诗词浏览与阅读路径足够直接。",
                    "AI 发现以模态方式展开，聚焦关键词、片段与意境检索。",
                    "设置与关于页改为共享模态体系，减少侧边栏式干扰。",
                ],
            ),
            section(
                "当前范围说明",
                &[
                    "首个 Iced 版本刻意不包含收藏、AI 赏析与旧筛选分组。",
                    "窗口尺寸持久化不在当前重写范围内。",
                ],
            ),
        ]
        .spacing(16),
    )
    .height(Length::Fill);

    modal_frame(
        modal_header("关于", Some("应用说明与当前重写范围")),
        body,
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

fn section<'a>(title: &'a str, items: &'a [&'a str]) -> Element<'a, Message> {
    let list = items
        .iter()
        .fold(column![text(title).size(18)].spacing(10), |column, item| {
            column.push(text(format!("- {item}")).size(15))
        });

    container(list)
        .padding(20)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}
