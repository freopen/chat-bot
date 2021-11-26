use std::time::Duration;

use anyhow::{Context, Result};
use feed_rs::{model::Feed, parser::parse_with_uri};
use log::{error, info};
use teloxide::prelude::*;
use tokio::{spawn, time::sleep};

use crate::db::*;

async fn get_feed(url: &str) -> Result<Feed> {
  let response = reqwest::get(url).await?;
  Ok(parse_with_uri(response.bytes().await?.as_ref(), Some(url))?)
}

pub async fn subscribe_command(
  params: String,
  context: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<()> {
  info!(
    "sub command from {}: {}",
    context
      .update
      .from()
      .context("Unknown user")?
      .username
      .as_ref()
      .unwrap_or(&"UNKNOWN".to_string()),
    params
  );
  match params.split_whitespace().collect::<Vec<_>>().as_slice() {
    ["add", url] => {
      get_feed(url).await.context("Unable to access this feed")?;
      DB.write(|db| {
        let subs = db
          .subscribe
          .chat_to_sub
          .entry(context.chat_id())
          .or_default();
        subs.insert(
          url.to_string(),
          SubInfo::Rss {
            last_entry: String::new(),
          },
        );
      })?;
      DB.save()?;
      context.answer("OK").await?;
    }
    ["list"] => {
      let subs: Vec<String> = DB.read(|db| {
        db.subscribe
          .chat_to_sub
          .get(&context.chat_id())
          .map(|subs| subs.keys().cloned().collect())
          .unwrap_or_default()
      })?;

      context
        .answer(format!("List of your subs: \n{}", subs.join("\n"),))
        .await?;
    }
    ["remove", url] => {
      DB.write(|db| {
        db.subscribe
          .chat_to_sub
          .entry(context.chat_id())
          .and_modify(|subs| {
            subs.remove(&url.to_string());
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

async fn process_sub(
  bot: AutoSend<Bot>,
  chat_id: i64,
  sub: String,
  sub_info: SubInfo,
) -> Result<()> {
  let feed = get_feed(&sub).await?;
  let SubInfo::Rss { last_entry } = sub_info;
  let to_send: Vec<_> = feed
    .entries
    .into_iter()
    .take_while(|entry| entry.id != last_entry)
    .collect();
  let next_last_entry = match to_send.first() {
    Some(entry) => entry.id.clone(),
    None => return Ok(()),
  };
  for entry in to_send.into_iter().rev() {
    info!("New sub message for sub: {}", sub);
    bot
      .send_message(
        chat_id,
        entry.links.first().context("No links found")?.href.clone(),
      )
      .await?;
  }
  DB.write(|db| {
    db.subscribe.chat_to_sub.entry(chat_id).and_modify(|subs| {
      subs.entry(sub).and_modify(|subinfo| {
        *subinfo = SubInfo::Rss {
          last_entry: next_last_entry,
        };
      });
    });
  })?;
  DB.save()?;
  Ok(())
}

pub async fn listen(bot: AutoSend<Bot>) -> Result<()> {
  loop {
    let subs = DB.read(|db| db.subscribe.clone())?;
    let mut handles = vec![];
    for (chat_id, chat_subs) in subs.chat_to_sub {
      for (sub, sub_info) in chat_subs {
        handles.push(spawn(process_sub(bot.clone(), chat_id, sub, sub_info)));
      }
    }
    for handle in handles.into_iter() {
      handle.await?.unwrap_or_else(|err| error!("{:?}", err));
    }
    sleep(Duration::from_secs(600)).await;
  }
}
