use iced::widget::svg;

pub const BRAND: &[u8] = include_bytes!("../../assets/icons/brand-book.svg");
pub const PLUS: &[u8] = include_bytes!("../../assets/icons/plus.svg");
pub const HOME: &[u8] = include_bytes!("../../assets/icons/home.svg");
pub const FAVORITE: &[u8] = include_bytes!("../../assets/icons/favorite-outline.svg");
pub const FAVORITE_FILLED: &[u8] = include_bytes!("../../assets/icons/favorite-filled.svg");
pub const ABOUT: &[u8] = include_bytes!("../../assets/icons/about.svg");
pub const SETTINGS: &[u8] = include_bytes!("../../assets/icons/settings.svg");
pub const THEME: &[u8] = include_bytes!("../../assets/icons/theme.svg");
pub const EDIT: &[u8] = include_bytes!("../../assets/icons/edit.svg");
pub const APPRECIATION: &[u8] = include_bytes!("../../assets/icons/appreciation.svg");
pub const CLOSE: &[u8] = include_bytes!("../../assets/icons/close.svg");
pub const SEARCH: &[u8] = include_bytes!("../../assets/icons/search.svg");

pub fn svg_handle(bytes: &'static [u8]) -> svg::Handle {
    svg::Handle::from_memory(bytes)
}
