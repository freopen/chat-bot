use anyhow::{anyhow, Result};
use bytes::Bytes;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct TelegramClient {
    token: String,
    http: reqwest::Client,
}

impl TelegramClient {
    pub fn new() -> Self {
        TelegramClient {
            token: std::env::var("TELEGRAM_TOKEN").expect("Unable to get Telegram token from env"),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create http client"),
        }
    }

    fn build_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    async fn request_to_json(&self, request: reqwest::RequestBuilder) -> Result<Value> {
        let response = request
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let mut response = match response {
            Value::Object(value) => value,
            _ => panic!(),
        };
        if response["ok"].as_bool().unwrap() {
            Ok(response.remove("result").unwrap())
        } else {
            Err(anyhow!("Telegram call returned error: {:?}", response))
        }
    }

    pub async fn call_method(&self, method: &str, params: Value) -> Result<Value> {
        let request = self.http.post(self.build_url(method)).json(&params);
        self.request_to_json(request).await
    }

    pub async fn get_updates(&self, offset: i64) -> Result<(i64, Value)> {
        let response = self
            .call_method(
                "getUpdates",
                json!({
                    "offset": offset,
                    "timeout": 60,
                    "allowed_updates": ["message"],
                }),
            )
            .await?;
        let new_offset = if let Some(last_update) = response.as_array().unwrap().last() {
            last_update["update_id"].as_i64().unwrap() + 1
        } else {
            offset
        };
        Ok((new_offset, response))
    }

    pub async fn flush_offset(&self, offset: i64) -> Result<()> {
        self.call_method(
            "getUpdates",
            json!({
                "offset": offset,
                "timeout": 0,
                "allowed_updates": ["message"],
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn get_file(&self, file_id: String) -> Result<Bytes> {
        let get_file_result = self
            .call_method("getFile", json!({ "file_id": file_id }))
            .await?;
        let file_path = get_file_result.as_object().unwrap()["file_path"]
            .as_str()
            .unwrap();
        let request = self.http.get(format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.token, file_path
        ));
        let bytes = request.send().await?.error_for_status()?.bytes().await?;
        Ok(bytes)
    }

    pub async fn send_photo(&self, photo: Vec<u8>, chat_id: i64, reply_to: i64) -> Result<Value> {
        let data_part = reqwest::multipart::Part::bytes(photo)
            .file_name("image.jpg")
            .mime_str("image/jpeg")?;
        let form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .text("reply_to_message_id", reply_to.to_string())
            .part("photo", data_part);
        let request = self.http.post(self.build_url("sendPhoto")).multipart(form);
        self.request_to_json(request).await
    }
}
