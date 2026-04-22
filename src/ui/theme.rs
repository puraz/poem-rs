use iced::{
    Background, Border, Color, Shadow, Theme, Vector,
    theme::Palette,
    widget::{button, container, text_input},
};

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

pub const BACKGROUND: Color = Color::from_rgb8(15, 18, 24);
pub const BACKGROUND_ELEVATED: Color = Color::from_rgb8(22, 27, 35);
pub const SURFACE_BASE: Color = Color::from_rgb8(27, 32, 42);
pub const SURFACE_RAISED: Color = Color::from_rgb8(34, 40, 52);
pub const SURFACE_ACCENT: Color = Color::from_rgb8(42, 52, 69);
pub const LINE_SUBTLE: Color = Color::from_rgba8(255, 255, 255, 0.08);
pub const LINE_STRONG: Color = Color::from_rgba8(184, 198, 224, 0.22);

pub const TEXT_STRONG: Color = Color::from_rgb8(242, 245, 250);
pub const TEXT: Color = Color::from_rgb8(222, 228, 237);
pub const TEXT_MUTED: Color = Color::from_rgb8(150, 162, 181);
pub const TEXT_SOFT: Color = Color::from_rgb8(118, 130, 149);

pub const ACCENT: Color = Color::from_rgb8(111, 159, 232);
pub const ACCENT_HI: Color = Color::from_rgb8(143, 186, 250);
pub const SUCCESS: Color = Color::from_rgb8(94, 186, 141);
pub const WARNING: Color = Color::from_rgb8(228, 180, 93);
pub const DANGER: Color = Color::from_rgb8(217, 103, 103);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Base,
    Raised,
    Accent,
    Outline,
    Toast,
    Backdrop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
}

pub const fn space(step: u16) -> u16 {
    match step {
        0 => 0,
        1 => SPACE_1,
        2 => SPACE_2,
        3 => SPACE_3,
        4 => SPACE_4,
        5 => SPACE_5,
        6 => SPACE_6,
        _ => SPACE_7,
    }
}

pub fn app_theme() -> Theme {
    Theme::custom(
        "Poem Desktop",
        Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
        },
    )
}

pub fn page_background(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(BACKGROUND)),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Base)
}

pub fn raised_panel(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Raised)
}

pub fn accent_panel(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Accent)
}

pub fn outline_panel(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Outline)
}

pub fn modal_backdrop(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Backdrop)
}

pub fn modal_frame(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(BACKGROUND_ELEVATED)),
        border: border(LINE_STRONG, 1.0, RADIUS_XL),
        shadow: shadow(Color::from_rgba8(0, 0, 0, 0.28), 0.0, 20.0, 48.0),
        snap: false,
    }
}

pub fn toast_surface(_theme: &Theme) -> container::Style {
    surface_style(SurfaceKind::Toast)
}

pub fn eyebrow_text(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(ACCENT_HI),
        ..container::Style::default()
    }
}

pub fn subdued_text(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_MUTED),
        ..container::Style::default()
    }
}

pub fn quiet_text(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SOFT),
        ..container::Style::default()
    }
}

pub fn button_primary(_theme: &Theme, status: button::Status) -> button::Style {
    button_style(Tone::Primary, true, status)
}

pub fn button_secondary(_theme: &Theme, status: button::Status) -> button::Style {
    button_style(Tone::Neutral, true, status)
}

pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    button_style(Tone::Neutral, false, status)
}

pub fn button_danger(_theme: &Theme, status: button::Status) -> button::Style {
    button_style(Tone::Danger, true, status)
}

pub fn button_nav(_theme: &Theme, status: button::Status) -> button::Style {
    button_style(Tone::Neutral, false, status)
}

pub fn button_nav_active(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button_style(Tone::Primary, false, status);
    style.background = Some(Background::Color(match status {
        button::Status::Active => tint(ACCENT, 0.16),
        button::Status::Hovered => tint(ACCENT, 0.22),
        button::Status::Pressed => tint(ACCENT, 0.28),
        button::Status::Disabled => tint(ACCENT, 0.10),
    }));
    style.border = border(tint(ACCENT_HI, 0.52), 1.0, RADIUS_MEDIUM);
    style
}

pub fn text_input_default(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    input_style(status, false)
}

pub fn text_input_search(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    input_style(status, true)
}

pub fn surface_style(kind: SurfaceKind) -> container::Style {
    match kind {
        SurfaceKind::Base => container_style(
            SURFACE_BASE,
            LINE_SUBTLE,
            RADIUS_LARGE,
            shadow(Color::from_rgba8(0, 0, 0, 0.14), 0.0, 8.0, 24.0),
        ),
        SurfaceKind::Raised => container_style(
            SURFACE_RAISED,
            LINE_STRONG,
            RADIUS_LARGE,
            shadow(Color::from_rgba8(0, 0, 0, 0.18), 0.0, 12.0, 30.0),
        ),
        SurfaceKind::Accent => container_style(
            SURFACE_ACCENT,
            tint(ACCENT, 0.55),
            RADIUS_LARGE,
            shadow(Color::from_rgba8(0, 0, 0, 0.18), 0.0, 10.0, 24.0),
        ),
        SurfaceKind::Outline => container_style(
            Color::from_rgba8(255, 255, 255, 0.02),
            LINE_SUBTLE,
            RADIUS_MEDIUM,
            Shadow::default(),
        ),
        SurfaceKind::Toast => container_style(
            BACKGROUND_ELEVATED,
            LINE_STRONG,
            RADIUS_MEDIUM,
            shadow(Color::from_rgba8(0, 0, 0, 0.18), 0.0, 10.0, 26.0),
        ),
        SurfaceKind::Backdrop => container::Style {
            text_color: None,
            background: Some(Background::Color(Color::from_rgba8(9, 11, 15, 0.72))),
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

pub fn chip_style(tone: Tone) -> container::Style {
    let (background, border_color, text_color) = match tone {
        Tone::Neutral => (
            Color::from_rgba8(255, 255, 255, 0.05),
            LINE_SUBTLE,
            TEXT_MUTED,
        ),
        Tone::Primary => (tint(ACCENT, 0.16), tint(ACCENT_HI, 0.42), ACCENT_HI),
        Tone::Success => (tint(SUCCESS, 0.16), tint(SUCCESS, 0.42), SUCCESS),
        Tone::Warning => (tint(WARNING, 0.16), tint(WARNING, 0.42), WARNING),
        Tone::Danger => (tint(DANGER, 0.16), tint(DANGER, 0.42), DANGER),
    };

    container::Style {
        text_color: Some(text_color),
        background: Some(Background::Color(background)),
        border: border(border_color, 1.0, RADIUS_SMALL),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn button_style(tone: Tone, filled: bool, status: button::Status) -> button::Style {
    let (base_fill, base_border, text) = match tone {
        Tone::Primary => (ACCENT, tint(ACCENT_HI, 0.82), TEXT_STRONG),
        Tone::Success => (SUCCESS, tint(SUCCESS, 0.88), TEXT_STRONG),
        Tone::Warning => (WARNING, tint(WARNING, 0.88), BACKGROUND),
        Tone::Danger => (DANGER, tint(DANGER, 0.88), TEXT_STRONG),
        Tone::Neutral => (SURFACE_RAISED, LINE_STRONG, TEXT),
    };

    let (background, border_color, text_color, shadow_color, offset_y) = match status {
        button::Status::Active => (
            if filled {
                base_fill
            } else {
                tint(base_fill, 0.06)
            },
            base_border,
            text_color_for_state(tone, filled, false),
            Color::from_rgba8(0, 0, 0, if filled { 0.18 } else { 0.10 }),
            if filled { 4.0 } else { 2.0 },
        ),
        button::Status::Hovered => (
            if filled {
                lift(base_fill, 0.06)
            } else {
                tint(ACCENT, 0.12)
            },
            if filled {
                lift(base_border, 0.08)
            } else {
                tint(ACCENT, 0.46)
            },
            text_color_for_state(tone, filled, true),
            Color::from_rgba8(0, 0, 0, if filled { 0.20 } else { 0.12 }),
            if filled { 6.0 } else { 3.0 },
        ),
        button::Status::Pressed => (
            if filled {
                deepen(base_fill, 0.08)
            } else {
                tint(ACCENT, 0.18)
            },
            if filled {
                deepen(base_border, 0.08)
            } else {
                tint(ACCENT, 0.58)
            },
            text,
            Color::from_rgba8(0, 0, 0, if filled { 0.14 } else { 0.09 }),
            1.0,
        ),
        button::Status::Disabled => (
            if filled {
                tint(base_fill, 0.32)
            } else {
                Color::from_rgba8(255, 255, 255, 0.02)
            },
            tint(base_border, 0.30),
            tint(text, 0.42),
            Color::TRANSPARENT,
            0.0,
        ),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: border(border_color, 1.0, RADIUS_MEDIUM),
        shadow: shadow(shadow_color, 0.0, offset_y, 18.0),
        snap: false,
    }
}

fn text_color_for_state(tone: Tone, filled: bool, hovered: bool) -> Color {
    if filled {
        return match tone {
            Tone::Warning => BACKGROUND,
            _ => TEXT_STRONG,
        };
    }

    if hovered { ACCENT_HI } else { TEXT }
}

fn input_style(status: text_input::Status, is_search: bool) -> text_input::Style {
    let base_background = if is_search {
        SURFACE_ACCENT
    } else {
        SURFACE_RAISED
    };

    let (background, border_color, placeholder, value, icon, selection) = match status {
        text_input::Status::Active => (
            base_background,
            if is_search {
                tint(ACCENT, 0.22)
            } else {
                LINE_SUBTLE
            },
            TEXT_SOFT,
            TEXT,
            TEXT_MUTED,
            tint(ACCENT, 0.34),
        ),
        text_input::Status::Hovered => (
            lift(base_background, 0.02),
            if is_search {
                tint(ACCENT, 0.40)
            } else {
                LINE_STRONG
            },
            TEXT_MUTED,
            TEXT_STRONG,
            TEXT,
            tint(ACCENT, 0.40),
        ),
        text_input::Status::Focused { .. } => (
            lift(base_background, 0.03),
            tint(ACCENT_HI, 0.70),
            TEXT_MUTED,
            TEXT_STRONG,
            ACCENT_HI,
            tint(ACCENT, 0.48),
        ),
        text_input::Status::Disabled => (
            tint(base_background, 0.75),
            tint(LINE_SUBTLE, 0.45),
            tint(TEXT_SOFT, 0.55),
            tint(TEXT_MUTED, 0.55),
            tint(TEXT_SOFT, 0.55),
            tint(ACCENT, 0.18),
        ),
    };

    text_input::Style {
        background: Background::Color(background),
        border: border(border_color, 1.0, RADIUS_MEDIUM),
        icon,
        placeholder,
        value,
        selection,
    }
}

fn container_style(
    background: Color,
    border_color: Color,
    radius: f32,
    shadow: Shadow,
) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
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

fn shadow(color: Color, x: f32, y: f32, blur_radius: f32) -> Shadow {
    Shadow {
        color,
        offset: Vector::new(x, y),
        blur_radius,
    }
}

fn tint(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn lift(color: Color, delta: f32) -> Color {
    Color::from_rgba(
        (color.r + delta).min(1.0),
        (color.g + delta).min(1.0),
        (color.b + delta).min(1.0),
        color.a,
    )
}

fn deepen(color: Color, delta: f32) -> Color {
    Color::from_rgba(
        (color.r - delta).max(0.0),
        (color.g - delta).max(0.0),
        (color.b - delta).max(0.0),
        color.a,
    )
}
