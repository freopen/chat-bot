use anyhow::{Context, Result};
use log::{error, info, warn};
use serde_json::{json, Value};
use tokio::sync::watch;

use crate::telegram_client::TelegramClient;
use crate::{
    enhance,
    telegram_client::{Message, Update, UpdateEnum},
};

async fn process_update(update: Update, telegram_client: TelegramClient) -> Result<()> {
    info!("Processing update: {:#?}", update);
    if let UpdateEnum::Message(message) = update.update_enum {
        match &message.text {
            Some(text) if text.starts_with("/enhance") => {}
            _ => return Ok(()),
        }
        let message_with_photo = match message {
            Message { photo: Some(_), .. } => Some(&message),
            Message {
                reply_to_message: Some(ref reply_to_message),
                ..
            } if reply_to_message.photo.is_some() => Some(reply_to_message.as_ref()),
            _ => None,
        };

        if let Some(message_with_photo) = message_with_photo {
            let chat_id = message.chat.id;
            let client_clone = telegram_client.clone();
            tokio::spawn(async move {
                client_clone
                    .call_method::<Value, Value>(
                        "sendChatAction",
                        json!({
                            "chat_id": chat_id,
                            "action": "upload_photo",
                        }),
                    )
                    .await
            });
            let file_id = &message_with_photo
                .photo
                .as_ref()
                .unwrap()
                .last()
                .context("Photo field has no photo sizes")?
                .file_id;
            let input_file = telegram_client.get_file(file_id.into()).await?;
            let output_file = enhance::overlay_image(input_file.to_vec())?;
            telegram_client
                .send_photo(output_file, message.chat.id, message.message_id)
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
                error!("{:#?}", error);
                continue;
            }
            let (new_offset, updates) = updates?;
            offset = new_offset;
            updates
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
