//! R2 媒体对象读写（绑定名 `MEDIA`）

use worker::{Bucket, Env, HttpMetadata};

use crate::http::{api_err, bytes_response, ApiResult};
use crate::media::JPEG_CONTENT_TYPE;

pub const AVATAR_PREFIX: &str = "avatars";
pub const APP_ICON_PREFIX: &str = "apps";
const CACHE_CONTROL: &str = "public, max-age=604800, immutable";

fn bucket(env: &Env) -> ApiResult<Bucket> {
    env.bucket("MEDIA")
        .map_err(|_| api_err(500, "internal_error", "R2 MEDIA binding missing"))
}

pub fn avatar_key(user_id: &str) -> String {
    format!("{AVATAR_PREFIX}/{user_id}.jpg")
}

pub fn app_icon_key(app_public_id: &str) -> String {
    format!("{APP_ICON_PREFIX}/{app_public_id}.jpg")
}

pub async fn put_jpeg(env: &Env, key: &str, bytes: Vec<u8>) -> ApiResult<()> {
    let bucket = bucket(env)?;
    bucket
        .put(key.to_string(), bytes)
        .http_metadata(HttpMetadata {
            content_type: Some(JPEG_CONTENT_TYPE.to_string()),
            cache_control: Some(CACHE_CONTROL.to_string()),
            ..Default::default()
        })
        .custom_metadata({
            let mut m = std::collections::HashMap::new();
            m.insert("processed".to_string(), "crop-resize-jpeg".to_string());
            m
        })
        .execute()
        .await
        .map_err(|_| api_err(500, "internal_error", "R2 put failed"))?;
    Ok(())
}

/// 读取 R2 对象；不存在返回 None
pub async fn get_jpeg_response(env: &Env, key: &str) -> ApiResult<Option<worker::Response>> {
    let bucket = bucket(env)?;
    let object = bucket
        .get(key)
        .execute()
        .await
        .map_err(|_| api_err(500, "internal_error", "R2 get failed"))?;
    let Some(object) = object else {
        return Ok(None);
    };
    let Some(body) = object.body() else {
        return Ok(None);
    };
    // 小图直接读字节，便于统一 content-type/cache 头
    let bytes = body
        .bytes()
        .await
        .map_err(|_| api_err(500, "internal_error", "R2 body read failed"))?;
    Ok(Some(bytes_response(
        bytes,
        JPEG_CONTENT_TYPE,
        "public, max-age=3600",
    )?))
}

pub async fn delete_quiet(env: &Env, key: &str) {
    if let Ok(bucket) = bucket(env) {
        let _ = bucket.delete(key).await;
    }
}
