use anyhow::Result;
use log::{error, info, warn};
use serde_json::{json, Value};
use tokio::sync::watch;

use crate::enhance;
use crate::telegram_client::TelegramClient;

async fn process_update(update: Value, telegram_client: TelegramClient) -> Result<()> {
    info!("Processing update: {}", update);
    if let Some(message) = update.get("message") {
        let chat_id = message["chat"]["id"].as_i64().unwrap();
        let reply_to = message["message_id"].as_i64().unwrap();
        if let Some(Value::Array(ref sizes)) = message.get("photo").or_else(|| {
            message
                .get("reply_to_message")
                .and_then(|origin| origin.get("photo"))
        }) {
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
            let output_file = enhance::overlay_image(input_file.to_vec())?;
            telegram_client
                .send_photo(output_file, chat_id, reply_to)
                .await?;
        } else {
            warn!("Photo was not found");
        }
    }
    Ok(())
}

pub async fn listen(ctrl_c: watch::Receiver<bool>) -> Result<()> {
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
