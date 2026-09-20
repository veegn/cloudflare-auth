//! 上传图像处理：强制中心裁剪为正方形 → 缩放 → JPEG 压缩
//!
//! 服务端强制执行，客户端预处理不可跳过。

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;

use crate::http::{api_err, ApiResult};

pub const MAX_UPLOAD_BYTES: usize = 5 * 1024 * 1024;
pub const MIN_EDGE_PX: u32 = 16;
pub const AVATAR_SIZE: u32 = 256;
pub const APP_ICON_SIZE: u32 = 256;
pub const JPEG_QUALITY: u8 = 82;
pub const JPEG_CONTENT_TYPE: &str = "image/jpeg";

/// 解码 → 中心裁剪正方形 → 缩放到 target → JPEG
pub fn crop_resize_jpeg(input: &[u8], target: u32) -> ApiResult<Vec<u8>> {
    if input.is_empty() {
        return Err(api_err(400, "invalid_image", "Empty image body"));
    }
    if input.len() > MAX_UPLOAD_BYTES {
        return Err(api_err(
            413,
            "image_too_large",
            format!("Image must be <= {} bytes", MAX_UPLOAD_BYTES),
        ));
    }
    let img = image::load_from_memory(input).map_err(|_| {
        api_err(
            400,
            "invalid_image",
            "Unsupported or corrupt image (jpeg/png/gif/webp)",
        )
    })?;
    let (w, h) = (img.width(), img.height());
    if w < MIN_EDGE_PX || h < MIN_EDGE_PX {
        return Err(api_err(
            400,
            "image_too_small",
            format!("Image must be at least {MIN_EDGE_PX}px on each side"),
        ));
    }
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    let cropped = img.crop_imm(x, y, side, side);
    let resized = cropped.resize_exact(target, target, FilterType::Triangle);
    let rgb = resized.to_rgb8();
    let mut buf: Vec<u8> = Vec::with_capacity(target as usize * target as usize / 4);
    {
        let enc = JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
        rgb.write_with_encoder(enc)
            .map_err(|_| api_err(500, "internal_error", "JPEG encode failed"))?;
    }
    if buf.len() as u32 >= side * side && buf.len() > 64 * 1024 {
        // 压缩结果异常偏大时仍允许，但限制在合理上限
        if buf.len() > 512 * 1024 {
            return Err(api_err(
                400,
                "image_too_large",
                "Compressed image exceeded size budget",
            ));
        }
    }
    Ok(buf)
}
