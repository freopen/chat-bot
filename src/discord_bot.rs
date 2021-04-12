use std::borrow::Cow;

use anyhow::Result;
use log::{error, info};
use serenity::prelude::*;
use serenity::Client;
use serenity::{http::AttachmentType, model::prelude::*};

use crate::enhance;

struct Handler;

async fn process_image(attachment: Attachment) -> Result<Vec<u8>> {
    enhance::overlay_image("foxify", attachment.download().await?, false)
}

async fn handle_reaction(ctx: &Context, reaction: &Reaction) -> Result<()> {
    if reaction.emoji == ReactionType::Unicode(String::from("💯")) {
        let msg = reaction
            .channel_id
            .message(ctx, reaction.message_id)
            .await
            .unwrap();
        info!("{:?}", msg);
        if msg.attachments.is_empty() {
            return Ok(());
        }
        let tasks: Vec<_> = msg
            .attachments
            .iter()
            .map(|attachment| tokio::spawn(process_image(attachment.clone())))
            .collect();
        let mut output_images = Vec::new();
        for task in tasks {
            output_images.push(task.await??)
        }
        reaction
            .channel_id
            .send_files(
                ctx,
                output_images
                    .into_iter()
                    .map(|image| AttachmentType::Bytes {
                        data: Cow::from(image),
                        filename: String::from("image.jpg"),
                    }),
                |m| m,
            )
            .await?;
    }
    Ok(())
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        info!("New reaction");
        if let Err(error) = handle_reaction(&ctx, &reaction).await {
            error!("Error while processing reaction: {:?}", error);
        }
    }
}

pub async fn listen() -> Result<()> {
    let mut client = Client::builder(std::env::var("DISCORD_TOKEN")?)
        .event_handler(Handler)
        .await?;
    client.start().await?;
    Ok(())
}
