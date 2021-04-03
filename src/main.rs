mod enhance;
mod telegram_bot;
mod telegram_client;

use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use std::io::Write;
use tokio::sync::watch;

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
    tokio::spawn(telegram_bot::listen(ctrl_c))
        .await
        .unwrap()
        .unwrap();
}
