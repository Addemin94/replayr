use crate::types::LogMessage;
use iced::window;
use lazy_static::lazy_static;
use tokio::sync::broadcast;

use chrono::Local;

/// Represents the severity level of a log message.
#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERR"),
        }
    }
}

/// Formats a log message with timestamp and level prefix.
pub fn format_log(level: LogLevel, msg: &str) -> String {
    format!(
        "[{}] [{}] {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        msg
    )
}

/// Convenience function to send a log message asynchronously.
pub async fn log(level: LogLevel, window_id: window::Id, msg: &str) {
    let _ = LOG_SENDER.lock().await.send(LogMessage {
        window_id,
        content: format_log(level, msg),
    });
}

/// Convenience function to send a main log message asynchronously.
pub async fn main_log(msg: String) {
    let _ = MAIN_LOG_SENDER.lock().await.send(msg);
}

// Global broadcast channels for logging and communication between tasks
lazy_static! {
    /// Sends log messages to specific windows
    pub static ref LOG_SENDER: std::sync::Arc<tokio::sync::Mutex<broadcast::Sender<LogMessage>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(broadcast::Sender::new(100)));
    /// Sends log messages to the main window
    pub static ref MAIN_LOG_SENDER: std::sync::Arc<tokio::sync::Mutex<broadcast::Sender<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(broadcast::Sender::new(100)));
    /// Sends connection status updates
    pub static ref CONNECTION_SENDER: std::sync::Arc<tokio::sync::Mutex<broadcast::Sender<(window::Id, bool)>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(broadcast::Sender::new(100)));
    /// Sends replay progress updates
    pub static ref PROGRESS_SENDER: std::sync::Arc<tokio::sync::Mutex<broadcast::Sender<(iced::window::Id, usize)>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(broadcast::Sender::new(100)));
}
