//! HTTP 响应、错误类型与请求体工具

use serde_json::{json, Value};
use worker::*;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

pub fn api_err(status: u16, code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError {
        status,
        code,
        message: message.into(),
    }
}

/// 成功 JSON 响应（`cache-control: no-store`）
pub fn ok_json(value: Value) -> ApiResult<Response> {
    let mut res = Response::from_json(&value)
        .map_err(|_| api_err(500, "internal_error", "serialize failed"))?;
    if let Ok(h) = res.headers_mut().set("cache-control", "no-store") {
        let _ = h;
    }
    Ok(res)
}

pub fn json_err(status: u16, code: &str, message: &str) -> Response {
    let body = json!({ "error": code, "message": message });
    match Response::from_json(&body) {
        Ok(res) => res.with_status(status),
        Err(_) => Response::error("error", status).unwrap_or_else(|_| Response::ok("").unwrap()),
    }
}

pub async fn body_json<T: serde::de::DeserializeOwned>(req: &mut Request) -> ApiResult<T> {
    req.json::<T>()
        .await
        .map_err(|_| api_err(400, "invalid_json", "Body must be JSON"))
}

/// 依次尝试 keys，返回首个非空字符串字段（trim 后）
pub fn body_str(body: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = body.get(*k).and_then(|v| v.as_str()) {
            let s = s.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    // 仍返回第一个出现的字段（即使空白），与原先 unwrap_or("") + 下游校验兼容
    for k in keys {
        if let Some(s) = body.get(*k).and_then(|v| v.as_str()) {
            return s.trim().to_string();
        }
    }
    String::new()
}

/// 不 trim 的原始字符串（密码等）
pub fn body_raw(body: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = body.get(*k).and_then(|v| v.as_str()) {
            return s.to_string();
        }
    }
    String::new()
}

pub fn preflight() -> Response {
    Response::empty()
        .unwrap()
        .with_status(204)
        .with_headers(preflight_headers())
}

fn preflight_headers() -> Headers {
    let h = Headers::new();
    h.set("access-control-allow-origin", "*").ok();
    h.set(
        "access-control-allow-headers",
        "content-type,authorization,x-app-id,x-app-secret",
    )
    .ok();
    h.set(
        "access-control-allow-methods",
        "GET,POST,PUT,PATCH,DELETE,OPTIONS",
    )
    .ok();
    h
}

pub fn with_cors(mut res: Response) -> Response {
    res.headers_mut().set("access-control-allow-origin", "*").ok();
    res
}

/// identicon / SVG 公共响应
pub fn svg_response(svg: String, max_age_secs: u32) -> ApiResult<Response> {
    let headers = Headers::new();
    headers
        .set("content-type", "image/svg+xml; charset=utf-8")
        .ok();
    headers
        .set("cache-control", &format!("public, max-age={max_age_secs}"))
        .ok();
    Response::from_body(ResponseBody::Body(svg.into_bytes().into()))
        .map(|r| r.with_headers(headers))
        .map_err(|_| api_err(500, "internal_error", "svg response failed"))
}

/// percent-decode（路径 / query 片段）
pub fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < s.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
