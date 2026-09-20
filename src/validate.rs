//! 业务校验、redirect_uri 序列化、App 凭证解析

use worker::*;

use crate::db::{js_str, db_first, AppRow};
use crate::http::{api_err, ApiResult};
use crate::password::verify_password;

pub fn email_valid(email: &str) -> bool {
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !email.chars().any(|c| c.is_whitespace())
}

pub fn username_valid(name: &str, min: usize, max: usize) -> bool {
    let len = name.chars().count();
    len >= min
        && len <= max
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn parse_redirect_uris(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(20)
        .collect()
}

pub fn serialize_redirect_uris(list: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    for item in list {
        let uri = item.trim();
        if uri.is_empty() || uri.len() > 2048 || out.iter().any(|x| x == uri) {
            continue;
        }
        out.push(uri.to_string());
        if out.len() >= 20 {
            break;
        }
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

/// 仅接受 http(s) 自定义图标；非法值视为未设置
pub fn normalize_icon_url(raw: Option<&str>) -> Option<String> {
    let url = raw?.trim();
    if url.is_empty() || url.len() > 512 {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return None;
    }
    Some(url.to_string())
}

/// `X-App-Id` + `X-App-Secret`：缺省 → None；半套/错误 → 401
pub async fn resolve_app_credentials(
    env: &Env,
    app_id: Option<&str>,
    app_secret: Option<&str>,
) -> ApiResult<Option<AppRow>> {
    let invalid = || api_err(401, "invalid_app_credentials", "Invalid app credentials");
    match (app_id, app_secret) {
        (None, None) => Ok(None),
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
            let app = db_first::<AppRow>(
                env,
                "SELECT * FROM apps WHERE app_id = ?",
                &[js_str(id)],
            )
            .await?
            .ok_or_else(invalid)?;
            if app.status != "active" || !verify_password(secret, &app.app_secret_hash) {
                return Err(invalid());
            }
            Ok(Some(app))
        }
        _ => Err(invalid()),
    }
}

/// 从请求头解析可选 App 凭证（register/login 的 X-App-* 门控）
pub async fn optional_app_credentials(req: &Request, env: &Env) -> ApiResult<Option<AppRow>> {
    let id = req.headers().get("x-app-id").ok().flatten();
    let secret = req.headers().get("x-app-secret").ok().flatten();
    resolve_app_credentials(env, id.as_deref(), secret.as_deref()).await
}

/// App 凭证必填场景：无凭证直接 401
pub async fn require_app_credentials(req: &Request, env: &Env) -> ApiResult<AppRow> {
    match optional_app_credentials(req, env).await? {
        Some(app) => Ok(app),
        None => Err(api_err(
            401,
            "invalid_app_credentials",
            "X-App-Id and X-App-Secret are required",
        )),
    }
}

/// `REQUIRE_APP_CREDENTIALS=true` 时强制携带 App 凭证
pub async fn ensure_app_credentials_if_required(
    req: &Request,
    env: &Env,
) -> ApiResult<Option<AppRow>> {
    let app = optional_app_credentials(req, env).await?;
    if crate::config::require_app_credentials(env) && app.is_none() {
        return Err(api_err(
            401,
            "invalid_app_credentials",
            "X-App-Id and X-App-Secret are required",
        ));
    }
    Ok(app)
}
