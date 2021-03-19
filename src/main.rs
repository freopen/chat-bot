use bytes::Bytes;
use image::GenericImageView;
use lazy_static::lazy_static;
use log::{error, info, warn};
use regex::Regex;
use serde_json::{json, Value};
use std::{io::Write, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Clone)]
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
                .timeout(Duration::from_secs(90))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create http client"),
            offset: 0,
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
            Err(format!("Telegram call returned error: {:?}", response).into())
        }
    }

    async fn call_method(&self, method: &str, params: &Value) -> Result<Value> {
        let request = self.http.post(self.build_url(method)).json(params);
        self.request_to_json(request).await
    }

    async fn get_updates(&mut self) -> Result<Value> {
        let response = self
            .call_method(
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

    async fn get_file(&self, file_id: String) -> Result<Bytes> {
        let get_file_result = self
            .call_method("getFile", &json!({ "file_id": file_id }))
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

    async fn send_photo(&self, photo: Vec<u8>, chat_id: i64, reply_to: i64) -> Result<Value> {
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

fn overlay_image(input_file: Bytes) -> Result<Vec<u8>> {
    let mut img = image::load_from_memory_with_format(&*input_file, image::ImageFormat::Jpeg)?;
    let ovr = image::open("assets/siriocra.png")?;
    let (img_w, img_h) = img.dimensions();
    let (ovr_w, ovr_h) = ovr.dimensions();
    if img_w * ovr_h < img_h * ovr_w {
        let new_ovr_h = ovr_h * img_w / ovr_w;
        let ovr = ovr.resize(img_w, new_ovr_h, image::imageops::CatmullRom);
        image::imageops::overlay(&mut img, &ovr, 0, img_h - new_ovr_h);
    } else {
        let new_ovr_w = ovr_w * img_h / ovr_h;
        let ovr = ovr.resize(new_ovr_w, img_h, image::imageops::CatmullRom);
        image::imageops::overlay(&mut img, &ovr, 0, 0);
    }
    let mut output = Vec::new();
    img.write_to(&mut output, image::ImageOutputFormat::Jpeg(100))?;
    Ok(output)
}

async fn process_update(update: Value, telegram_client: TelegramClient) -> Result<()> {
    info!("Processing update: {}", update);
    if let Some(message) = update.get("message") {
        let chat_id = message["chat"]["id"].as_i64().unwrap();
        let reply_to = message["message_id"].as_i64().unwrap();
        if let Some(Value::Array(ref sizes)) = message.get("photo").or(message
            .get("reply_to_message")
            .and_then(|origin| origin.get("photo")))
        {
            let file_id = sizes.last().unwrap().as_object().unwrap()["file_id"]
                .as_str()
                .unwrap();
            let input_file = telegram_client.get_file(file_id.into()).await?;
            let output_file = overlay_image(input_file)?;
            telegram_client
                .send_photo(output_file, chat_id, reply_to)
                .await?;
        } else {
            warn!("Photo was not found");
        }
    }
    Ok(())
}

async fn listen() -> ! {
    let mut telegram_client = TelegramClient::new();
    loop {
        let updates = {
            let updates = telegram_client.get_updates().await;
            if let Err(error) = updates {
                error!("{}", error);
                continue;
            }
            if let Value::Array(array) = updates.unwrap() {
                array
            } else {
                panic!()
            }
        };
        let joins: Vec<_> = updates
            .into_iter()
            .map(|update| tokio::spawn(process_update(update, telegram_client.clone())))
            .collect();
        for join in joins {
            if let Err(error) = join.await.unwrap() {
                error!("{}", error);
            }
        }
    }
}

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
        .filter(None, log::LevelFilter::Trace)
        .init();

    tokio::spawn(listen()).await.unwrap();
}
