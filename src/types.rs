use iced::window;
use serde::{Deserialize, Serialize};

/// Represents a log message associated with a specific window.
#[derive(Clone)]
pub struct LogMessage {
    pub window_id: window::Id,
    pub content: String,
}

/// Commands that can be sent to a session task to control its behavior.
#[derive(Clone)]
pub enum SessionCommand {
    SendPacket(Vec<u8>, PayloadType),
    Disconnect,
}

/// Represents the state of a window, either a live session or a replay session.
#[derive(Clone)]
pub enum WindowState {
    Session(SessionData),
    Replay(ReplayData),
}

/// Represents a window with a title and its current state.
#[derive(Clone)]
pub struct Window {
    pub title: String,
    pub state: WindowState,
}

/// Specifies the type of payload data: hexadecimal or ASCII text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PayloadType {
    Hex,
    Ascii,
}

/// Specifies the network protocol to use: TCP or UDP.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

/// Represents a complete session that can be replayed, including protocol and list of payloads.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplayableSession {
    pub protocol: Protocol,
    pub payloads: Vec<ReplayablePayload>,
}

/// Represents a single payload in a replay session, with its data, type, and delay from previous payload.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplayablePayload {
    pub payload: String,
    pub payload_type: PayloadType,
    pub delay: u64,
}

impl ReplayablePayload {
    pub fn get_payload(&self) -> Result<Vec<u8>, hex::FromHexError> {
        match self.payload_type {
                PayloadType::Hex => hex::decode(&self.payload.replace(" ", "")),
                PayloadType::Ascii => Ok(self.payload.as_bytes().to_vec())
        }
    }
}

/// Holds the state of an active session window, including user input, logs, connection status, and replay data.
#[derive(Clone)]
pub struct SessionData {
    pub payload_input: String,
    pub log: String,
    pub sender: Option<tokio::sync::mpsc::Sender<SessionCommand>>,
    pub connected: bool,
    pub payload_type: PayloadType,
    pub protocol: Protocol,
    pub replay_payloads: Vec<ReplayablePayload>,
    pub last_packet_time: Option<std::time::Instant>,
    pub input_placeholder: String,
    pub initial_payload: String,
    pub initial_payload_type: PayloadType,
}

/// Holds the state of a replay session window, including logs, payloads, and progress tracking.
#[derive(Clone)]
pub struct ReplayData {
    pub log: String,
    pub payloads: Vec<ReplayablePayload>,
    pub connected: bool,
    pub file_name: String,
    pub current_index: usize,
}
