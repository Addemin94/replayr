use crate::log::{log, LogLevel, PROGRESS_SENDER};
use crate::types::{Protocol, ReplayablePayload};
use hex;
use iced::window;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

/// Replays a sequence of payloads over TCP or UDP, with delays between each packet.
/// Logs progress and handles connection setup based on the protocol.
pub async fn replay_task(
    protocol: Protocol,
    payloads: Vec<ReplayablePayload>,
    addr: String,
    port: String,
    window_id: window::Id,
) {
    // Prepare target address
    let addr_full = format!("{}:{}", addr, port);
    match protocol {
        Protocol::Tcp => {
            // Establish TCP connection for replay
            let stream = match TcpStream::connect(&addr_full).await {
                Ok(s) => s,
                Err(e) => {
                    log(
                        LogLevel::Error,
                        window_id,
                        &format!("Replay connection failed: {}", e),
                    )
                    .await;
                    return;
                }
            };
            log(
                LogLevel::Info,
                window_id,
                &format!("Replay connected to {}", addr_full),
            )
            .await;
            let (mut reader, mut writer) = stream.into_split();
            let disconnect_flag = Arc::new(AtomicBool::new(false));
            let send_fut = {
                let disconnect_flag = Arc::clone(&disconnect_flag);
                async move {
                    // Replay each payload with delay
                    for (i, payload) in payloads.iter().enumerate() {
                        if disconnect_flag.load(Ordering::Relaxed) {
                            log(LogLevel::Info, window_id, "Replay stopped due to disconnect").await;
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(payload.delay)).await;
                        if disconnect_flag.load(Ordering::Relaxed) {
                            log(LogLevel::Info, window_id, "Replay stopped due to disconnect").await;
                            break;
                        }
                        let data = payload.get_payload();
                        match data {
                            Ok(data) => {
                                if data.is_empty() {
                                    continue; // Skip empty payloads
                                }
                                if let Err(e) = writer.write_all(&data).await {
                                    log(
                                        LogLevel::Error,
                                        window_id,
                                        &format!("Replay send failed: {}", e),
                                    )
                                    .await;
                                    break;
                                }
                                let _ = PROGRESS_SENDER.lock().await.send((window_id, i + 1));
                                log(
                                    LogLevel::Info,
                                    window_id,
                                    &format!("Sent: {}", payload.payload),
                                )
                                .await;
                            }
                            Err(_) => {
                                log(LogLevel::Warn, window_id, "Replay invalid payload").await;
                            }
                        }
                    }
                }
            };
            let read_fut = {
                let disconnect_flag = Arc::clone(&disconnect_flag);
                async move {
                    let mut buf = [0; 1024];
                    loop {
                        match reader.read(&mut buf).await {
                            Ok(0) => {
                                disconnect_flag.store(true, Ordering::Relaxed);
                                log(LogLevel::Info, window_id, "Connection closed by server").await;
                                break; // Connection closed
                            }
                            Ok(n) => {
                                log(
                                    LogLevel::Info,
                                    window_id,
                                    &format!("Received: {}", hex::encode(&buf[..n])),
                                )
                                .await;
                            }
                            Err(e) => {
                                disconnect_flag.store(true, Ordering::Relaxed);
                                log(
                                    LogLevel::Error,
                                    window_id,
                                    &format!("Replay read error: {}", e),
                                )
                                .await;
                                break;
                            }
                        }
                    }
                }
            };
            tokio::join!(send_fut, read_fut);
        }
        Protocol::Udp => {
            // Bind UDP socket for replay (no connection needed)
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(e) => {
                    log(
                        LogLevel::Error,
                        window_id,
                        &format!("Replay UDP bind failed: {}", e),
                    )
                    .await;
                    return;
                }
            };
            log(
                LogLevel::Info,
                window_id,
                &format!("Replay UDP ready to {}", addr_full),
            )
            .await;
            // Replay each payload with delay
            for (i, payload) in payloads.iter().enumerate() {
                tokio::time::sleep(tokio::time::Duration::from_millis(payload.delay)).await;
                let data = payload.get_payload();
                match data {
                    Ok(data) => {
                        if data.is_empty() {
                            continue; // Skip empty payloads
                        }
                        if let Err(e) = socket.send_to(&data, &addr_full).await {
                            log(
                                LogLevel::Error,
                                window_id,
                                &format!("Replay send failed: {}", e),
                            )
                            .await;
                            break;
                        }
                        let _ = PROGRESS_SENDER.lock().await.send((window_id, i + 1));
                        log(
                            LogLevel::Info,
                            window_id,
                            &format!("Sent: {}", payload.payload),
                        )
                        .await;
                    }
                    Err(_) => {
                        log(LogLevel::Warn, window_id, "Replay invalid payload").await;
                    }
                }
            }
        }
    }
    // Log replay completion
    log(LogLevel::Info, window_id, "Replay finished").await;
}
