//! 静态资源 + HTML 品牌占位符注入

use worker::*;

use crate::config;
use crate::http::{api_err, ApiResult};

pub async fn serve_assets(env: &Env, url: &str) -> ApiResult<Response> {
    let assets = env
        .assets("ASSETS")
        .map_err(|_| api_err(500, "internal_error", "Assets binding missing"))?;
    let mut res = assets
        .fetch(url, None)
        .await
        .map_err(|_| api_err(500, "internal_error", "Asset fetch failed"))?;
    let ct = res
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !ct.contains("text/html") || res.status_code() >= 400 {
        return Ok(res);
    }
    let html = res
        .text()
        .await
        .map_err(|_| api_err(500, "internal_error", "Asset read failed"))?;
    let name = config::app_name(env);
    let service = config::service_url(env);
    let replaced = html
        .replace("{{APP_NAME}}", &name)
        .replace("{{SERVICE_URL}}", &service)
        .replace("https://cloudflare-auth.veegn.workers.dev", &service);
    let headers = Headers::new();
    headers
        .set("content-type", "text/html; charset=utf-8")
        .ok();
    headers.set("cache-control", "no-store").ok();
    Response::from_body(ResponseBody::Body(replaced.into_bytes().into()))
        .map(|r| r.with_headers(headers))
        .map_err(|_| api_err(500, "internal_error", "Asset response failed"))
}
