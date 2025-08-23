use scraper::{Html, Selector};

use crate::common::STATIC_FILE_URL;
use crate::utils::files::get_file_url;

/// 清理图片URL - 移除STATIC_FILE_URL前缀和查询参数
/// 将图片链接，转为路径。用于保存富文本的图片
pub fn to_html_image_paths(html: &str) -> String {
    let base = STATIC_FILE_URL.to_string() + "/";
    let document = Html::parse_document(html);
    let selector = Selector::parse("img").unwrap();
    let mut result = html.to_string();
    for element in document.select(&selector) {
        if let Some(src) = element.value().attr("src") {
            let src_amp = src.replace("&", "&amp;");
            // 移除 STATIC_FILE_URL 前缀
            let cleaned = src.replace(&base, "");
            // 移除查询参数
            let cleaned = cleaned.split('?').next().unwrap_or(&cleaned);
            if cleaned != src {
                result = result.replace(&src_amp, &cleaned).replace(&src, &cleaned);
            }
        }
    }
    result
}

/// 将图片路径转换为完整的URL - 添加STATIC_FILE_URL前缀
/// 将路径转为图片链接，用于显示富文本的图片
pub fn to_html_image_urls(html: &str) -> String {
    let base = STATIC_FILE_URL.to_string() + "/";
    let document = Html::parse_document(html);
    let selector = Selector::parse("img").unwrap();
    let mut result = html.to_string();
    for element in document.select(&selector) {
        if let Some(src) = element.value().attr("src") {
            let src_amp = src.replace("&", "&amp;");
            if !src.starts_with("http://")
                && !src.starts_with("https://")
                && !src.starts_with("data:")
                && !src.starts_with(&base)
            {
                let new_src = get_file_url(Some(src)).unwrap_or_default();
                result = result
                    .replace(
                        &format!("src=\"{}\"", src_amp),
                        &format!("src=\"{}\"", new_src),
                    )
                    .replace(&format!("src=\"{}\"", src), &format!("src=\"{}\"", new_src));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_html_image_paths() {
        const TEST_URL: &str = STATIC_FILE_URL;

        // 测试移除静态文件URL前缀
        let html = r#"<p><strong>“大”系列</strong>DS900/300C 加工中心开启了特雷维桑的“重型”系列。包括可以车削的中大型工件尺寸以及工作台的承重。</p><p><strong>工作台 </strong>DS900 /300 C加工中心使用直径达1500mm的固定件进行车削加工。1250mm或1400mm的标准台面可承载7000Kg的重量。可选项用1600 x 1600mm的台面增加承重到12000Kg，所有可用托盘更换系统。工作台是专门设计，以确保良好的排除加工所产生的残留物和切削液，便于操作人员工作。</p><p><strong>车削头</strong>DS900/300C - DS900/300C RAM型号，作为Trevisan加工中心的一个系列，配备了一个车削头，可以让你对固定工件进行车削操作。 尤其是这种装置，它的功能更加广泛，因为它在滑块中配备了两个刀座，因此单个刀具可以从第一个刀座移动到第二个刀座，并实现更大范围的车削直径。车削装置可以进行球面，圆锥，圆柱车削，和螺纹的加工，总是使用理想的切削参数，因为能够以恒定的切削速度工作。在这个模块中，为了优化精度，U轴平旋盘配备了平衡系统，平衡块从刀架的相反方向出来。</p><p><img src="https://rijin.csme.top/api/static/rijin/product/270446417775054848.jpg?expires=1745211878&amp;signature=MNJrjQPHHNu09xYFFVLbjAFAkEQ6Ap8P8qrHCia_2VC8cBaHj5wWgKjgTgUhnbqc"></p><p><strong>卧式主轴</strong>在主轴箱中，一个铣削/钻孔的套筒主轴用于扩大该型号的加工能力，37KW功率以及2000RPM转速，并且可以结合常规平旋盘实现车削加工。该主轴用于执行所有的铣削、钻孔和攻丝操作，并可配备角度头或其他附件。</p><p><strong>RAM主轴</strong>DS900/300C像它的姊妹机型DS600/200C一样，有一个RAM（镗杆），通过增加功率来提高机器的性能。在RAM机型中，卧式主轴不再在一个固定的位置，而是具有700mm的工作行程，更容易接近到零件。该主轴配备了U轴功能，可自动更换外置式旋转直径达250mm的平旋盘，转速高达2, 000 RPM。</p><p><strong>刀库</strong>刀具库是标准配置，有40个刀具位置，可扩展到80和120个刀位。该刀库可配备一个外部刀具装卸装置，使您可以安全地装载刀具或简单地检查或更换插入，而无需机器停机。它还可以配备内存芯片，将刀具信息从刀具测量仪直接传递到加工中心。<img src="https://rijin.csme.top/api/static/rijin/product/270442503898025984.jpg?expires=1745210945&amp;signature=MNJrjQPHHNu09xYFFVLbjPv0Pw26KLIpPYJkv-vNA0Io6h6lf6JEzcN1B0cbQ6Hj"></p>"#;

        println!("{}", to_html_image_paths(&html));
        // 测试移除静态文件URL前缀
        let html1 = format!(r#"<img src="{TEST_URL}/image1.jpg">"#);
        assert_eq!(to_html_image_paths(&html1), r#"<img src="image1.jpg">"#);

        // 测试移除查询参数
        let html2 = r#"<img src="image2.png?width=100&height=200">"#;
        assert_eq!(to_html_image_paths(html2), r#"<img src="image2.png">"#);

        // 测试同时移除前缀和查询参数
        let html3 = format!(r#"<img src="{TEST_URL}/image3.gif?timestamp=123">"#);
        assert_eq!(to_html_image_paths(&html3), r#"<img src="image3.gif">"#);

        // 测试不需要修改的情况
        let html4 = r#"<img src="local.png">"#;
        assert_eq!(to_html_image_paths(html4), html4);

        // 测试多个图片的情况
        let html5 = format!(r#"<div><img src="{TEST_URL}/1.jpg"><img src="2.png?x=1"></div>"#);
        assert_eq!(
            to_html_image_paths(&html5),
            r#"<div><img src="1.jpg"><img src="2.png"></div>"#
        );
    }

    #[test]
    fn test_to_html_image_urls() {
        const TEST_URL: &str = STATIC_FILE_URL;

        // 测试添加静态文件URL前缀
        let html1 = r#"<p><strong>“大”系列</strong>DS900/300C 加工中心开启了特雷维桑的“重型”系列。包括可以车削的中大型工件尺寸以及工作台的承重。</p><p><strong>工作台 </strong>DS900 /300 C加工中心使用直径达1500mm的固定件进行车削加工。1250mm或1400mm的标准台面可承载7000Kg的重量。可选项用1600 x 1600mm的台面增加承重到12000Kg，所有可用托盘更换系统。工作台是专门设计，以确保良好的排除加工所产生的残留物和切削液，便于操作人员工作。</p><p><strong>车削头</strong>DS900/300C - DS900/300C RAM型号，作为Trevisan加工中心的一个系列，配备了一个车削头，可以让你对固定工件进行车削操作。 尤其是这种装置，它的功能更加广泛，因为它在滑块中配备了两个刀座，因此单个刀具可以从第一个刀座移动到第二个刀座，并实现更大范围的车削直径。车削装置可以进行球面，圆锥，圆柱车削，和螺纹的加工，总是使用理想的切削参数，因为能够以恒定的切削速度工作。在这个模块中，为了优化精度，U轴平旋盘配备了平衡系统，平衡块从刀架的相反方向出来。</p><p><img src="rijin/product/270446417775054848.jpg"></p><p><strong>卧式主轴</strong>在主轴箱中，一个铣削/钻孔的套筒主轴用于扩大该型号的加工能力，37KW功率以及2000RPM转速，并且可以结合常规平旋盘实现车削加工。该主轴用于执行所有的铣削、钻孔和攻丝操作，并可配备角度头或其他附件。</p><p><strong>RAM主轴</strong>DS900/300C像它的姊妹机型DS600/200C一样，有一个RAM（镗杆），通过增加功率来提高机器的性能。在RAM机型中，卧式主轴不再在一个固定的位置，而是具有700mm的工作行程，更容易接近到零件。该主轴配备了U轴功能，可自动更换外置式旋转直径达250mm的平旋盘，转速高达2, 000 RPM。</p><p><strong>刀库</strong>刀具库是标准配置，有40个刀具位置，可扩展到80和120个刀位。该刀库可配备一个外部刀具装卸装置，使您可以安全地装载刀具或简单地检查或更换插入，而无需机器停机。它还可以配备内存芯片，将刀具信息从刀具测量仪直接传递到加工中心。<img src="rijin/product/270442503898025984.jpg"></p>"#;
        to_html_image_urls(html1);
        println!("{}", to_html_image_urls(html1));

        // 测试添加静态文件URL前缀
        let html1 = r#"<img src="image1.jpg">"#;
        assert_eq!(
            to_html_image_urls(html1),
            format!(r#"<img src="{TEST_URL}/image1.jpg">"#)
        );

        // 测试不需要修改的情况(已有http://)
        let html2 = r#"<img src="http://example.com/image2.png">"#;
        assert_eq!(to_html_image_urls(html2), html2);

        // 测试不需要修改的情况(已有https://)
        let html3 = r#"<img src="https://example.com/image3.gif">"#;
        assert_eq!(to_html_image_urls(html3), html3);

        // 测试不需要修改的情况(已有STATIC_FILE_URL)
        let html4 = format!(r#"<img src="{TEST_URL}/image4.png">"#);
        assert_eq!(to_html_image_urls(&html4), html4);

        // 测试多个图片的情况
        let html5 = r#"<div><img src="1.jpg"><img src="https://site.com/2.png"></div>"#;
        assert_eq!(
            to_html_image_urls(html5),
            format!(r#"<div><img src="{TEST_URL}/1.jpg"><img src="https://site.com/2.png"></div>"#)
        );
    }
}
