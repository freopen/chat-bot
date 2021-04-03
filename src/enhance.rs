use bytes::Bytes;
use anyhow::Result;
use image::GenericImageView;

pub(crate) fn overlay_image(input_file: Bytes) -> Result<Vec<u8>> {
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