use anyhow::Result;
use teloxide::prelude::*;

use crate::db::*;

pub async fn subscribe_command(
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
                subs.insert(
                    url.to_string(),
                    SubInfo::RSS {
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
                    .map(|subs| subs.keys().map(|s| s.clone()).collect())
                    .unwrap_or(vec![])
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
