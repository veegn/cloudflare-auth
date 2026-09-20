//! Workers Observability 结构化日志
//!
//! 以 JSON **对象** 调用 `console.log`，便于 Workers Logs 按字段索引/过滤。
//! 安全约定：绝不记录密码、JWT、App Secret、password_hash。

use serde_json::{json, Map, Value};
use worker::{js_sys, web_sys};

fn parse_js(value: &Value) -> wasm_bindgen::JsValue {
    js_sys::JSON::parse(&value.to_string())
        .unwrap_or_else(|_| wasm_bindgen::JsValue::from_str(&value.to_string()))
}

pub fn emit(value: Value) {
    web_sys::console::log_1(&parse_js(&value));
}

pub fn emit_warn(value: Value) {
    web_sys::console::warn_1(&parse_js(&value));
}

pub fn emit_error(value: Value) {
    web_sys::console::error_1(&parse_js(&value));
}

/// 通用业务事件：`event` + 可选字段
pub fn event(name: &str, fields: Value) {
    let mut map = Map::new();
    map.insert("event".into(), json!(name));
    if let Some(obj) = fields.as_object() {
        for (k, v) in obj {
            if k == "event" {
                continue;
            }
            map.insert(k.clone(), v.clone());
        }
    } else if !fields.is_null() {
        map.insert("detail".into(), fields);
    }
    emit(Value::Object(map));
}

/// 单次 HTTP 调用摘要（路径不含 query，避免 code/state 等敏感参数入日志）
pub fn http_request(
    method: &str,
    path: &str,
    status: u16,
    duration_ms: u64,
    error_code: Option<&str>,
) {
    let mut map = Map::new();
    map.insert("event".into(), json!("http_request"));
    map.insert("method".into(), json!(method));
    map.insert("path".into(), json!(path));
    map.insert("status".into(), json!(status));
    map.insert("duration_ms".into(), json!(duration_ms));
    if let Some(code) = error_code {
        map.insert("error".into(), json!(code));
        emit_warn(Value::Object(map));
    } else if status >= 500 {
        emit_error(Value::Object(map));
    } else {
        emit(Value::Object(map));
    }
}

/// HTTP 方法 → 规范大写标签
pub fn method_label(method: &worker::Method) -> &'static str {
    match method {
        worker::Method::Get => "GET",
        worker::Method::Post => "POST",
        worker::Method::Put => "PUT",
        worker::Method::Patch => "PATCH",
        worker::Method::Delete => "DELETE",
        worker::Method::Head => "HEAD",
        worker::Method::Options => "OPTIONS",
        _ => "OTHER",
    }
}
