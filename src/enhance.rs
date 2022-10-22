use anyhow::Result;
use image::GenericImageView;

pub(crate) fn overlay_image(filename: &str, input_file: Vec<u8>, mirror: bool) -> Result<Vec<u8>> {
  let mut img = image::load_from_memory(&input_file)?;
  let assets_folder = {
    let mut result = std::env::current_exe()?;
    result.pop();
    result.pop();
    result.push("assets");
    result
  };
  let ovr = image::open(format!("{}/{}.png", assets_folder.display(), filename))?;
  let (img_w, img_h) = img.dimensions();
  let (ovr_w, ovr_h) = ovr.dimensions();
  let (new_ovr_w, new_ovr_h) = if img_w * ovr_h < img_h * ovr_w {
    (img_w, ovr_h * img_w / ovr_w)
  } else {
    (ovr_w * img_h / ovr_h, img_h)
  };
  let ovr = ovr.resize(new_ovr_w, new_ovr_h, image::imageops::CatmullRom);
  if mirror {
    image::imageops::overlay(&mut img, &ovr.fliph(), img_w - new_ovr_w, img_h - new_ovr_h);
  } else {
    image::imageops::overlay(&mut img, &ovr, 0, img_h - new_ovr_h);
  }
  let mut output = Vec::new();
  img.write_to(&mut output, image::ImageOutputFormat::Jpeg(100))?;
  Ok(output)
}
