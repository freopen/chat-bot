use anyhow::{Context, Result};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{error, info};
use teloxide::{
    net::Download,
    prelude::*,
    requests::RequesterExt,
    types::{ChatAction, ChatKind, InputFile},
    utils::command::BotCommand,
    Bot,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    db::{Subscription, DB},
    enhance,
};

#[derive(BotCommand, Debug)]
#[command(rename = "lowercase")]
enum FreopenBotCommand {
    Sirify(String),
    Ayrify(String),
    Foxify(String),
    Ukulelify(String),
    Subscribe(String),
}

fn debug_format_message(message: &Message) -> String {
    lazy_static! {
        static ref UNKNOWN: String = String::from("unknown");
        static ref PRIVATE: String = String::from("private");
    }
    let user = message
        .from()
        .and_then(|user| user.username.as_ref())
        .unwrap_or(&UNKNOWN);
    let chat = match &message.chat.kind {
        ChatKind::Public(public) => public.title.as_ref().unwrap_or(&UNKNOWN),
        ChatKind::Private(_) => &PRIVATE,
    };
    format!("user: {}, chat: {}", user, chat)
}

async fn subscribe_command(
    params: String,
    context: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<()> {
    match params.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["add", url] => {
            DB.write(|db| {
                let subs = db
                    .subscribe
                    .chat_to_sub
                    .entry(context.chat_id())
                    .or_default();
                subs.push(Subscription::RSS {
                    url: url.to_string(),
                    last_entry: String::new(),
                });
            })?;
            DB.save()?;
            context.answer("OK").await?;
        }
        ["list"] => {
            let subs: Vec<String> = DB.read(|db| {
                db.subscribe
                    .chat_to_sub
                    .get(&context.chat_id())
                    .map(|subs| {
                        subs.iter()
                            .map(|sub| match sub {
                                Subscription::RSS { url, .. } => url.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or(vec![])
            })?;

            context
                .answer(format!("List of your subs: \n{}", subs.join("\n"),))
                .await?;
        }
        ["remove", url] => {
            DB.write(|db| {
                let subs = db
                    .subscribe
                    .chat_to_sub
                    .entry(context.chat_id())
                    .or_default();
                subs.retain(|sub| match sub {
                    Subscription::RSS { url: sub_url, .. } => url != sub_url,
                });
            })?;
            DB.save()?;
            context.answer("OK").await?;
        }
        _ => {
            context
                .answer("Usage: add <url> | list | remove <url>")
                .await?;
        }
    }
    Ok(())
}

async fn process_message(context: &UpdateWithCx<AutoSend<Bot>, Message>) -> Result<()> {
    let message = &context.update;
    let bot = &context.requester;
    if let Ok(command) = FreopenBotCommand::parse(
        message.text().unwrap_or(""),
        std::env::var("TELEGRAM_BOTNAME")?,
    ) {
        if let FreopenBotCommand::Subscribe(params) = command {
            return subscribe_command(params, context).await;
        }
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
            let (filename, param) = match command {
                FreopenBotCommand::Sirify(param) => ("sirify", param),
                FreopenBotCommand::Ayrify(param) => ("ayrify", param),
                FreopenBotCommand::Foxify(param) => ("foxify", param),
                FreopenBotCommand::Ukulelify(param) => ("ukulelify", param),
                _ => return Ok(()),
            };
            let mirror = param == "mirror";
            info!(
                "Enhancing photo with template {}(m:{}), {}",
                filename,
                mirror,
                debug_format_message(message)
            );
            let output_photo = enhance::overlay_image(filename, photo, mirror)?;
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
            if let Err(err) = process_message(&message).await {
                message.answer(format!("Error: {:#?}", err));
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
