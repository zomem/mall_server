use anyhow::Result;
use base64::{Engine, engine::general_purpose};
use fast_qr::convert::{Builder, Shape, image::ImageBuilder};
use fast_qr::qr::QRBuilder;

/// 生成二维码，返回图片 base64编码
pub fn generate_qrcode(data: &str) -> Result<String> {
    let qrcode = QRBuilder::new(data).build()?;
    let img = ImageBuilder::default()
        .shape(Shape::Square)
        .background_color([255, 255, 255, 255]) // Handles transparency
        .fit_width(200)
        .to_bytes(&qrcode)?;
    Ok(vec_to_data_url(img, "image/png"))
}

/// 将 图片 vec 转成 base64
fn vec_to_data_url(image_data: Vec<u8>, mime_type: &str) -> String {
    let base64_string = general_purpose::STANDARD.encode(&image_data);
    format!("data:{};base64,{}", mime_type, base64_string)
}

#[cfg(test)]
mod test {
    use super::*;
    use fast_qr::convert::{Builder, Shape, image::ImageBuilder};
    use fast_qr::qr::QRBuilder;

    #[test]
    fn test_qrcode() {
        let qrcode = QRBuilder::new("https://example.com/").build().unwrap();
        let _img = ImageBuilder::default()
            .shape(Shape::Square)
            .background_color([255, 255, 255, 255]) // Handles transparency
            .fit_width(600)
            .to_file(&qrcode, "out.png");

        // let data = ImageBuilder::default()
        //     .shape(Shape::Square)
        //     .background_color([255, 255, 255, 255]) // Handles transparency
        //     .fit_width(600)
        //     .to_bytes(&qrcode)
        //     .unwrap();
        // println!("{:?}", data);

        let url = generate_qrcode("https://example.com/").unwrap();
        println!("{:?}", url);
    }
}
