use log::{error, info};
use serde_json::{json, Value};
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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

    fn build_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    async fn run_method(&self, method: &str, params: &Value) -> Result<Value> {
        let request = self
            .http
            .post(self.build_url("getUpdates"))
            .timeout(Duration::from_secs(if method == "getUpdates" {
                90
            } else {
                10
            }))
            .json(params);
        let response = request.send().await?.json::<Value>().await?;
        let mut response = match response {
            Value::Object(value) => value,
            _ => panic!(),
        };
        if response["ok"].as_bool().unwrap() {
            Ok(response.remove("result").unwrap())
        } else {
            Err(format!("getUpdates returned error: {:?}", response).into())
        }
    }

    async fn get_updates(&mut self) -> Result<Value> {
        let response = self
            .run_method(
                "getUpdates",
                &json!({
                    "offset": self.offset,
                    "timeout": 60,
                    "allowed_updates": ["message"],
                }),
            )
            .await?;
        if let Some(last_update) = response.as_array().unwrap().last() {
            self.offset = last_update["update_id"].as_i64().unwrap() + 1;
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
