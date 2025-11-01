mod config;
mod log;
mod replay;
mod session;
mod types;
mod udp;
mod ui;

use config::load_config;
use iced::Subscription;
use iced::window::Icon;
use png;
use log::{format_log, LogLevel, CONNECTION_SENDER, LOG_SENDER, MAIN_LOG_SENDER, PROGRESS_SENDER};
use ui::{App, Message};
fn get_app_icon() -> Icon {
    let icon_bytes = include_bytes!("../assets/icon.png");

    let decoder = png::Decoder::new(std::io::Cursor::new(&icon_bytes[..]));
    let mut reader = decoder.read_info().expect("Failed to read PNG");
    let width = reader.info().width;
    let height = reader.info().height;
    let mut buf = vec![0; (width * height * 4) as usize];
    reader.next_frame(&mut buf).expect("Failed to decode PNG");

    iced::window::icon::from_rgba(buf, width, height).expect("Invalid icon format")
}

fn main() -> iced::Result {
    println!(
        "{}",
        format_log(LogLevel::Info, "Starting replayr...")
    );
    let config = load_config();
    println!(
        "{}",
        format_log(LogLevel::Info, &format!("Config loaded: {:?}", config))
    );
    iced::daemon(App::title, ui::update_app, ui::view_app)
        .subscription(|_state: &App| {
            Subscription::batch(vec![
                iced::window::close_events().map(Message::Closed),
                Subscription::run_with_id(
                    "log",
                    iced::futures::stream::unfold(
                        None::<tokio::sync::broadcast::Receiver<crate::types::LogMessage>>,
                        |state| async move {
                            let mut receiver = match state {
                                Some(r) => r,
                                None => LOG_SENDER.lock().await.subscribe(),
                            };
                            match receiver.recv().await {
                                Ok(msg) => Some((
                                    Message::LogReceived(msg.content, msg.window_id),
                                    Some(receiver),
                                )),
                                Err(_) => None,
                            }
                        },
                    ),
                ),
                Subscription::run_with_id(
                    "main_log",
                    iced::futures::stream::unfold(
                        None::<tokio::sync::broadcast::Receiver<String>>,
                        |state| async move {
                            let mut receiver = match state {
                                Some(r) => r,
                                None => MAIN_LOG_SENDER.lock().await.subscribe(),
                            };
                            match receiver.recv().await {
                                Ok(content) => Some((Message::MainLog(content), Some(receiver))),
                                Err(_) => None,
                            }
                        },
                    ),
                ),
                Subscription::run_with_id(
                    "connection",
                    iced::futures::stream::unfold(
                        None::<tokio::sync::broadcast::Receiver<(iced::window::Id, bool)>>,
                        |state| async move {
                            let mut receiver = match state {
                                Some(r) => r,
                                None => CONNECTION_SENDER.lock().await.subscribe(),
                            };
                            match receiver.recv().await {
                                Ok((id, connected)) => {
                                    Some((Message::ConnectionStatus(id, connected), Some(receiver)))
                                }
                                Err(_) => None,
                            }
                        },
                    ),
                ),
                Subscription::run_with_id(
                    "progress",
                    iced::futures::stream::unfold(
                        None::<tokio::sync::broadcast::Receiver<(iced::window::Id, usize)>>,
                        |state| async move {
                            let mut receiver = match state {
                                Some(r) => r,
                                None => PROGRESS_SENDER.lock().await.subscribe(),
                            };
                            match receiver.recv().await {
                                Ok((id, current)) => {
                                    Some((Message::ReplayProgress(id, current), Some(receiver)))
                                }
                                Err(_) => None,
                            }
                        },
                    ),
                ),
            ])
        })
        .run_with(|| {
            let mut app = App {
                config,
                ..Default::default()
            };
            let (main_window_id, task) = iced::window::open(iced::window::Settings {
                icon: Some(get_app_icon()),
                ..Default::default()
            });
            app.main_window_id = main_window_id;
            (app, task.map(|_| Message::NoOp))
        })
}
