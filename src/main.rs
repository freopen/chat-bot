use log::{error, info};
use std::{cmp::min, error::Error, time::Duration};

use serde_json::json;

struct TelegramClient {
    token: String,
    http: reqwest::Client,
    offset: i64,
}

impl TelegramClient {
    fn new() -> Self {
        TelegramClient {
            token: std::env::var("TELEGRAM_TOKEN").expect("Unable to get Telegram token from env"),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create http client"),
            offset: 0,
        }
    }

    fn build_url(&self, op: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, op)
    }

    async fn get_updates(&mut self) -> Result<serde_json::Value, Box<dyn Error>> {
        info!("Getting updates");
        let request = self
            .http
            .get(self.build_url("getUpdates"))
            .timeout(Duration::from_secs(90))
            .json(&json!({
                "offset": self.offset,
                "timeout": 60,
                "allowed_updates": ["message"],
            }));
        let response = request.send().await?.json::<serde_json::Value>().await?;
        if !response["ok"]
            .as_bool()
            .ok_or("No ok property in response")?
        {
            return Err("Response is not ok".into());
        }

        {
            let updates = response["result"]
                .as_array()
                .ok_or("getUpdates response is not an array")?;
            let update_ids = updates
                .iter()
                .map(|update| update["update_id"].as_i64().ok_or("No update_id in update"));
            let mut min_update_id = i64::MAX;
            for update_id in update_ids {
                min_update_id = min(min_update_id, update_id?);
            }
            if min_update_id != i64::MAX {
                self.offset = min_update_id + 1
            };
        };

        Ok(response)
    }
}

async fn telegram_bot() -> ! {
    let mut client = TelegramClient::new();
    loop {
        match client.get_updates().await {
            Ok(response) => {
                info!("{}", response)
            }
            Err(error) => {
                error!("{}", error)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    tokio::spawn(telegram_bot()).await.unwrap();
}
