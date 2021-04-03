mod enhance;
mod telegram_client;

use anyhow::Result;
use lazy_static::lazy_static;
use log::{error, info, warn};
use regex::Regex;
use serde_json::{json, Value};
use telegram_client::TelegramClient;
use std::io::Write;
use tokio::sync::watch;


async fn process_update(update: Value, telegram_client: TelegramClient) -> Result<()> {
    info!("Processing update: {}", update);
    if let Some(message) = update.get("message") {
        let chat_id = message["chat"]["id"].as_i64().unwrap();
        let reply_to = message["message_id"].as_i64().unwrap();
        if let Some(Value::Array(ref sizes)) = message.get("photo").or(message
            .get("reply_to_message")
            .and_then(|origin| origin.get("photo")))
        {
            let client_clone = telegram_client.clone();
            tokio::spawn(async move {
                client_clone
                    .call_method(
                        "sendChatAction",
                        json!({
                            "chat_id": chat_id,
                            "action": "upload_photo",
                        }),
                    )
                    .await
            });
            let file_id = sizes.last().unwrap().as_object().unwrap()["file_id"]
                .as_str()
                .unwrap();
            let input_file = telegram_client.get_file(file_id.into()).await?;
            let output_file = enhance::overlay_image(input_file)?;
            telegram_client
                .send_photo(output_file, chat_id, reply_to)
                .await?;
        } else {
            warn!("Photo was not found");
        }
    }
    Ok(())
}

async fn listen(ctrl_c: watch::Receiver<bool>) -> Result<()> {
    let mut ctrl_c = ctrl_c;
    let telegram_client = TelegramClient::new();
    let mut offset = 0;
    loop {
        let updates = {
            let updates = telegram_client.get_updates(offset);
            let updates = tokio::select! {
                _ = ctrl_c.changed() => {
                    info!("Shutdown signal arrived to Telegram listener, flushing current offset.");
                    telegram_client.flush_offset(offset).await?;
                    info!("Offset flushed, exiting.");
                    return Ok(());
                }
                updates = updates => updates,
            };

            if let Err(error) = updates {
                error!("{}", error);
                continue;
            }
            if let (new_offset, Value::Array(array)) = updates.unwrap() {
                offset = new_offset;
                array
            } else {
                panic!()
            }
        };
        let joins: Vec<_> = updates
            .into_iter()
            .map(|update| tokio::spawn(process_update(update, telegram_client.clone())))
            .collect();
        for join in joins {
            if let Err(error) = join.await.unwrap() {
                error!("{}", error);
            }
        }
    }
}

fn format_path(
    path: &str,
    line: u32,
    buf: &mut env_logger::fmt::Formatter,
) -> std::result::Result<(), std::io::Error> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^src/(.*)$|^/.*/.cargo/registry/src/[^/]+/([^/]+)/").unwrap();
    }
    let cap = RE.captures(path);
    if let Some(cap) = cap {
        if let Some(source_file) = cap.get(1) {
            write!(buf, "{}:{}", source_file.as_str(), line)
        } else if let Some(external_lib) = cap.get(2) {
            write!(buf, "<{}>", external_lib.as_str())
        } else {
            write!(buf, "{}:{}", path, line)
        }
    } else {
        write!(buf, "{}:{}", path, line)
    }
}

async fn await_signal(signal_kind: tokio::signal::unix::SignalKind) -> Result<()> {
    tokio::signal::unix::signal(signal_kind)?.recv().await;
    Ok(())
}

#[tokio::main]
async fn main() {
    let (ctrl_c_sender, ctrl_c) = watch::channel(false);
    tokio::spawn(async move {
        tokio::select! {
            _ = await_signal(tokio::signal::unix::SignalKind::interrupt()) => {info!("SIGINT received");},
            _ = await_signal(tokio::signal::unix::SignalKind::terminate()) => {info!("SIGTERM received");},
        };
        ctrl_c_sender.send(true).unwrap();
    });
    dotenv::dotenv().ok();
    env_logger::Builder::new()
        .format(|buf, record| {
            write!(
                buf,
                "{} {} ",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                buf.default_styled_level(record.level()),
            )?;
            format_path(
                record.file().unwrap_or("<unknown>"),
                record.line().unwrap_or(0),
                buf,
            )?;
            writeln!(buf, " {}", record.args())
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    info!("Listening for telegram updates...");
    tokio::spawn(listen(ctrl_c)).await.unwrap().unwrap();
}
