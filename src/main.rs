use futures::StreamExt;
use std::io::Write;
use telegram_bot::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("TELEGRAM_TOKEN").unwrap();
    let api = Api::new(token.as_str());
    let overlay = image::io::Reader::open("siriocra.png")?.decode()?;

    let mut stream = api.stream();
    stream.allowed_updates(&[AllowedUpdate::Message]);
    while let Some(update) = stream.next().await {
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            let message_id = message.id;
            let message_chat_id = message.chat.id();
            let mut send_photo = false;
            if let Some(source_message_box) = message.reply_to_message {
                if let MessageOrChannelPost::Message(source_message) = *source_message_box {
                    if let MessageKind::Photo { data, .. } = source_message.kind {
                        for photo in data.into_iter() {
                            let file = api.send(GetFile::new(photo)).await?;
                            let response =
                                reqwest::blocking::get(file.get_url(token.as_str()).unwrap())?;
                            let input_image_bytes = response.bytes()?;
                            let mut file = std::fs::File::create("input.jpg")?;
                            file.write_all(&input_image_bytes)?;
                            let mut image = image::io::Reader::open("input.jpg")?.decode()?;
                            image::imageops::overlay(&mut image, &overlay, 0, 0);
                            image.save("output.jpg")?;
                            send_photo = true;
                        }
                    }
                }
            }
            if send_photo {
                api.spawn(
                    SendPhoto::new(message_chat_id, InputFileUpload::with_path("output.jpg"))
                        .reply_to(message_id),
                );
            }
        }
    }
    Ok(())
}
