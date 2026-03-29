use std::fmt::Display;

use crate::gui::Message;
use crate::net::util::*;
use chrono::{DateTime, Local};
use cosmic::{iced::Length, widget::table};
use csvlib::{CsvError, DocEntry, Row};

pub const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S.%3f";

#[derive(Clone, Debug, PartialEq)]
pub struct MulticastMessage {
    pub(crate) time_stamp: DateTime<Local>,
    pub(crate) src: String,
    pub(crate) bytes: Vec<u8>,
}
impl From<MulticastMessage> for Row {
    fn from(value: MulticastMessage) -> Self {
        let timestamp = value.time_stamp.timestamp_millis().to_string();
        let source = value.src;
        let data = value
            .bytes
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .join("");
        csvlib::csv![timestamp, source, data]
    }
}

impl TryFrom<DocEntry<'_>> for MulticastMessage {
    type Error = CsvError;

    fn try_from(value: DocEntry) -> Result<Self, Self::Error> {
        let timestamp_millis: i64 = value.get("TIMESTAMP")?;
        let source: String = value.get("SOURCE")?;
        let data_str: String = value.get("DATA")?;

        let time_stamp = DateTime::<Local>::from(
            std::time::UNIX_EPOCH + std::time::Duration::from_millis(timestamp_millis as u64),
        );
        if data_str.chars().any(|c| !c.is_ascii_hexdigit()) {
            Err(CsvError::Generic("DATA field contains non-hexadecimal characters".into()))
        } else {
            let bytes = (0..data_str.len())
                .step_by(2)
                // okay to unwrap here because we already checked for non-hex characters, and the length is guaranteed to be even
                .map(|i| u8::from_str_radix(&data_str[i..i + 2], 16).unwrap_or_default())
                .collect::<Vec<u8>>();

            Ok(MulticastMessage {
                time_stamp,
                src: source,
                bytes,
            })
        }
    }
}
impl table::ItemCategory for MulticastTableHeader {
    fn width(&self) -> cosmic::iced::Length {
        match self {
            MulticastTableHeader::Time | MulticastTableHeader::TimeInv => Length::Fixed(200.),
            MulticastTableHeader::Src | MulticastTableHeader::SrcInv => Length::Fixed(200.),
            MulticastTableHeader::Data | MulticastTableHeader::DataInv => Length::FillPortion(50),
        }
    }
}

impl table::ItemInterface<MulticastTableHeader> for MulticastMessage {
    fn get_icon(&self, _category: MulticastTableHeader) -> Option<cosmic::widget::Icon> {
        None
    }

    fn get_text(&self, category: MulticastTableHeader) -> std::borrow::Cow<'static, str> {
        match category {
            MulticastTableHeader::Time | MulticastTableHeader::TimeInv => {
                std::borrow::Cow::Owned(self.time_stamp.format(TIME_FORMAT).to_string())
            }
            MulticastTableHeader::Data | MulticastTableHeader::DataInv => {
                std::borrow::Cow::Owned(net_util_data_hexdump(&self.bytes))
            }
            MulticastTableHeader::Src | MulticastTableHeader::SrcInv => {
                std::borrow::Cow::Owned(self.src.clone())
            }
        }
    }

    fn compare(&self, other: &Self, category: MulticastTableHeader) -> std::cmp::Ordering {
        match category {
            MulticastTableHeader::Time | MulticastTableHeader::TimeInv => {
                self.time_stamp.cmp(&other.time_stamp)
            }
            MulticastTableHeader::Data | MulticastTableHeader::DataInv => {
                self.bytes.cmp(&other.bytes)
            }
            MulticastTableHeader::Src | MulticastTableHeader::SrcInv => self.src.cmp(&other.src),
        }
    }
}

impl MulticastTableHeader {
    // Invisible headers
    pub const ALL_INV: [Self; 3] = [
        MulticastTableHeader::TimeInv,
        MulticastTableHeader::SrcInv,
        MulticastTableHeader::DataInv,
    ];
    // visible headers
    pub const ALL_VIS: [Self; 3] = [
        MulticastTableHeader::Time,
        MulticastTableHeader::Src,
        MulticastTableHeader::Data,
    ];
}
impl Display for MulticastTableHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulticastTableHeader::Time => write!(f, "Time"),
            MulticastTableHeader::Src => write!(f, "Source"),
            MulticastTableHeader::Data => write!(f, "Data"),
            MulticastTableHeader::TimeInv => write!(f, ""),
            MulticastTableHeader::SrcInv => write!(f, ""),
            MulticastTableHeader::DataInv => write!(f, ""),
        }
    }
}
#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MulticastTableHeader {
    #[default]
    Time,
    Src,
    Data,
    TimeInv,
    SrcInv,
    DataInv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableAction {
    None,
}
impl cosmic::widget::menu::Action for TableAction {
    type Message = Message;
    fn message(&self) -> Self::Message {
        match self {
            TableAction::None => Message::NoOp,
        }
    }
}
