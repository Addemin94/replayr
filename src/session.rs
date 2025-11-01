use crate::log::{format_log, log, main_log, LogLevel, CONNECTION_SENDER};
use crate::types::{PayloadType, SessionCommand};
use hex::decode;
use iced::window;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Handles the TCP session asynchronously, managing connection, sending/receiving data, and logging.
/// Runs in a separate task and communicates via channels for commands and logs.
pub async fn tcp_task(
    mut rx: tokio::sync::mpsc::Receiver<SessionCommand>,
    addr: String,
    port: String,
    initial_payload: String,
    initial_payload_type: PayloadType,
    window_id: window::Id,
) {
    // Attempt to connect to the TCP server
    let addr_full = format!("{}:{}", addr, port);
    let stream = match TcpStream::connect(&addr_full).await {
        Ok(s) => s,
        Err(e) => {
            // Log connection failure and exit
            log(
                LogLevel::Error,
                window_id,
                &format!("Connection failed: {}", e),
            )
            .await;
            main_log(format_log(
                LogLevel::Error,
                &format!("Session failed: {}", addr_full),
            ))
            .await;
            return;
        }
    };
    // Log successful connection
    log(
        LogLevel::Info,
        window_id,
        &format!("Connected to {}", addr_full),
    )
    .await;
    main_log(format_log(
        LogLevel::Info,
        &format!("Session started: {}", addr_full),
    ))
    .await;
    let _ = CONNECTION_SENDER.lock().await.send((window_id, true));

    // Split the stream into reader and writer
    let (mut _reader, mut writer) = stream.into_split();

    // Send initial payload if provided
    if !initial_payload.is_empty() {
        let data = match initial_payload_type {
            PayloadType::Hex => decode(&initial_payload.replace(" ", "")),
            PayloadType::Ascii => Ok(initial_payload.as_bytes().to_vec()),
        };
        match data {
            Ok(data) => {
                if let Err(e) = writer.write_all(&data).await {
                    log(
                        LogLevel::Error,
                        window_id,
                        &format!("Failed to send initial payload: {}", e),
                    )
                    .await;
                } else {
                    log(
                        LogLevel::Info,
                        window_id,
                        &format!("Sent initial payload: {}", match initial_payload_type {
                            PayloadType::Hex => hex::encode(&data),
                            PayloadType::Ascii => String::from_utf8_lossy(&data).to_string(),
                        }),
                    )
                    .await;
                }
            }
            Err(_) => {
                log(LogLevel::Warn, window_id, &format!("Invalid initial payload ({:?})", initial_payload_type)).await;
            }
        }
    }
    // Main event loop: handle commands and incoming data
    let mut buf = [0; 1024];
    loop {
        tokio::select! {
            // Handle incoming commands from the UI
            cmd = rx.recv() => {
                match cmd {
                    Some(SessionCommand::SendPacket(data, payload_type)) => {
                        if let Err(e) = writer.write_all(&data).await {
                            log(LogLevel::Error, window_id, &format!("Send failed: {}", e)).await;
                            break;
                        }
                        log(LogLevel::Info, window_id, &format!("Sent: {}",
                            match payload_type {
                                PayloadType::Hex => hex::encode(&data),
                                PayloadType::Ascii => String::from_utf8_lossy(&data).to_string(),
                            })).await;
                    }
                    Some(SessionCommand::Disconnect) => {
                        main_log(format_log(LogLevel::Info, "Disconnect received")).await;
                        break;
                    }
                    None => break,
                }
            }
            // Handle incoming data from the server
            n = _reader.read(&mut buf) => {
                match n {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        let data = &buf[..n];
                        log(LogLevel::Info, window_id, &format!("Received: {}", hex::encode(data))).await;
                    }
                    Err(e) => {
                        log(LogLevel::Error, window_id, &format!("Read error: {}", e)).await;
                        break;
                    }
                }
            }
        }
    }
    // Notify disconnection and session end
    let _ = CONNECTION_SENDER.lock().await.send((window_id, false));
    main_log(format_log(
        LogLevel::Info,
        &format!("Session ended: {}", addr_full),
    ))
    .await;
}
