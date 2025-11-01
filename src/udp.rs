use crate::log::{log, LogLevel};
use crate::types::PayloadType;
use iced::window;
use tokio::net::UdpSocket;

/// Sends a UDP packet to the specified address and port, logging the result.
/// Since UDP is connectionless, this function binds to a local socket and sends the data.
pub async fn send_udp_packet(
    data: Vec<u8>,
    addr: String,
    port: String,
    window_id: window::Id,
    payload_type: PayloadType,
) {
    // Prepare the target address and bind a local UDP socket
    let addr_full = format!("{}:{}", addr, port);
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            log(
                crate::log::LogLevel::Error,
                window_id,
                &format!("UDP bind failed: {}", e),
            )
            .await;
            return;
        }
    };
    // Attempt to send the packet and log the result
    if let Err(e) = socket.send_to(&data, &addr_full).await {
        log(
            LogLevel::Error,
            window_id,
            &format!("UDP send failed: {}", e),
        )
        .await;
    } else {
        log(
            LogLevel::Info,
            window_id,
            &format!(
                "Sent: {}",
                match payload_type {
                    PayloadType::Hex => hex::encode(&data),
                    PayloadType::Ascii => String::from_utf8_lossy(&data).to_string(),
                }
            ),
        )
        .await;
    }
}
