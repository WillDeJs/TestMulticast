use cosmic::iced::futures::select;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;
use tokio::io::ErrorKind;
use tokio::net::UdpSocket;

use cosmic::iced::futures::channel::mpsc::{self, Receiver};
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
    CommError(String),
    RowReceived(MulticastMessage),
    SendEvent(String, String, String), // (ip, port, data)
}
pub struct Listener;

impl Listener {
    pub fn start() -> impl Stream<Item = Message> {
        stream::channel(100, |mut output| async move {
            let _ = tokio::task::spawn_blocking(|| async move {
                let (tx, mut rx) = mpsc::channel::<ListenerEvent>(100);
                println!("Waiting for first event");
                let _ = output.send(Message::Ready(tx)).await;
                let mut buffer = vec![0; MAX_BUFFER_LEN];
                loop {
                    let event = rx.select_next_some().await;
                    match event {
                        ListenerEvent::Register(ip, port, ttl) => {
                            let socket = match Self::multicast_registration(&ip, &port, &ttl) {
                                Ok(socket) => {
                                    let _ = output.send(Message::RegisterSuccess).await;
                                    socket
                                }
                                Err(error) => {
                                    let _ = output
                                        .send(Message::RegisterFail(format!(
                                            "Could not complete registration. Error: `{error}`."
                                        )))
                                        .await;
                                    continue;
                                }
                            };
                            let socket =
                                match UdpSocket::from_std(std::net::UdpSocket::from(socket)) {
                                    Ok(socket) => socket,
                                    Err(e) => {
                                        let _ =
                                            output.send(Message::RegisterFail(e.to_string())).await;
                                        break;
                                    }
                                };
                            loop {
                                let event_handle = Self::check_for_gui_events(&mut rx);
                                let data_handle = Self::handle_data_receive(&socket, &mut buffer);
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
                                    NetStatus::RowReceived(row) => {
                                        let _ = output.send(Message::NewRow(row)).await;
                                    }
                                    NetStatus::SendEvent(ip, port, data) => {
                                        if let Err(e) = socket
                                            .send_to(&data.as_bytes(), &format!("{ip}:{port}"))
                                            .await
                                        {
                                            let _ = output
                                                .send(Message::CommError(e.to_string()))
                                                .await;
                                        }
                                    }
                                    NetStatus::Stop => {
                                        // let the GUI know we are disconnected
                                        let _ = output.send(Message::Disconnected).await;
                                        // exit out of this event check loop, wait for a new registration
                                        break;
                                    }
                                    NetStatus::CommError(error) => {
                                        let _ = output.send(Message::CommError(error)).await;
                                    }
                                }

                                // Sleep a little bit so we don't overload the thread
                                tokio::time::sleep(Duration::from_millis(50)).await;
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

    async fn handle_data_receive(socket: &UdpSocket, buffer: &mut Vec<u8>) -> NetStatus {
        match socket.recv_from(buffer).await {
            Ok((read, src)) => {
                let time_stamp = chrono::Local::now();
                let bytes = buffer[0..read].to_vec();
                let src = src.to_string();
                let multicast_message = MulticastMessage {
                    time_stamp,
                    src,
                    bytes,
                };
                NetStatus::RowReceived(multicast_message)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                // our socket is non-blocking, we simply continue
                NetStatus::Continue
            }
            Err(error) => NetStatus::CommError(error.to_string()),
        }
    }
    async fn check_for_gui_events(rx: &mut Receiver<ListenerEvent>) -> NetStatus {
        println!("Handle event receive...");
        let event = rx.select_next_some().await;

        match event {
            ListenerEvent::Register(_, _, _) => NetStatus::Continue,
            ListenerEvent::SendData(ip, port, data) => NetStatus::SendEvent(ip, port, data),
            ListenerEvent::Stop => NetStatus::Stop,
        }
    }

    fn multicast_registration(ip: &str, port: &str, ttl: &str) -> Result<Socket, String> {
        let port: u16 = port.parse().map_err(|e: ParseIntError| e.to_string())?;
        let ttl: u32 = ttl.parse().map_err(|e: ParseIntError| e.to_string())?;
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| e.to_string())?;
        // Local address bind it to a local port in any available interface
        let local_address = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

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
            .join_multicast_v4(&multicast_address, &local_address.ip())
            .map_err(|e| e.to_string())?;

        // set non-blocking so that both our send and receive functions would work without contention (they share a socket)
        // this is needed because the select macro would poll both futures and we set socket to read, write a the same time
        // we could have a scenario on which the read end is suspended, and the write end can't work.
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(socket)
    }
}
