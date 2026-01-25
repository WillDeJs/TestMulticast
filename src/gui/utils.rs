use cosmic::{
    Theme,
    iced::{Border, Color, border, widget::container::Style},
};

pub fn gui_util_bordered_style(theme: &Theme) -> Style {
    let border_color = match theme.theme_type {
        cosmic::theme::ThemeType::Light => Color::from_rgb(0.8, 0.8, 0.8),
        _ => Color::from_rgb(0.3, 0.3, 0.3),
    };
    Style {
        border: Border {
            color: border_color,
            width: 1.0,
            radius: border::Radius::new(3.0),
        },
        ..Style::default()
    }
}
