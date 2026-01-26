use std::fmt::Display;

use crate::gui::Message;
use crate::net::util::*;
use chrono::{DateTime, Local};
use cosmic::{iced::Length, widget::table};

pub const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S.%3f";

#[derive(Clone, Debug, PartialEq)]
pub struct MulticastMessage {
    pub(crate) time_stamp: DateTime<Local>,
    pub(crate) src: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) local_ip: String,
    pub(crate) interface: String,
}
impl table::ItemCategory for MulticastTableHeader {
    fn width(&self) -> cosmic::iced::Length {
        match self {
            MulticastTableHeader::Time | MulticastTableHeader::TimeInv => Length::Fixed(200.),
            MulticastTableHeader::Src | MulticastTableHeader::SrcInv => Length::Fixed(200.),
            MulticastTableHeader::Interface | MulticastTableHeader::InterfaceInv => {
                Length::Fixed(200.)
            }
            MulticastTableHeader::LocalIp | MulticastTableHeader::LocalIpInv => Length::Fixed(200.),
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
            MulticastTableHeader::Interface | MulticastTableHeader::InterfaceInv => {
                std::borrow::Cow::Owned(self.interface.clone())
            }
            MulticastTableHeader::LocalIp | MulticastTableHeader::LocalIpInv => {
                std::borrow::Cow::Owned(self.local_ip.clone())
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
            MulticastTableHeader::Interface | MulticastTableHeader::InterfaceInv => {
                self.interface.cmp(&other.interface)
            }
            MulticastTableHeader::LocalIp | MulticastTableHeader::LocalIpInv => {
                self.local_ip.cmp(&other.local_ip)
            }
        }
    }
}

impl MulticastTableHeader {
    // Invisible headers
    pub const ALL_INV: [Self; 5] = [
        MulticastTableHeader::TimeInv,
        MulticastTableHeader::SrcInv,
        MulticastTableHeader::InterfaceInv,
        MulticastTableHeader::LocalIpInv,
        MulticastTableHeader::DataInv,
    ];
    // visible headers
    pub const ALL_VIS: [Self; 5] = [
        MulticastTableHeader::Time,
        MulticastTableHeader::Src,
        MulticastTableHeader::Interface,
        MulticastTableHeader::LocalIp,
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
            MulticastTableHeader::Interface => write!(f, "NIC"),
            MulticastTableHeader::InterfaceInv => write!(f, ""),
            MulticastTableHeader::LocalIp => write!(f, "Local NIC IP"),
            MulticastTableHeader::LocalIpInv => write!(f, ""),
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
    Interface,
    InterfaceInv,
    LocalIp,
    LocalIpInv,
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
