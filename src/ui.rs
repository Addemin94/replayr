// UI module for the replayr application using Iced framework.
// Handles user interface rendering, event handling, and state management.

use iced::widget::{
    button, column, container, radio, row, scrollable, text, text_input, tooltip, Space,
};
use iced::Length;
use iced::{exit, Background, Color, Element, Task, Theme};

use iced::window;
use std::collections::HashMap;
use crate::config::Config;
use crate::log::LOG_SENDER;
use crate::types::{
    LogMessage, PayloadType, ReplayData, ReplayablePayload, ReplayableSession, SessionCommand,
    SessionData, Window, WindowState,
};
use tokio::sync::mpsc;
use WindowState::Replay;
use WindowState::Session;

/// Messages representing user interactions and system events in the UI.
/// Each variant corresponds to a specific action or update in the application state.
#[derive(Debug, Clone)]
pub enum Message {
    /// User changed the server address in the main window.
    AddressChanged(window::Id, String),
    /// User changed the port in the main window.
    PortChanged(window::Id, String),
    /// User changed the initial payload in the main window.
    InitialPayloadChanged(window::Id, String),
    /// User changed the initial payload type in the main window.
    InitialPayloadTypeChanged(window::Id, PayloadType),
    /// User selected a different protocol (TCP/UDP).
    ProtocolChanged(window::Id, crate::types::Protocol),
    /// User initiated a connection or session open.
    Connect(window::Id),
    /// A new window has been opened.
    WindowOpened(window::Id),
    /// User input changed in a session window.
    InputChanged(window::Id, String),
    /// User changed payload type (Hex/ASCII).
    PayloadTypeChanged(window::Id, PayloadType),
    /// User requested to send a packet.
    SendPacket(window::Id),
    // CloseSession(window::Id), // Unused variant
    /// A window was closed.
    Closed(window::Id),
    /// Log message received for a specific window.
    LogReceived(String, window::Id),
    /// Log message for the main window.
    MainLog(String),
    /// Connection status update for a session.
    ConnectionStatus(window::Id, bool),
    // TitleChanged(window::Id, String), // Unused variant
    /// Export session data for replay.
    ExportSession(window::Id),
    /// Export logs to a file.
    ExportLogs(window::Id),
    /// User initiated replay connection by selecting a file.
    ReplayConnect,
    /// Replay window opened with loaded session data.
    ReplayWindowOpenedWithFile(ReplayableSession, String),
    /// Replay session has started.
    ReplayStarted(window::Id),
    /// Progress update during replay.
    ReplayProgress(window::Id, usize),
    /// Error occurred during replay.
    ReplayError(String),
    /// No operation (used for async task completion).
    NoOp,
}

/// Main application state holding configuration, logs, and window management.
/// This struct is the central state for the entire UI application.
#[derive(Clone)]
pub struct App {
    /// Application configuration (address, port, protocol, etc.).
    pub config: Config,
    /// Last used address for new sessions.
    pub last_addr: String,
    /// Last used port for new sessions.
    pub last_port: String,
    /// Last used initial payload for new sessions.
    pub last_payload: String,
    /// ID of the main configuration window.
    pub main_window_id: window::Id,
    /// Accumulated log messages for the main window.
    pub main_log: String,
    /// Map of window IDs to their respective Window data.
    pub windows: HashMap<window::Id, Window>,
}

/// Default implementation for App, initializing with default config and empty state.
impl Default for App {
    fn default() -> Self {
        Self {
            config: Config::default(),
            last_addr: String::new(),
            last_port: String::new(),
            last_payload: String::new(),
            main_window_id: window::Id::unique(),
            main_log: String::new(),
            windows: HashMap::new(),
        }
    }
}

/// Implementation for App, providing utility methods.
impl App {
    /// Returns the title for a given window ID.
    /// Main window has a fixed title; session windows use their dynamic title.
    pub fn title(&self, window: window::Id) -> String {
        if window == self.main_window_id {
            "replayr".into()
        } else {
            self.windows
                .get(&window)
                .map(|window| window.title.clone())
                .unwrap_or_default()
        }
    }
}

/// Renders the UI for a specific window based on its ID.
/// For the main window, shows configuration options; for session windows, delegates to Window::view.
pub fn view_app(state: &App, id: window::Id) -> Element<'_, Message, Theme, iced::Renderer> {
    // Render main
    if id == state.main_window_id {
        container(
            column![
                row![
                    text("Protocol:"),
                    radio(
                        "TCP",
                        crate::types::Protocol::Tcp,
                        Some(state.config.protocol),
                        move |p| Message::ProtocolChanged(id, p)
                    ),
                    radio(
                        "UDP",
                        crate::types::Protocol::Udp,
                        Some(state.config.protocol),
                        move |p| Message::ProtocolChanged(id, p)
                    ),
                ]
                .spacing(10),
                row![
                    text("Address:"),
                    text_input("", &state.config.address)
                        .on_input(move |s| Message::AddressChanged(id, s)),
                    Space::with_width(10),
                    text("Port:"),
                    text_input("", &state.config.port)
                        .on_input(move |s| Message::PortChanged(id, s))
                        .width(Length::Fixed(75f32))
                ]
                .spacing(10),
                if state.config.protocol == crate::types::Protocol::Tcp {
                           row![
                    radio(
                         "Hex",
                         PayloadType::Hex,
                         Some(state.config.initial_payload_type),
                         move |pt| Message::InitialPayloadTypeChanged(id, pt)
                     ),
                     radio(
                         "ASCII",
                         PayloadType::Ascii,
                         Some(state.config.initial_payload_type),
                         move |pt| Message::InitialPayloadTypeChanged(id, pt)
                     ),
                     text_input("optional initial payload...", &state.config.initial_payload)
                         .on_input(move |s| Message::InitialPayloadChanged(id, s)),
                 ]
                 .spacing(10)
                }
                else
                {
                    row![]
                },

                row![
                    tooltip(
                        button(match state.config.protocol {
                            crate::types::Protocol::Tcp => "Connect",
                            crate::types::Protocol::Udp => "Open Session",
                        })
                        .on_press(Message::Connect(id)),
                        match state.config.protocol {
                            crate::types::Protocol::Tcp => "Connect to the TCP server",
                            crate::types::Protocol::Udp => "Open UDP session",
                        },
                        tooltip::Position::Top
                    ),
                    Space::with_width(10),
                    tooltip(
                        button("Replay Connect").on_press(Message::ReplayConnect),
                        "Use a recorded session to connect",
                        tooltip::Position::Top
                    ),
                ],
                container(scrollable(text(&state.main_log)))
                    .style(|_theme| iced::widget::container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.3))),
                        ..Default::default()
                    })
                    .height(Length::Fill)
                    .width(Length::Fill),
            ]
            .spacing(15)
            .padding(20),
        )
        .center_x(Length::Fill)
        .into()
    } else if let Some(window_data) = state.windows.get(&id) {
        // Render session or replay window
        window_data.view(id)
    } else {
        // Fallback for unknown window
        text("Unknown window state").into()
    }
}

/// Updates the application state based on incoming messages.
/// Handles user interactions, system events, and async task results.
pub fn update_app(state: &mut App, message: Message) -> Task<Message> {
    match message {
        // Update address in config and save
        Message::AddressChanged(id, addr) => {
            if id == state.main_window_id {
                state.config.address = addr;
                crate::config::save_config(&state.config);
            }
            Task::none()
        }
        // Update port in config and save
        Message::PortChanged(id, port) => {
            if id == state.main_window_id {
                state.config.port = port;
                crate::config::save_config(&state.config);
            }
            Task::none()
        }
        // Update initial payload in config and save
        Message::InitialPayloadChanged(id, payload) => {
            if id == state.main_window_id {
                state.config.initial_payload = payload;
                crate::config::save_config(&state.config);
            }
            Task::none()
        }
        // Update initial payload type in config and save
        Message::InitialPayloadTypeChanged(id, payload_type) => {
            if id == state.main_window_id {
                state.config.initial_payload_type = payload_type;
                crate::config::save_config(&state.config);
            }
            Task::none()
        }
        // Update protocol in config and save
        Message::ProtocolChanged(id, protocol) => {
            if id == state.main_window_id {
                state.config.protocol = protocol;
                crate::config::save_config(&state.config);
            }
            Task::none()
        }
        // Open a new session window and start connection task
        Message::Connect(id) => {
            if id == state.main_window_id {
                state.last_addr = state.config.address.clone();
                state.last_port = state.config.port.clone();
                state.last_payload = state.config.initial_payload.clone();

                let (new_id, task) = window::open(window::Settings {
                    ..window::Settings::default()
                });

                Task::batch(vec![
                    task.map(move |_| Message::WindowOpened(new_id)),
                    if matches!(state.config.protocol, crate::types::Protocol::Tcp) {
                        Task::perform(async move { true }, move |_| {
                            Message::ConnectionStatus(new_id, true)
                        })
                    } else {
                        Task::none()
                    },
                ])
            } else {
                Task::none()
            }
        }
        // Initialize new session window with data and start TCP task if applicable
        Message::WindowOpened(id) => {
            let (tx, rx) = mpsc::channel(100);
            state.windows.insert(
                id,
                Window {
                    title: match state.config.protocol {
                        crate::types::Protocol::Tcp => "replayr - Tcp session (disconnected)".to_string(),
                        crate::types::Protocol::Udp => "replayr - Udp session".to_string(),
                    },
                    state: Session(SessionData {
                        payload_input: String::new(),
                        log: String::new(),
                        sender: Some(tx),
                        connected: matches!(state.config.protocol, crate::types::Protocol::Udp),
                        payload_type: PayloadType::Hex,
                        protocol: state.config.protocol,
                        replay_payloads: Vec::new(),
                        last_packet_time: None,
                        input_placeholder: "68656c6c6f20776f726c64".into(),
                    }),
                },
            );
            let addr = state.last_addr.clone();
            let port = state.last_port.clone();
            let payload = state.last_payload.clone();
            let payload_type = state.config.initial_payload_type;
            if matches!(state.config.protocol, crate::types::Protocol::Tcp) {
                Task::batch(vec![Task::perform(
                    async move {
                        crate::session::tcp_task(rx, addr, port, payload, payload_type, id).await;
                    },
                    |_| Message::NoOp,
                )])
            } else {
                Task::none()
            }
        }
        // Update payload input in session data
        Message::InputChanged(id, hex) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Session(data) = &mut window_data.state {
                    data.payload_input = hex;
                }
            }
            Task::none()
        }
        // Update payload type and adjust placeholder text
        Message::PayloadTypeChanged(id, payload_type) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Session(data) = &mut window_data.state {
                    data.payload_type = payload_type;
                    data.input_placeholder = match payload_type {
                        PayloadType::Ascii => "Hello World".into(),
                        _ => "68656c6c6f20776f726c64".into(),
                    }
                }
            }
            Task::none()
        }
        // Validate and send packet, record for replay if valid
        Message::SendPacket(id) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Session(data) = &mut window_data.state {
                    let now = std::time::Instant::now();
                    let delay = if let Some(last) = data.last_packet_time {
                        now.duration_since(last).as_millis() as u64
                    } else {
                        0
                    };
                    let hex = data.payload_input.clone();
                    let payload_type = data.payload_type;
                    let protocol = data.protocol;
                    let window_id = id;
                    let addr = state.config.address.clone();
                    let port = state.config.port.clone();
                    let sender = data.sender.clone();
                    // Validate and store payload if valid and non-empty
                    let packet_data = match payload_type {
                        PayloadType::Hex => hex::decode(hex.replace(" ", "")),
                        PayloadType::Ascii => Ok(hex.as_bytes().to_vec()),
                    };
                    match packet_data {
                        Ok(valid_data) => {
                            if !valid_data.is_empty() {
                                data.replay_payloads.push(ReplayablePayload {
                                    payload: hex.clone(),
                                    payload_type,
                                    delay,
                                });
                            }
                            data.last_packet_time = Some(now);
                            Task::perform(
                                async move {
                                    match protocol {
                                        crate::types::Protocol::Tcp => {
                                            if let Some(sender) = sender {
                                                let _ = sender
                                                    .send(SessionCommand::SendPacket(
                                                        valid_data,
                                                        payload_type,
                                                    ))
                                                    .await;
                                            }
                                        }
                                        crate::types::Protocol::Udp => {
                                            crate::udp::send_udp_packet(
                                                valid_data,
                                                addr,
                                                port,
                                                window_id,
                                                payload_type,
                                            )
                                            .await;
                                        }
                                    }
                                },
                                |_| Message::NoOp,
                            )
                        }
                        Err(_) => Task::perform(
                            async move {
                                let _ = LOG_SENDER.lock().await.send(LogMessage {
                                    window_id,
                                    content: "Invalid input".to_string(),
                                });
                            },
                            |_| Message::NoOp,
                        ),
                    }
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }
        // Message::TitleChanged(id, title) => {
        //     if let Some(window) = state.windows.get_mut(&id) {
        //         window.title = title;
        //     }
        //     Task::none()
        // }
        // Exit app if main window closed
        Message::Closed(id) if id == state.main_window_id => exit(),
        // Close session window and disconnect if needed
        Message::Closed(id) => {
            let command = if let Some(window_data) = state.windows.get(&id) {
                match &window_data.state {
                    Session(data) => {
                        if let Some(sender) = &data.sender {
                            let sender = sender.clone();
                            Task::perform(
                                async move {
                                    let _ = sender.send(SessionCommand::Disconnect).await;
                                },
                                |_| Message::NoOp,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                }
            } else {
                Task::none()
            };
            Task::batch(vec![command, window::close(id)])
        }
        // Append log message to the appropriate window's log
        Message::LogReceived(content, id) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                match &mut window_data.state {
                    WindowState::Session(data) => {
                        data.log.push_str(&content);
                        data.log.push('\n');
                    }
                    WindowState::Replay(data) => {
                        data.log.push_str(&content);
                        data.log.push('\n');
                    }
                }
            }
            Task::none()
        }
        // Append log message to main window log
        Message::MainLog(content) => {
            state.main_log.push_str(&content);
            state.main_log.push('\n');
            Task::none()
        }
        // Update connection status and window title
        Message::ConnectionStatus(id, connected) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Session(data) = &mut window_data.state {
                    data.connected = connected;
                    window_data.title = if connected {
                        format!("Tcp session (connected to {})", state.config.address)
                    } else {
                        "Tcp session (disconnected)".to_string()
                    };
                }
            }
            Task::none()
        }
        // Export session payloads to JSON file for replay
        Message::ExportSession(id) => {
            if let Some(window_data) = state.windows.get(&id) {
                if let WindowState::Session(data) = &window_data.state {
                    let replay = crate::types::ReplayableSession {
                        protocol: data.protocol,
                        payloads: data.replay_payloads.clone(),
                    };
                    let title = window_data.title.clone();
                    Task::perform(
                        async move {
                            let json = serde_json::to_string_pretty(&replay).unwrap();
                            let file_path = tokio::task::spawn_blocking(move || {
                                rfd::FileDialog::new()
                                    .set_title("Export Replay")
                                    .add_filter("JSON Files", &["json"])
                                    .set_file_name(format!(
                                        "{}.json",
                                        title
                                            .replace(" ", "_")
                                            .replace("(", "")
                                            .replace(")", "")
                                            .replace("to_", "")
                                    ))
                                    .save_file()
                            })
                            .await
                            .unwrap();
                            if let Some(path) = file_path {
                                if let Err(e) = tokio::fs::write(path, json).await {
                                    eprintln!("Failed to export replay: {}", e);
                                }
                            }
                        },
                        |_| Message::NoOp,
                    )
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }
        // Export session logs to text file
        Message::ExportLogs(id) => {
            if let Some(window_data) = state.windows.get(&id) {
                if let WindowState::Session(data) = &window_data.state {
                    let logs = data.log.clone();
                    let title = window_data.title.clone();
                    Task::perform(
                        async move {
                            let file_path = tokio::task::spawn_blocking(move || {
                                rfd::FileDialog::new()
                                    .set_title("Export Logs")
                                    .add_filter("Text Files", &["txt"])
                                    .set_file_name(format!(
                                        "{}_logs.txt",
                                        title
                                            .replace(" ", "_")
                                            .replace("(", "")
                                            .replace(")", "")
                                            .replace("to_", "")
                                    ))
                                    .save_file()
                            })
                            .await
                            .unwrap();
                            if let Some(path) = file_path {
                                if let Err(e) = tokio::fs::write(path, logs).await {
                                    eprintln!("Failed to export logs: {}", e);
                                }
                            }
                        },
                        |_| Message::NoOp,
                    )
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }
        // Open file dialog to select replay file and load session
        Message::ReplayConnect => {
            let current_protocol = state.config.protocol;
            Task::perform(
                async move {
                    let file_path = tokio::task::spawn_blocking(|| {
                        rfd::FileDialog::new()
                            .set_title("Select Replay File")
                            .add_filter("JSON Files", &["json"])
                            .pick_file()
                    })
                    .await
                    .unwrap();
                    if let Some(path) = file_path {
                        let content = tokio::fs::read_to_string(&path)
                            .await
                            .map_err(|e| format!("Failed to read file: {}", e))?;
                        let replay: ReplayableSession = serde_json::from_str(&content)
                            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
                        if replay.protocol == current_protocol {
                            let file_name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            Ok((replay, file_name))
                        } else {
                            Err(format!(
                                "Replay protocol ({:?}) does not match current protocol ({:?})",
                                replay.protocol, current_protocol
                            ))
                        }
                    } else {
                        Err("No file selected".to_string())
                    }
                },
                move |result| match result {
                    Ok((replay, file_name)) => {
                        Message::ReplayWindowOpenedWithFile(replay, file_name)
                    }
                    Err(err) => Message::ReplayError(err),
                },
            )
        }
        // Open replay window and start replay task
        Message::ReplayWindowOpenedWithFile(replay, file_name) => {
            let (new_id, task) = window::open(window::Settings::default());
            let payloads_clone = replay.payloads.clone();
            let protocol = replay.protocol;
            state.windows.insert(
                new_id,
                Window {
                    title: "Replay session".to_string(),
                    state: Replay(ReplayData {
                        log: String::new(),
                        payloads: replay.payloads,
                        connected: false,
                        file_name: file_name.clone(),
                        current_index: 0,
                    }),
                },
            );
            let addr = state.config.address.clone();
            let port = state.config.port.clone();
            Task::batch(vec![
                task.map(move |_| Message::ReplayStarted(new_id)),
                Task::perform(
                    async move {
                        crate::replay::replay_task(protocol, payloads_clone, addr, port, new_id)
                            .await;
                    },
                    |_| Message::NoOp,
                ),
            ])
        }
        // Mark replay as connected
        Message::ReplayStarted(id) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Replay(data) = &mut window_data.state {
                    data.connected = true;
                }
            }
            Task::none()
        }
        // Update replay progress index
        Message::ReplayProgress(id, current) => {
            if let Some(window_data) = state.windows.get_mut(&id) {
                if let WindowState::Replay(data) = &mut window_data.state {
                    data.current_index = current;
                }
            }
            Task::none()
        }
        // Log replay error to main log
        Message::ReplayError(err) => {
            state.main_log.push_str(&format!(
                "[{}] Error: {}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                err
            ));
            Task::none()
        }

        _ => Task::none(),
    }
}

/// Implementation for Window, providing view rendering based on state.
impl Window {
    /// Renders the UI for session or replay windows.
    pub fn view(&self, id: window::Id) -> iced::Element<'_, crate::ui::Message, iced::Theme> {
        match &self.state {
            // Render session window with input controls and logs
            Session(data) => iced::widget::container(
                iced::widget::column![
                    iced::widget::row![
                        iced::widget::text("Payload type:"),
                        iced::widget::radio(
                            "Hex",
                            PayloadType::Hex,
                            Some(data.payload_type),
                            move |pt| crate::ui::Message::PayloadTypeChanged(id, pt)
                        ),
                        iced::widget::radio(
                            "ASCII",
                            PayloadType::Ascii,
                            Some(data.payload_type),
                            move |pt| crate::ui::Message::PayloadTypeChanged(id, pt)
                        ),
                    ]
                    .spacing(10),
                    iced::widget::row![
                        iced::widget::text_input(&data.input_placeholder, &data.payload_input)
                            .on_input(move |s| crate::ui::Message::InputChanged(id, s)),
                        if data.connected {
                            iced::widget::tooltip(
                                iced::widget::button("Send")
                                    .on_press(crate::ui::Message::SendPacket(id)),
                                "",
                                iced::widget::tooltip::Position::FollowCursor,
                            )
                        } else {
                            iced::widget::tooltip(
                                iced::widget::button("Send"),
                                "Disconnected!",
                                iced::widget::tooltip::Position::FollowCursor,
                            )
                        }
                    ]
                    .spacing(10),
                    iced::widget::container(
                        iced::widget::scrollable(iced::widget::text(&data.log))
                            .height(iced::Length::Fill)
                            .width(iced::Length::Fill)
                    )
                    .style(|_theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.3, 0.3, 0.3
                        ))),
                        ..Default::default()
                    })
                    .height(iced::Length::Fill)
                    .width(iced::Length::Fill),
                    iced::widget::row![
                        iced::widget::tooltip(
                            iced::widget::button("Export Replay")
                                .on_press(crate::ui::Message::ExportSession(id)),
                            "Save for replay",
                            iced::widget::tooltip::Position::Top
                        ),
                        iced::widget::Space::with_width(10),
                        iced::widget::tooltip(
                            iced::widget::button("Export Logs")
                                .on_press(crate::ui::Message::ExportLogs(id)),
                            "Save all logs to file",
                            iced::widget::tooltip::Position::Top
                        ),
                        iced::widget::Space::with_width(iced::Length::Fill),
                        iced::widget::button("Close").on_press(crate::ui::Message::Closed(id))
                    ]
                ]
                .spacing(15)
                .padding(20),
            )
            .center_x(iced::Length::Fill)
            .into(),
            // Render replay window with progress and logs
            WindowState::Replay(data) => iced::widget::container(
                iced::widget::column![
                    iced::widget::text(format!("Replaying session from: {}", data.file_name)),
                    iced::widget::text(format!(
                        "Progress: {}/{}",
                        data.current_index,
                        data.payloads.len()
                    )),
                    iced::widget::container(
                        iced::widget::scrollable(iced::widget::text(&data.log))
                            .height(iced::Length::Fill)
                            .width(iced::Length::Fill)
                    )
                    .style(|_theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.3, 0.3, 0.3
                        ))),
                        ..Default::default()
                    })
                    .height(iced::Length::Fill)
                    .width(iced::Length::Fill),
                    iced::widget::row![
                        iced::widget::button("Close").on_press(crate::ui::Message::Closed(id))
                    ]
                ]
                .spacing(15)
                .padding(20),
            )
            .center_x(iced::Length::Fill)
            .into(),
        }
    }
}
