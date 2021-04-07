use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone)]
pub struct TelegramClient {
    token: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct GetUpdates {
    offset: i64,
    limit: i64,
    timeout: i64,
    allowed_updates: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub reply_to_message: Option<Box<Message>>,
    pub media_group_id: Option<String>,
    pub text: Option<String>,
    pub photo: Option<Vec<PhotoSize>>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoSize {
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateEnum {
    Message(Message),
    ChannelPost(Message),
}

#[derive(Debug, Deserialize)]
pub struct Update {
    update_id: i64,
    #[serde(flatten)]
    pub update_enum: UpdateEnum,
}

#[derive(Deserialize)]
struct Response<T> {
    ok: bool,
    error_code: Option<i64>,
    description: Option<String>,
    result: Option<T>,
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

    async fn request_to_json<'de, T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T> {
        let response: Response<T> = request.send().await?.error_for_status()?.json().await?;
        if response.ok {
            Ok(response
                .result
                .context("No result field found in OK response")?)
        } else {
            Err(anyhow!(
                "Telegram call returned error: {} - {}",
                response
                    .error_code
                    .context("No error_code field found in Err response")?,
                response
                    .description
                    .context("No description field found in Err response")?
            ))
        }
    }

    pub async fn call_method<RequestType: Serialize, ResponseType: DeserializeOwned>(
        &self,
        method: &str,
        params: RequestType,
    ) -> Result<ResponseType> {
        let request = self.http.post(self.build_url(method)).json(&params);
        self.request_to_json(request).await
    }

    pub async fn get_updates(&self, offset: i64) -> Result<(i64, Vec<Update>)> {
        let response: Vec<Update> = self
            .call_method(
                "getUpdates",
                GetUpdates {
                    offset: offset,
                    limit: 100,
                    timeout: 60,
                    allowed_updates: ["message", "channel_post"].into(),
                },
            )
            .await?;
        let new_offset = if let Some(last_update) = response.last() {
            last_update.update_id + 1
        } else {
            offset
        };
        Ok((new_offset, response))
    }

    pub async fn flush_offset(&self, offset: i64) -> Result<()> {
        self.call_method::<_, Value>(
            "getUpdates",
            GetUpdates {
                offset: offset,
                limit: 1,
                timeout: 0,
                allowed_updates: ["message"].into(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn get_file(&self, file_id: String) -> Result<Bytes> {
        let get_file_result: Value = self
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

    pub async fn send_photo(&self, photo: Vec<u8>, chat_id: i64, reply_to: i64) -> Result<Message> {
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
