mod discord_bot;
mod enhance;
mod telegram_bot;

use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use std::io::Write;

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

#[tokio::main]
async fn main() {
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
    let telegram_thread = tokio::spawn(telegram_bot::listen());
    let discord_thread = tokio::spawn(discord_bot::listen());
    telegram_thread.await.unwrap().unwrap();
    discord_thread.await.unwrap().unwrap();
}
