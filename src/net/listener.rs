use cosmic::iced::futures::select;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::time::Duration;
use tokio::io::ErrorKind;
use tokio::net::UdpSocket;

use cosmic::iced::futures::channel::mpsc::{self, Receiver, Sender};
use cosmic::iced::futures::{FutureExt, SinkExt, StreamExt};

use cosmic::iced::{futures::Stream, stream};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::data::MulticastMessage;
use crate::gui::Message;
const MAX_BUFFER_LEN: usize = 65000;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerEvent {
    Register(String, String, String),
    SendData(String, String, String),
    Stop,
}

#[derive(Debug)]
enum NetStatus {
    Continue,
    Stop,
}

struct NetworkConnection {
    name: String,
    ip: Ipv4Addr,
    socket: UdpSocket,
}

pub struct Listener;

impl Listener {
    pub fn start() -> impl Stream<Item = Message> {
        stream::channel(100, |mut output| async move {
            let _ = tokio::task::spawn_blocking(|| async move {
                let (tx, mut rx) = mpsc::channel::<ListenerEvent>(100);
                let _ = output.send(Message::Ready(tx)).await;
                let mut buffer = vec![0; MAX_BUFFER_LEN];
                loop {
                    let event = rx.select_next_some().await;
                    match event {
                        ListenerEvent::Register(ip, port, ttl) => {
                            let mut combined_error = String::new();
                            let all_interfaces = Self::get_all_ipv4_local_addresses();
                            let mut connections = Vec::<NetworkConnection>::new();
                            for (interface, local_ip) in all_interfaces.iter() {
                                let socket = match Self::multicast_registration_one_interface(
                                    &ip, &port, &ttl, local_ip,
                                ) {
                                    Ok(socket) => socket,
                                    Err(error) => {
                                        combined_error
                                            .push_str(&format!("{interface} -> {error}\n"));
                                        continue;
                                    }
                                };
                                connections.push(NetworkConnection {
                                    name: interface.clone(),
                                    ip: *local_ip,
                                    socket,
                                });
                            }
                            if connections.is_empty() {
                                let _ = output.send(Message::RegisterFail(combined_error)).await;
                                continue;
                            } else {
                                let _ = output
                                    .send(Message::RegisterSuccess(connections.len()))
                                    .await;
                            }

                            loop {
                                let event_handle = Self::check_for_gui_events(
                                    &connections,
                                    &mut rx,
                                    output.clone(),
                                );
                                let data_handle = Self::handle_data_receive(
                                    &connections,
                                    &mut buffer,
                                    output.clone(),
                                );
                                let result = select! {
                                    result = data_handle.fuse() => {
                                        result
                                    },
                                    result = event_handle.fuse() => {
                                       result
                                    },

                                };
                                match result {
                                    // should never receive row here
                                    NetStatus::Continue => (),
                                    NetStatus::Stop => {
                                        // let the GUI know we are disconnected
                                        let _ = output.send(Message::Disconnected).await;
                                        // exit out of this event check loop, wait for a new registration
                                        break;
                                    }
                                }

                                // Sleep a little bit so we don't overload the thread
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                        // nothing to do here, we only stop or send data when we are inside of the send loop
                        ListenerEvent::Stop | ListenerEvent::SendData(_, _, _) => (),
                    }
                }
            })
            .await
            .unwrap()
            .await;
        })
    }

    async fn handle_data_receive(
        connections: &Vec<NetworkConnection>,
        buffer: &mut Vec<u8>,
        mut output: Sender<Message>,
    ) -> NetStatus {
        for connection in connections {
            match connection.socket.try_recv_from(buffer) {
                Ok((read, src)) => {
                    let time_stamp = chrono::Local::now();
                    let bytes = buffer[0..read].to_vec();
                    let src = src.to_string();
                    let local_ip = connection.ip.to_string();
                    let interface = connection.name.clone();
                    let multicast_message = MulticastMessage {
                        time_stamp,
                        local_ip,
                        interface,
                        src,
                        bytes,
                    };
                    let _ = output.send(Message::NewRow(multicast_message)).await;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    // our socket is non-blocking, we simply continue
                    // NetStatus::Continue
                    continue;
                }
                Err(error) => {
                    let _ = output
                        .send(Message::CommError(format!(
                            "Interface: `{}`, Addr: `{}` -> Error: `{}`",
                            connection.name,
                            connection.ip,
                            error.to_string()
                        )))
                        .await;
                }
            }
        }
        NetStatus::Continue
    }
    async fn check_for_gui_events(
        connections: &Vec<NetworkConnection>,
        rx: &mut Receiver<ListenerEvent>,
        mut output: Sender<Message>,
    ) -> NetStatus {
        let event = rx.select_next_some().await;
        match event {
            ListenerEvent::Register(_, _, _) => NetStatus::Continue,
            ListenerEvent::SendData(ip, port, data) => {
                for connection in connections {
                    if let Err(e) = connection
                        .socket
                        .send_to(&data.as_bytes(), &format!("{ip}:{port}"))
                        .await
                    {
                        let _ = output
                            .send(Message::CommError(format!(
                                "Interface: `{}`, Addr: `{}` -> Error: `{}`",
                                connection.name,
                                connection.ip,
                                e.to_string()
                            )))
                            .await;
                    }
                }
                NetStatus::Continue
            }
            ListenerEvent::Stop => NetStatus::Stop,
        }
    }

    fn multicast_registration_one_interface(
        ip: &str,
        port: &str,
        ttl: &str,
        iface: &Ipv4Addr,
    ) -> Result<UdpSocket, String> {
        let port: u16 = port
            .parse()
            .map_err(|_| "Invalid port provided".to_owned())?;
        let ttl: u32 = ttl
            .parse()
            .map_err(|_| "invalid TTL value provided".to_owned())?;
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| e.to_string())?;
        // Local address bind it to a local port in any available interface
        let local_address = SocketAddrV4::new(iface.clone(), port);

        // Multicast address passed by the user
        let multicast_address = Ipv4Addr::from_str(&format!("{ip}")).map_err(|e| e.to_string())?;

        // allow reusing address (needed for MULTICAST)
        socket.set_reuse_address(true).map_err(|e| e.to_string())?;

        socket.set_ttl_v4(ttl).map_err(|e| e.to_string())?;

        // convert to socket2 struct so we can use it
        let bind_address = SockAddr::from(SocketAddr::V4(local_address));
        socket.bind(&bind_address).map_err(|e| e.to_string())?;

        // register on multicast group
        socket
            .join_multicast_v4(&multicast_address, &iface)
            .map_err(|e| e.to_string())?;

        // set non-blocking so that both our send and receive functions would work without contention (they share a socket)
        // this is needed because the select macro would poll both futures and we set socket to read, write a the same time
        // we could have a scenario on which the read end is suspended, and the write end can't work.
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;

        // Map to tokio socket so we can read it asynchronously
        let socket =
            UdpSocket::from_std(std::net::UdpSocket::from(socket)).map_err(|e| e.to_string())?;
        Ok(socket)
    }

    fn get_all_ipv4_local_addresses() -> HashMap<String, Ipv4Addr> {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let mut interface_ip_map = HashMap::<String, Ipv4Addr>::new();
        // interface_ip_map.insert("Localhost".to_owned(), Ipv4Addr::LOCALHOST);
        for (name, data) in networks.iter() {
            for network in data.ip_networks() {
                if let IpAddr::V4(address) = network.addr {
                    interface_ip_map.insert(name.clone(), address.clone());
                }
            }
        }
        interface_ip_map
    }
}
