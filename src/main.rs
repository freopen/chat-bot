mod db;
mod discord_bot;
mod enhance;
mod subscribe;
mod telegram_bot;

use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use std::io::Write;
use tokio::{
    signal::unix::{signal, SignalKind},
    spawn,
};

use crate::db::*;

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

async fn run() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::Builder::new()
        .format(|buf, record| {
            write!(
                buf,
                "{} {} {} ",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                buf.default_styled_level(record.level()),
                record.module_path().unwrap_or("<unknown>"),
            )?;
            format_path(
                record.file().unwrap_or("<unknown>"),
                record.line().unwrap_or(0),
                buf,
            )?;
            writeln!(buf, " {}", record.args())
        })
        .filter_level(log::LevelFilter::Warn)
        .filter_module("freopen_chat_bot", log::LevelFilter::Info)
        .init();

    info!("Listening for telegram updates...");
    let telegram_thread = spawn(telegram_bot::listen());
    let discord_thread = spawn(discord_bot::listen());
    signal(SignalKind::terminate())?.recv().await;
    info!("Abort signal received");
    telegram_thread.abort();
    info!("Telegram thread aborted");
    discord_thread.abort();
    info!("Discord thread aborted");
    DB.flush()?;
    info!("DB flushed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
