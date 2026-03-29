use std::collections::HashMap;

use crate::{
    data::{self, MulticastMessage, MulticastTableHeader, TableAction},
    gui::utils::gui_util_bordered_style,
    net::{
        ListenerEvent, listener,
        util::{net_util_data_ascii, net_util_data_hexdump},
    },
};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::widget::menu::action::MenuAction;
use cosmic::{
    Action,
    iced_widget::scrollable::RelativeOffset,
    widget::{RcElementWrapper, table, table::model, warning},
};
use cosmic::{
    ApplicationExt, Apply, Core, Task,
    iced::{
        Alignment, Font, Length,
        futures::{SinkExt, channel::mpsc::Sender},
    },
    iced_core::Element,
    iced_widget::toggler,
    task,
    widget::{
        button::{self, destructive},
        column::column,
        container,
        menu::{self, KeyBind},
        row::row,
        search_input, text, text_editor, text_input,
    },
};
use cosmic::{iced::id, widget::scrollable};

const MAX_PACKET_COUNT: usize = 1000;
pub struct App {
    core: Core,
    ip_address: String,
    port: String,
    ttl: String,
    send_data: String,
    ascii_output: text_editor::Content,
    table_header: table::SingleSelectModel<MulticastMessage, MulticastTableHeader>,
    results_table_model: table::SingleSelectModel<MulticastMessage, MulticastTableHeader>,
    detailed_output: text_editor::Content,
    registered: bool,
    interfaces_connected: usize,
    search_query: String,
    auto_scroll: bool,
    sender: Option<Sender<ListenerEvent>>,
    dialog_type: DialogType,
    dialog_message: String,
    all_rows: Vec<MulticastMessage>,
    warning: Option<String>,
    showing_count: usize,
    showing_details: bool,
    has_unsaved_changes: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    IpChange(String),
    PortChange(String),
    TestDataChange(String),
    TtlChange(String),
    ItemSelect(table::Entity),
    CategorySelect(MulticastTableHeader),
    Register,
    Unregister,
    SendData,
    NoOp,
    Empty,
    NewRow(MulticastMessage),
    RegisterFail(String),
    ShowError(String),
    RegisterSuccess(usize),
    Disconnected,
    ChangeAutoScroll(bool),
    SearchChange(String),
    SearchQuery(String),
    Ready(Sender<ListenerEvent>),
    CloseDialog,
    QueryEdit(String),
    DetailedOutputEdit(text_editor::Action),
    AsciiOutputEdit(text_editor::Action),
    CloseWarning,
    DataSave,
    DataLoad,
    OutputFileSelected(String),
    LoadFile(String),
    DataLoaded(Vec<MulticastMessage>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileMenuAction {
    DataSave,
    DataLoad,
}

impl MenuAction for FileMenuAction {
    type Message = Message;
    fn message(&self) -> Self::Message {
        match self {
            FileMenuAction::DataSave => Message::DataSave,
            FileMenuAction::DataLoad => Message::DataLoad,
        }
    }
}
#[allow(unused)]
#[derive(Debug, Clone, Default)]
enum DialogType {
    Warning,
    Info,
    Error,
    #[default]
    None,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.mctest.app";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }
    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            Message::IpChange(value) => {
                // only allow numbers and digits for IP
                self.ip_address = value
                    .chars()
                    .filter(|c| c.is_numeric() || *c == '.')
                    .collect();
            }
            Message::PortChange(port) => {
                // only allow numbers for port
                self.port = port.chars().filter(|c| c.is_numeric()).collect();
            }
            Message::TtlChange(ttl) => {
                self.ttl = ttl.chars().filter(|c| c.is_numeric()).collect();
            }
            Message::TestDataChange(data) => self.send_data = data,
            Message::ItemSelect(entity) => {
                let id = self.results_table_model.active();
                // toggle details on and off if clicked again on same row
                if entity == id {
                    model::selection::Selectable::deactivate(&mut self.results_table_model, id);
                    self.detailed_output = text_editor::Content::new();
                    self.ascii_output = text_editor::Content::new();
                    self.showing_details = false;
                } else {
                    self.results_table_model.activate(entity);
                    if let Some(item) = self.results_table_model.item(entity) {
                        let detailed_data = format!(
                            "> At: {}\n> From: {}\n> Length: {} byte(s)\n> Hex Dump: {}",
                            item.time_stamp.format(data::TIME_FORMAT),
                            item.src,
                            item.bytes.len(),
                            &net_util_data_hexdump(&item.bytes)
                        );
                        self.detailed_output = text_editor::Content::with_text(&detailed_data);
                        self.ascii_output =
                            text_editor::Content::with_text(&net_util_data_ascii(&item.bytes));
                        self.showing_details = true;
                    }
                }
            }
            Message::CategorySelect(_) => {}
            Message::Register => {
                if self.sender.is_some() {
                    let ip = self.ip_address.clone();
                    let port = self.port.clone();
                    let ttl = self.ttl.clone();
                    let mut sender = self.sender.as_mut().unwrap().clone();
                    return task::future(async move {
                        let _ = sender.send(ListenerEvent::Register(ip, port, ttl)).await;
                        Message::NoOp
                    });
                }
            }
            Message::Unregister => {
                if self.sender.is_some() {
                    let mut sender = self.sender.as_mut().unwrap().clone();
                    return task::future(async move {
                        let _ = sender.send(ListenerEvent::Stop).await;
                        Message::NoOp
                    });
                }
            }
            Message::SendData => {
                if self.sender.is_some() && !self.send_data.trim().is_empty() {
                    let ip = self.ip_address.clone();
                    let port = self.port.clone();
                    let data = self.send_data.clone();
                    let mut sender = self.sender.as_mut().unwrap().clone();
                    return task::future(async move {
                        let _ = sender.send(ListenerEvent::SendData(ip, port, data)).await;
                        Message::NoOp
                    });
                }
            }
            Message::NoOp => {}
            Message::Empty => {
                self.all_rows.clear();
                self.results_table_model.clear();
                self.detailed_output = text_editor::Content::new();
                self.ascii_output = text_editor::Content::new();
                self.showing_count = 0;
                self.showing_details = false;
            }
            Message::NewRow(multicast_message) => {
                let mut inserted = false;
                self.all_rows.push(multicast_message.clone());
                let query = self.search_query.trim().to_lowercase();
                if query.is_empty() {
                    self.showing_count += 1;
                    let _ = self.results_table_model.insert(multicast_message);
                    inserted = true;
                } else {
                    let string_time = multicast_message
                        .time_stamp
                        .format(data::TIME_FORMAT)
                        .to_string();
                    let string_data = String::from_utf8_lossy(&multicast_message.bytes)
                        .to_string()
                        .to_lowercase();
                    if string_time.contains(&query)
                        || string_data.contains(&query)
                        || multicast_message.src.to_lowercase().contains(&query)
                    {
                        let _ = self.results_table_model.insert(multicast_message);
                        self.showing_count += 1;
                        inserted = true;
                    }
                }
                let scroll_task = if self.auto_scroll && inserted {
                    cosmic::iced::widget::scrollable::snap_to(
                        id::Id::new("results-table"),
                        RelativeOffset::END,
                    )
                    .map(Action::App)
                } else {
                    Task::none()
                };
                let warning_task = if self.all_rows.iter().len() >= MAX_PACKET_COUNT {
                    self.warning = Some(format!(
                        "Packet capture stopped as more than {MAX_PACKET_COUNT} have been received."
                    ));
                    Task::done(Message::Unregister).map(Action::App)
                } else {
                    Task::none()
                };
                return Task::batch([scroll_task, warning_task]);
            }
            Message::DataLoaded(rows) => {
                self.all_rows = rows.clone();
                self.results_table_model.clear();
                self.search_query.clear();
                for row in rows {
                    let _ = self.results_table_model.insert(row);
                }
                self.showing_count = self.all_rows.len();
            }
            Message::RegisterFail(error) => {
                self.dialog_message.push_str(&error);
                self.dialog_type = DialogType::Error;
            }
            Message::RegisterSuccess(connections) => {
                self.all_rows.clear();
                self.results_table_model.clear();
                self.interfaces_connected = connections;
                self.registered = true;
                self.warning = None;
                self.detailed_output = text_editor::Content::new();
                self.ascii_output = text_editor::Content::new();
                self.showing_count = 0;
                self.showing_details = false;
            }
            Message::Disconnected => {
                self.registered = false;
            }
            Message::ShowError(error) => {
                self.dialog_message.push_str(&error);
                self.dialog_type = DialogType::Error;
            }
            Message::Ready(sender) => {
                self.sender = Some(sender);
            }
            Message::CloseDialog => {
                self.dialog_type = DialogType::None;
                self.dialog_message.clear();
            }
            Message::CloseWarning => {
                self.warning = None;
            }
            Message::ChangeAutoScroll(value) => {
                self.auto_scroll = value;
            }
            Message::SearchChange(query) => self.search_query = query,
            Message::SearchQuery(query) => {
                let query = query.trim().to_lowercase();
                self.showing_count = 0;
                self.results_table_model.clear();
                for row in &self.all_rows {
                    let string_time = row.time_stamp.format(data::TIME_FORMAT).to_string();
                    let string_data = String::from_utf8_lossy(&row.bytes).to_string();
                    if string_time.contains(&query)
                        || string_data.to_lowercase().contains(&query)
                        || row.src.to_lowercase().contains(&query)
                    {
                        let _ = self.results_table_model.insert(row.clone());
                        self.showing_count += 1;
                    }
                }
            }
            Message::QueryEdit(query) => self.search_query = query,
            Message::DetailedOutputEdit(action) => {
                if !action.is_edit() {
                    self.detailed_output.perform(action);
                }
            }
            Message::AsciiOutputEdit(action) => {
                if !action.is_edit() {
                    self.ascii_output.perform(action);
                }
            }
            Message::DataSave => {
                return cosmic::task::future(async move {
                    let filter = FileFilter::new("CSV File").extension("csv");
                    let dialog = file_chooser::save::Dialog::new()
                        .title("Save captured data".to_owned())
                        .filter(filter);
                    match dialog.save_file().await {
                        Ok(response) => {
                            if let Some(url) = response.url() {
                                Message::OutputFileSelected(url.path().to_owned())
                            } else {
                                Message::ShowError("File not created".to_string())
                            }
                        }
                        Err(cosmic::dialog::file_chooser::Error::Cancelled) => Message::NoOp,
                        Err(e) => Message::ShowError(format!("File save error: {}", e.to_string())),
                    }
                });
            }
            Message::DataLoad => {
                return cosmic::task::future(async move {
                    let filter = FileFilter::new("CSV File").extension("csv");
                    let dialog = file_chooser::open::Dialog::new()
                        .title("Load captured data".to_owned())
                        .filter(filter);
                    match dialog.open_file().await {
                        Ok(response) => Message::LoadFile(response.url().path().to_owned()),
                        Err(cosmic::dialog::file_chooser::Error::Cancelled) => Message::NoOp,
                        Err(e) => Message::ShowError(format!("File load error: {}", e.to_string())),
                    }
                });
            }
            Message::OutputFileSelected(url) => {
                let rows = self.all_rows.clone();

                return cosmic::task::future(async move {
                    // Message::NoOp
                    let result = tokio::task::spawn_blocking(move || {
                        crate::gui::utils::gui_util_save_data_to_csv(rows, url)
                    })
                    .await;
                    result.unwrap_or(Message::ShowError("Failed to save file.".to_string()))
                })
                .map(Action::App);
            }
            Message::LoadFile(url) => {
                let rows = self.all_rows.clone();

                return cosmic::task::future(async move {
                    // Message::NoOp
                    let result = tokio::task::spawn_blocking(move || {
                        let rows = crate::gui::utils::gui_util_load_data_from_csv(url);
                        rows
                    })
                    .await;
                    result.unwrap_or(Message::ShowError("Failed to load file.".to_string()))
                })
                .map(Action::App);
            }
        }

        Task::none()
    }

    fn init(core: cosmic::Core, _flags: Self::Flags) -> (Self, cosmic::app::Task<Self::Message>) {
        let mut app = App {
            core,
            ip_address: "".to_owned(),
            port: "".to_owned(),
            ttl: "255".to_owned(),
            send_data: "".to_owned(),
            ascii_output: text_editor::Content::new(),
            table_header: table::Model::new(MulticastTableHeader::ALL_VIS.to_vec()),
            results_table_model: table::Model::new(MulticastTableHeader::ALL_INV.to_vec()),
            detailed_output: text_editor::Content::new(),
            registered: false,
            search_query: "".to_owned(),
            auto_scroll: true,
            sender: None,
            dialog_type: DialogType::default(),
            dialog_message: String::new(),
            all_rows: Vec::new(),
            warning: None,
            interfaces_connected: 0,
            showing_count: 0,
            showing_details: false,
            has_unsaved_changes: false,
        };
        app.results_table_model
            .sort(MulticastTableHeader::Time, false);
        app.set_header_title("Multicast Test".to_owned());
        (app, Task::none())
    }

    fn header_start(&self) -> Vec<cosmic::Element<'_, Self::Message>> {
        let menu = menu::bar(vec![menu::Tree::with_children(
            RcElementWrapper::new(Element::from(menu::root("File"))),
            menu::items(
                &HashMap::new(),
                vec![
                    menu::Item::Button("Save Data", None, FileMenuAction::DataSave),
                    menu::Item::Button("Load Data", None, FileMenuAction::DataLoad),
                ],
            ),
        )]);

        vec![menu.into()]
    }
    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        cosmic::iced::Subscription::run(listener::Listener::start)
    }

    fn view(&self) -> cosmic::Element<'_, Self::Message> {
        let mut ip_text_input = text_input("Multicast IP Address", &self.ip_address);
        let mut port_number_input = text_input("1234", &self.port);
        let mut ttl_input = text_input("1234", &self.ttl);

        if !self.registered {
            ip_text_input = ip_text_input.on_input(Message::IpChange);
            port_number_input = port_number_input.on_input(Message::PortChange);
            ttl_input = ttl_input.on_input(Message::TtlChange)
        }
        let register_button = if self.registered {
            button::destructive("Unregister").on_press(Message::Unregister)
        } else {
            button::standard("Register").on_press(Message::Register)
        };
        let send_command = if self.registered {
            Some(Message::SendData)
        } else {
            None
        };
        let send_data_button = button::suggested("Send")
            .spacing(50)
            .on_press_maybe(send_command);
        let data_text_input = text_input("Type ASCII data to send", &self.send_data)
            .on_input(Message::TestDataChange);
        let row_ascii_view = text_editor(&self.ascii_output)
            .font(Font::MONOSPACE)
            .wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph)
            .on_action(Message::AsciiOutputEdit);
        let row_detailed_view = text_editor(&self.detailed_output)
            .font(Font::MONOSPACE)
            .wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph)
            .on_action(Message::DetailedOutputEdit);
        let search_command = if !self.all_rows.is_empty() {
            Some(Message::SearchQuery)
        } else {
            None
        };
        let search_bar = search_input(
            "Type any pattern to be searched in all columns.",
            &self.search_query,
        )
        .on_input(Message::QueryEdit)
        .on_submit_maybe(search_command);
        let auto_scroll_toggle = toggler(self.auto_scroll).on_toggle(Message::ChangeAutoScroll);
        let clear_button = destructive("Empty").on_press(Message::Empty);

        // This is part of an ugly hack, the table widget
        // is not scrollable, so we wrap it it in a scrollable
        // Unfortunately, that makes the header scrollable
        // So we have two table widgets,
        // * One shows the header
        // * One shows the data
        let results_header = table(&self.table_header).category_context(|_category| {
            Some(menu::items(
                &HashMap::<KeyBind, TableAction>::new(),
                Vec::<menu::Item<TableAction, String>>::new(),
            ))
        });
        let results_table = table(&self.results_table_model)
            .on_item_left_click(Message::ItemSelect)
            .on_category_left_click(Message::CategorySelect)
            .category_context(|_category| {
                Some(menu::items(
                    &HashMap::<KeyBind, TableAction>::new(),
                    Vec::<menu::Item<TableAction, String>>::new(),
                ))
            })
            .item_context(|_item| {
                Some(menu::items(
                    &HashMap::<KeyBind, TableAction>::new(),
                    Vec::<menu::Item<TableAction, String>>::new(),
                ))
            })
            .apply(Element::from);
        let warning = match &self.warning {
            Some(message) => warning(message)
                .on_close(Message::CloseWarning)
                .apply(Element::from),
            None => container("").apply(Element::from),
        };
        column()
            .push(
                row()
                    .push(text("Multicast Address: ").width(150))
                    .push(ip_text_input.width(Length::FillPortion(70)))
                    .push("Port: ")
                    .push(port_number_input.width(Length::FillPortion(30)))
                    .push(text("TTL:"))
                    .push(ttl_input.width(Length::FillPortion(30)))
                    .push(register_button.width(90))
                    .spacing(10),
            )
            .push(
                row()
                    .push(text("Test Data: ").width(150))
                    .push(data_text_input.width(Length::FillPortion(60)))
                    .push(send_data_button.width(90))
                    .spacing(10),
            )
            .push(
                column()
                    .push(container(search_bar).center_x(Length::Fill))
                    .push(results_header.height(Length::Shrink))
                    .push(
                        container(
                            scrollable(results_table)
                                .height(Length::FillPortion(90))
                                .id(id::Id::new("results-table")),
                        )
                        .style(gui_util_bordered_style),
                    )
                    .spacing(15),
            )
            .push(
                row()
                    .push(text("Auto-Scroll"))
                    .push(auto_scroll_toggle)
                    .push(text(format!("Captured: {} Packets", self.all_rows.len())))
                    .push(text(format!("Showing: {} Packets", self.showing_count)))
                    .push(text(format!("Interfaces: {}.", self.interfaces_connected)))
                    .push(row().width(Length::Fill))
                    .push(clear_button)
                    .spacing(30)
                    .align_y(Alignment::Center),
            )
            .push(if self.showing_details {
                row()
                    .push(row_detailed_view.height(200))
                    .push(row_ascii_view.height(200))
            } else {
                row()
            })
            .push(warning)
            .spacing(10)
            .padding(10)
            .into()
    }

    fn dialog(&self) -> Option<cosmic::Element<'_, Self::Message>> {
        match self.dialog_type {
            DialogType::Info => {
                let dialog = cosmic::widget::dialog::dialog()
                    .title("Information")
                    .body(&self.dialog_message)
                    // .icon(icon::from_name("dialog-information-symbolic"))
                    .primary_action(button::text("Ok").on_press(Message::CloseDialog))
                    .into();
                Some(dialog)
            }
            DialogType::Warning => {
                let dialog = cosmic::widget::dialog::dialog()
                    .title("Warning")
                    .body(&self.dialog_message)
                    // .icon(icon::from_name("dialog-warning-symbolic"))
                    .primary_action(button::text("Ok").on_press(Message::CloseDialog))
                    .into();
                Some(dialog)
            }
            DialogType::Error => {
                let dialog = cosmic::widget::dialog::dialog()
                    .title("Error")
                    .body(&self.dialog_message)
                    .primary_action(button::text("Ok").on_press(Message::CloseDialog))
                    // .icon(icon::from_name("dialog-error-symbolic"))
                    .into();
                Some(dialog)
            }
            DialogType::None => None,
        }
    }
}
