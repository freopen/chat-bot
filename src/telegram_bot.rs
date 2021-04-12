use anyhow::{Context, Result};
use futures::StreamExt;
use log::error;
use teloxide::{
    net::Download,
    prelude::*,
    requests::RequesterExt,
    types::{ChatAction, InputFile},
    utils::command::BotCommand,
    Bot,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::enhance;

#[derive(BotCommand, Debug)]
#[command(rename = "lowercase", parse_with = "split")]
enum FreopenBotCommand {
    Sirify,
    Ayrify,
    Foxify,
    Ukulelify,
}

async fn process_message(context: UpdateWithCx<AutoSend<Bot>, Message>) -> Result<()> {
    let message = &context.update;
    let bot = &context.requester;
    if let Ok(command) = FreopenBotCommand::parse(
        message.text().unwrap_or(""),
        std::env::var("TELEGRAM_BOTNAME")?,
    ) {
        let photo = message
            .photo()
            .or_else(|| message.reply_to_message()?.photo());
        if let Some(photo) = photo {
            bot.send_chat_action(context.chat_id(), ChatAction::UploadPhoto)
                .await?;
            let photo = photo.last().context("Empty PhotoSize")?;
            let photo = {
                let mut buf = vec![];
                bot.download_file(
                    bot.get_file(&photo.file_id).await?.file_path.as_str(),
                    &mut buf,
                )
                .await?;
                buf
            };
            let filename = match command {
                FreopenBotCommand::Sirify => "sirify",
                FreopenBotCommand::Ayrify => "ayrify",
                FreopenBotCommand::Foxify => "foxify",
                FreopenBotCommand::Ukulelify => "ukulelify",
            };
            let output_photo = enhance::overlay_image(filename, photo)?;
            context
                .answer_photo(InputFile::memory("image.jpg", output_photo))
                .reply_to_message_id(message.id)
                .await?;
        }
    }
    Ok(())
}

async fn messages_handler(rx: DispatcherHandlerRx<AutoSend<Bot>, Message>) {
    UnboundedReceiverStream::new(rx)
        .for_each_concurrent(None, |message| async move {
            if let Err(err) = process_message(message).await {
                error!("{:?}", err);
            }
        })
        .await;
}

pub async fn listen() -> Result<()> {
    let bot = Bot::new(std::env::var("TELEGRAM_TOKEN")?).auto_send();
    Dispatcher::new(bot)
        .messages_handler(messages_handler)
        .dispatch()
        .await;
    Ok(())
}
