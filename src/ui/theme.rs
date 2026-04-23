use iced::{
    Background, Border, Color, Shadow, Theme, Vector,
    theme::Palette,
    widget::{button, container, text_input},
};

use super::message::ThemeChoice;

pub const SPACE_1: u16 = 4;
pub const SPACE_2: u16 = 8;
pub const SPACE_3: u16 = 12;
pub const SPACE_4: u16 = 16;
pub const SPACE_5: u16 = 24;
pub const SPACE_6: u16 = 32;
pub const SPACE_7: u16 = 40;

pub const RADIUS_SMALL: f32 = 10.0;
pub const RADIUS_MEDIUM: f32 = 16.0;
pub const RADIUS_LARGE: f32 = 24.0;
pub const RADIUS_XL: f32 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    App,
    Sidebar,
    Pane,
    PaneSoft,
    PanelAccent,
    Outline,
    Toast,
    Backdrop,
    Appreciation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy)]
struct Tokens {
    background: Color,
    sidebar: Color,
    pane: Color,
    pane_soft: Color,
    appreciation: Color,
    line_subtle: Color,
    line_strong: Color,
    title: Color,
    text: Color,
    text_muted: Color,
    text_soft: Color,
    primary: Color,
    primary_hover: Color,
    success: Color,
    warning: Color,
    danger: Color,
    shadow: Color,
}

pub fn app_theme(choice: ThemeChoice) -> Theme {
    let palette = match choice {
        ThemeChoice::Songyanjian => Palette {
            background: color(0xF9F6F0),
            text: color(0x4A4440),
            primary: color(0xC45A4A),
            success: Color::from_rgb8(0x5D, 0x8D, 0x79),
            warning: color(0xA6_4B_3A),
            danger: color(0xC4_5A_4A),
        },
        ThemeChoice::Hanjiangxue => Palette {
            background: color(0x1E1E1E),
            text: color(0xDCD7D0),
            primary: color(0xC45A4A),
            success: Color::from_rgb8(0x7A, 0xA6, 0x8A),
            warning: color(0xE5D3B0),
            danger: color(0xD46B5C),
        },
    };

    Theme::custom(choice.display_name().to_string(), palette)
}

pub fn page_background(theme: &Theme) -> container::Style {
    let tokens = tokens(theme);
    container_style(
        tokens.background,
        Color::TRANSPARENT,
        0.0,
        Shadow::default(),
        tokens.text,
    )
}

pub fn panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Pane)
}

pub fn sidebar_panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Sidebar)
}

pub fn raised_panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::PaneSoft)
}

pub fn accent_panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::PanelAccent)
}

pub fn library_item_selected_panel(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    let (background, border_color, text_color, shadow_style) = if is_light(theme) {
        (
            color(0xF7ECE8),
            tint(t.primary, 0.28),
            t.title,
            shadow(t.shadow, 0.0, 6.0, 18.0),
        )
    } else {
        (
            color(0x3A2D2A),
            tint(t.primary_hover, 0.12),
            color(0xFFF7EF),
            shadow(t.shadow, 0.0, 6.0, 18.0),
        )
    };

    container_style(
        background,
        border_color,
        RADIUS_LARGE,
        shadow_style,
        text_color,
    )
}

pub fn outline_panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Outline)
}

pub fn modal_backdrop(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Backdrop)
}

pub fn modal_frame(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    container_style(
        t.pane,
        t.line_strong,
        RADIUS_XL,
        shadow(t.shadow, 0.0, 18.0, 42.0),
        t.text,
    )
}

pub fn toast_surface(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Toast)
}

pub fn toast_message_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(if is_light(theme) {
            color(0xE6DDD4)
        } else {
            color(0xCFC6BD)
        }),
        ..container::Style::default()
    }
}

pub fn appreciation_panel(theme: &Theme) -> container::Style {
    surface_style(theme, SurfaceKind::Appreciation)
}

pub fn eyebrow_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(tokens(theme).primary),
        ..container::Style::default()
    }
}

pub fn subdued_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(tokens(theme).text_muted),
        ..container::Style::default()
    }
}

pub fn quiet_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(tokens(theme).text_soft),
        ..container::Style::default()
    }
}

pub fn library_item_meta_selected(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(if is_light(theme) {
            color(0x84584F)
        } else {
            color(0xE8CFC8)
        }),
        ..container::Style::default()
    }
}

pub fn library_item_snippet_selected(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(if is_light(theme) {
            color(0x6F5A55)
        } else {
            color(0xCFBFBA)
        }),
        ..container::Style::default()
    }
}

pub fn title_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(tokens(theme).title),
        ..container::Style::default()
    }
}

pub fn button_primary(theme: &Theme, status: button::Status) -> button::Style {
    button_style(theme, Tone::Primary, true, status)
}

pub fn button_secondary(theme: &Theme, status: button::Status) -> button::Style {
    button_style(theme, Tone::Neutral, true, status)
}

pub fn button_ghost(theme: &Theme, status: button::Status) -> button::Style {
    button_style(theme, Tone::Neutral, false, status)
}

pub fn button_danger(theme: &Theme, status: button::Status) -> button::Style {
    button_style(theme, Tone::Danger, true, status)
}

pub fn button_nav(theme: &Theme, status: button::Status) -> button::Style {
    button_style(theme, Tone::Neutral, false, status)
}

pub fn button_nav_active(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let mut style = button_style(theme, Tone::Primary, false, status);
    style.background = Some(Background::Color(match status {
        button::Status::Active => tint(t.primary, 0.12),
        button::Status::Hovered => tint(t.primary, 0.18),
        button::Status::Pressed => tint(t.primary, 0.24),
        button::Status::Disabled => tint(t.primary, 0.08),
    }));
    style.border = border(t.primary, 1.0, RADIUS_LARGE);
    style
}

pub fn text_input_default(theme: &Theme, status: text_input::Status) -> text_input::Style {
    input_style(theme, status, false)
}

pub fn text_input_search(theme: &Theme, status: text_input::Status) -> text_input::Style {
    input_style(theme, status, true)
}

pub fn text_input_search_prominent(theme: &Theme, status: text_input::Status) -> text_input::Style {
    input_style_with_radius(theme, status, true, RADIUS_LARGE)
}

pub fn chip_style(theme: &Theme, tone: Tone) -> container::Style {
    let t = tokens(theme);
    let (background, border_color, text_color) = match tone {
        Tone::Neutral => (t.pane_soft, t.line_subtle, t.text_muted),
        Tone::Primary => (tint(t.primary, 0.14), tint(t.primary, 0.45), t.primary),
        Tone::Success => (tint(t.success, 0.16), tint(t.success, 0.45), t.success),
        Tone::Warning => (tint(t.warning, 0.16), tint(t.warning, 0.45), t.warning),
        Tone::Danger => (tint(t.danger, 0.16), tint(t.danger, 0.45), t.danger),
    };

    container_style(
        background,
        border_color,
        RADIUS_SMALL,
        Shadow::default(),
        text_color,
    )
}

pub fn surface_style(theme: &Theme, kind: SurfaceKind) -> container::Style {
    let t = tokens(theme);
    match kind {
        SurfaceKind::App => container_style(
            t.background,
            Color::TRANSPARENT,
            0.0,
            Shadow::default(),
            t.text,
        ),
        SurfaceKind::Sidebar => container_style(
            t.sidebar,
            t.line_subtle,
            RADIUS_LARGE,
            shadow(t.shadow, 0.0, 8.0, 20.0),
            t.text,
        ),
        SurfaceKind::Pane => container_style(
            t.pane,
            t.line_subtle,
            RADIUS_LARGE,
            shadow(t.shadow, 0.0, 8.0, 20.0),
            t.text,
        ),
        SurfaceKind::PaneSoft => container_style(
            t.pane_soft,
            t.line_subtle,
            RADIUS_LARGE,
            shadow(t.shadow, 0.0, 4.0, 12.0),
            t.text,
        ),
        SurfaceKind::PanelAccent => container_style(
            tint(t.primary, if is_light(theme) { 0.08 } else { 0.14 }),
            tint(t.primary, 0.38),
            RADIUS_LARGE,
            shadow(t.shadow, 0.0, 6.0, 18.0),
            t.text,
        ),
        SurfaceKind::Outline => container_style(
            Color::from_rgba8(255, 255, 255, if is_light(theme) { 0.55 } else { 0.03 }),
            t.line_subtle,
            RADIUS_MEDIUM,
            Shadow::default(),
            t.text,
        ),
        SurfaceKind::Toast => container_style(
            if is_light(theme) {
                color(0x3A3531)
            } else {
                color(0x2A2A2A)
            },
            if is_light(theme) {
                Color::from_rgba8(255, 255, 255, 0.12)
            } else {
                t.line_strong
            },
            RADIUS_MEDIUM,
            shadow(t.shadow, 0.0, 10.0, 24.0),
            color(0xFFFDF7),
        ),
        SurfaceKind::Backdrop => container::Style {
            text_color: None,
            background: Some(Background::Color(Color::from_rgba8(17, 16, 14, 0.42))),
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        },
        SurfaceKind::Appreciation => container_style(
            t.appreciation,
            t.line_subtle,
            RADIUS_LARGE,
            Shadow::default(),
            t.text,
        ),
    }
}

fn button_style(theme: &Theme, tone: Tone, filled: bool, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let (base_fill, base_border, base_text) = match tone {
        Tone::Primary => (
            t.primary,
            t.primary_hover,
            if is_light(theme) {
                color(0xFFFFFF)
            } else {
                color(0xFFF7EF)
            },
        ),
        Tone::Success => (
            t.success,
            t.success,
            if is_light(theme) {
                color(0xFFFFFF)
            } else {
                color(0xFFF7EF)
            },
        ),
        Tone::Warning => (
            t.warning,
            t.warning,
            if is_light(theme) {
                color(0x3A3531)
            } else {
                color(0x1E1E1E)
            },
        ),
        Tone::Danger => (t.danger, t.danger, color(0xFFFFFF)),
        Tone::Neutral => (
            if is_light(theme) {
                color(0xFFFFFF)
            } else {
                t.pane_soft
            },
            t.line_strong,
            t.text,
        ),
    };

    let background = match status {
        button::Status::Active => {
            if filled {
                base_fill
            } else {
                tint(base_fill, if is_light(theme) { 0.05 } else { 0.12 })
            }
        }
        button::Status::Hovered => {
            if filled {
                lift(base_fill, 0.05)
            } else {
                tint(base_fill, if is_light(theme) { 0.10 } else { 0.18 })
            }
        }
        button::Status::Pressed => {
            if filled {
                deepen(base_fill, 0.05)
            } else {
                tint(base_fill, if is_light(theme) { 0.15 } else { 0.24 })
            }
        }
        button::Status::Disabled => tint(base_fill, if filled { 0.35 } else { 0.08 }),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: if matches!(status, button::Status::Disabled) {
            tint(base_text, 0.45)
        } else if !filled && tone == Tone::Neutral {
            if matches!(status, button::Status::Hovered) {
                t.primary
            } else {
                t.text
            }
        } else {
            base_text
        },
        border: border(
            if filled { base_border } else { t.line_subtle },
            1.0,
            RADIUS_LARGE,
        ),
        shadow: shadow(
            t.shadow,
            0.0,
            if filled && !matches!(status, button::Status::Pressed) {
                4.0
            } else {
                1.0
            },
            if filled { 16.0 } else { 0.0 },
        ),
        snap: false,
    }
}

fn input_style(theme: &Theme, status: text_input::Status, is_search: bool) -> text_input::Style {
    input_style_with_radius(theme, status, is_search, RADIUS_MEDIUM)
}

fn input_style_with_radius(
    theme: &Theme,
    status: text_input::Status,
    is_search: bool,
    radius: f32,
) -> text_input::Style {
    let t = tokens(theme);
    let base = if is_search {
        color(0xFFFFFF)
    } else {
        t.pane_soft
    };
    let (background, border_color) = match status {
        text_input::Status::Active => (base, t.line_subtle),
        text_input::Status::Hovered => (lift(base, 0.01), t.line_strong),
        text_input::Status::Focused { .. } => (lift(base, 0.02), t.primary),
        text_input::Status::Disabled => (tint(base, 0.75), tint(t.line_subtle, 0.55)),
    };

    text_input::Style {
        background: Background::Color(background),
        border: border(border_color, 1.0, radius),
        icon: t.text_muted,
        placeholder: t.text_soft,
        value: t.text,
        selection: tint(t.primary, if is_light(theme) { 0.22 } else { 0.34 }),
    }
}

fn tokens(theme: &Theme) -> Tokens {
    if is_light(theme) {
        Tokens {
            background: color(0xF9F6F0),
            sidebar: color(0xF3EFE6),
            pane: color(0xFCFAF5),
            pane_soft: color(0xFFFFFF),
            appreciation: color(0xF6F2EB),
            line_subtle: color(0xE2DCD2),
            line_strong: color(0xD1C7B8),
            title: color(0x3A3531),
            text: color(0x4A4440),
            text_muted: color(0x8B8174),
            text_soft: color(0x9C9388),
            primary: color(0xC45A4A),
            primary_hover: color(0xA64B3A),
            success: color(0x6A8B76),
            warning: color(0xA64B3A),
            danger: color(0xC45A4A),
            shadow: Color::from_rgba8(78, 61, 41, 0.10),
        }
    } else {
        Tokens {
            background: color(0x1E1E1E),
            sidebar: color(0x252525),
            pane: color(0x2A2A2A),
            pane_soft: color(0x303030),
            appreciation: color(0x2F2A26),
            line_subtle: Color::from_rgba8(220, 215, 208, 0.12),
            line_strong: Color::from_rgba8(229, 211, 176, 0.22),
            title: color(0xE5D3B0),
            text: color(0xDCD7D0),
            text_muted: color(0xB7AEA3),
            text_soft: color(0x938A82),
            primary: color(0xC45A4A),
            primary_hover: color(0xD46B5C),
            success: color(0x87A68D),
            warning: color(0xE5D3B0),
            danger: color(0xD46B5C),
            shadow: Color::from_rgba8(0, 0, 0, 0.22),
        }
    }
}

fn is_light(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r > 0.6
}

fn container_style(
    background: Color,
    border_color: Color,
    radius: f32,
    shadow: Shadow,
    text_color: Color,
) -> container::Style {
    container::Style {
        text_color: Some(text_color),
        background: Some(Background::Color(background)),
        border: border(border_color, 1.0, radius),
        shadow,
        snap: false,
    }
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: radius.into(),
    }
}

fn shadow(color: Color, x: f32, y: f32, blur: f32) -> Shadow {
    Shadow {
        color,
        offset: Vector::new(x, y),
        blur_radius: blur,
    }
}

fn color(hex: u32) -> Color {
    Color::from_rgb8(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

fn tint(color: Color, amount: f32) -> Color {
    let alpha = amount.clamp(0.0, 1.0);
    Color {
        r: color.r + (1.0 - color.r) * alpha,
        g: color.g + (1.0 - color.g) * alpha,
        b: color.b + (1.0 - color.b) * alpha,
        a: color.a,
    }
}

fn deepen(color: Color, amount: f32) -> Color {
    let alpha = amount.clamp(0.0, 1.0);
    Color {
        r: color.r * (1.0 - alpha),
        g: color.g * (1.0 - alpha),
        b: color.b * (1.0 - alpha),
        a: color.a,
    }
}

fn lift(color: Color, amount: f32) -> Color {
    tint(color, amount)
}
