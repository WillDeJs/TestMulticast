use cosmic::{
    Theme,
    iced::{Border, Color, border, widget::container::Style},
};

use crate::{data::MulticastMessage, gui::Message};

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

pub fn gui_util_save_data_to_csv(rows: Vec<MulticastMessage>, url: String) -> Message {
    
    let mut csv_doc = csvlib::Document::with_headers(&["TIMESTAMP", "SOURCE", "DATA"]);
    for row in rows {
        csv_doc.insert(row);
    }
    match csv_doc.write_to_file(&url) {
        Ok(_) => Message::NoOp,
        Err(e) => Message::ShowError(format!("File save error: {}", e.to_string()))
    }
}

pub fn gui_util_load_data_from_csv(url: String) -> Message {
    if let Ok(csv_doc) = csvlib::Document::from_path(&url) {
        let rows = csv_doc.rows_decoded::<MulticastMessage>().flatten().collect();
        Message::DataLoaded(rows)

    } else {
        Message::ShowError("Failed to load file.".to_string())
    }
}