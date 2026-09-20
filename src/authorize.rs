//! OAuth-like 授权流：inspect / complete / code exchange
//!
//! 会话创建见 `session.rs`；时间工具见 `time.rs`。

use crate::db::{js_str, db_first, load_user, public_user, AppRow, AuthCodeRow, UserRow};
use crate::http::{api_err, json_err, ok_json, ApiResult};
use crate::password::{random_hex, verify_password};
use crate::session::create_session;
use crate::time::{format_unix_iso, now_ts, parse_iso_pub};
use crate::validate::parse_redirect_uris;

const CODE_TTL_SECS: i64 = 300;

#[derive(serde::Deserialize)]
pub struct AuthorizeQuery {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub response_type: Option<String>,
}

/// 从 URL query 构造 AuthorizeQuery（兼容 `app_id` → `client_id`）
pub fn query_from_url(url: &worker::Url) -> AuthorizeQuery {
    let mut map = std::collections::HashMap::new();
    if let Some(q) = url.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(
                    crate::http::url_decode(k),
                    crate::http::url_decode(v),
                );
            }
        }
    }
    AuthorizeQuery {
        client_id: map
            .get("client_id")
            .cloned()
            .or_else(|| map.get("app_id").cloned()),
        redirect_uri: map.get("redirect_uri").cloned(),
        state: map.get("state").cloned(),
        response_type: map.get("response_type").cloned(),
    }
}

fn normalize_uri(uri: &str) -> String {
    uri.trim().to_string()
}

fn parse_rt(raw: Option<&str>) -> ApiResult<String> {
    match raw.unwrap_or("code").to_ascii_lowercase().as_str() {
        "code" => Ok("code".into()),
        "token" => Ok("token".into()),
        _ => Err(api_err(
            400,
            "unsupported_response_type",
            "response_type must be code or token",
        )),
    }
}

pub async fn load_active_app(env: &worker::Env, client_id: &str) -> ApiResult<AppRow> {
    let app = db_first::<AppRow>(
        env,
        "SELECT * FROM apps WHERE app_id = ?",
        &[js_str(client_id)],
    )
    .await?
    .ok_or_else(|| api_err(400, "invalid_client", "Unknown or revoked client"))?;
    if app.status != "active" {
        return Err(api_err(400, "invalid_client", "Unknown or revoked client"));
    }
    Ok(app)
}

fn redirect_allowed(app: &AppRow, redirect_uri: &str) -> bool {
    parse_redirect_uris(Some(&app.redirect_uris))
        .iter()
        .any(|u| u == redirect_uri)
}

/// GET /authorize：参数校验 + 应用元信息（给控制台确认页用）
pub async fn inspect_authorize(
    env: &worker::Env,
    query: AuthorizeQuery,
) -> worker::Response {
    let client_id = query.client_id.unwrap_or_default().trim().to_string();
    let redirect_uri = normalize_uri(query.redirect_uri.as_deref().unwrap_or(""));
    if client_id.is_empty() {
        return json_err(400, "invalid_request", "client_id is required");
    }
    if redirect_uri.is_empty() {
        return json_err(400, "invalid_request", "redirect_uri is required");
    }
    let rt = match parse_rt(query.response_type.as_deref()) {
        Ok(v) => v,
        Err(e) => return json_err(e.status, e.code, &e.message),
    };
    let app = match load_active_app(env, &client_id).await {
        Ok(a) => a,
        Err(e) => return json_err(e.status, e.code, &e.message),
    };
    if !redirect_allowed(&app, &redirect_uri) {
        return json_err(
            400,
            "invalid_request",
            "redirect_uri is not registered for this app",
        );
    }
    ok_json(serde_json::json!({
        "ok": true,
        "app": {
            "appId": app.app_id,
            "name": app.name,
            "description": app.description,
            "iconUrl": crate::validate::normalize_icon_url(app.icon_url.as_deref()),
            "iconPath": format!("/v1/apps/{}/icon", app.app_id),
        },
        "redirectUri": redirect_uri,
        "state": query.state.unwrap_or_default(),
        "responseType": rt,
    }))
    .unwrap_or_else(|e| json_err(e.status, e.code, &e.message))
}

/// POST /auth/authorize：用户确认后发 code 或直接 token
pub async fn complete_authorize(
    env: &worker::Env,
    user: &UserRow,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    response_type: &str,
) -> ApiResult<worker::Response> {
    let rt = parse_rt(Some(response_type))?;
    let redirect_uri = normalize_uri(redirect_uri);
    if client_id.trim().is_empty() || redirect_uri.is_empty() {
        return Err(api_err(
            400,
            "invalid_request",
            "client_id and redirect_uri are required",
        ));
    }
    let app = load_active_app(env, client_id.trim()).await?;
    if !redirect_allowed(&app, &redirect_uri) {
        return Err(api_err(
            400,
            "invalid_request",
            "redirect_uri is not registered for this app",
        ));
    }

    if rt == "code" {
        let code = format!("ac_{}", random_hex(24));
        let expires_at = format_unix_iso(now_ts() + CODE_TTL_SECS);
        db_run_auth_code(env, &code, &app.app_id, &user.id, &redirect_uri, &expires_at).await?;
        let mut url = format!("{redirect_uri}?code={code}");
        if !state.is_empty() {
            url.push_str(&format!("&state={state}"));
        }
        return ok_json(serde_json::json!({
            "redirectTo": url,
            "responseType": "code",
        }));
    }

    let (token, _) = create_session(env, &user.id, Some(&app.app_id)).await?;
    let ttl = crate::config::token_ttl(env);
    let mut frag = format!("access_token={token}&token_type=Bearer&expires_in={ttl}");
    if !state.is_empty() {
        frag.push_str(&format!("&state={state}"));
    }
    ok_json(serde_json::json!({
        "redirectTo": format!("{redirect_uri}#{frag}"),
        "responseType": "token",
    }))
}

async fn db_run_auth_code(
    env: &worker::Env,
    code: &str,
    app_id: &str,
    user_id: &str,
    redirect_uri: &str,
    expires_at: &str,
) -> ApiResult<()> {
    crate::db::db_run(
        env,
        "INSERT INTO auth_codes (code, app_id, user_id, redirect_uri, expires_at, used) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        &[
            js_str(code),
            js_str(app_id),
            js_str(user_id),
            js_str(redirect_uri),
            js_str(expires_at),
        ],
    )
    .await
}

/// POST /auth/token：一次性授权码换 JWT
pub async fn exchange_code(
    env: &worker::Env,
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> ApiResult<worker::Response> {
    let code = code.trim();
    let client_id = client_id.trim();
    if code.is_empty() || client_id.is_empty() || client_secret.is_empty() {
        return Err(api_err(
            400,
            "invalid_request",
            "code, client_id and client_secret are required",
        ));
    }

    let row = db_first::<AuthCodeRow>(env, "SELECT * FROM auth_codes WHERE code = ?", &[js_str(code)])
        .await?
        .ok_or_else(|| {
            api_err(
                400,
                "invalid_grant",
                "Authorization code is invalid or already used",
            )
        })?;
    if row.used != 0 || row.app_id != client_id {
        return Err(api_err(
            400,
            "invalid_grant",
            "Authorization code is invalid or already used",
        ));
    }
    if parse_iso_pub(&row.expires_at) < now_ts() {
        return Err(api_err(400, "invalid_grant", "Authorization code expired"));
    }

    let app = load_active_app(env, client_id).await?;
    if !verify_password(client_secret, &app.app_secret_hash) {
        return Err(api_err(401, "invalid_client", "Invalid client secret"));
    }

    let user = load_user(env, &row.user_id).await?;
    crate::db::db_run(
        env,
        "UPDATE auth_codes SET used = 1 WHERE code = ? AND used = 0",
        &[js_str(code)],
    )
    .await?;

    let (token, _) = create_session(env, &user.id, Some(&app.app_id)).await?;
    ok_json(serde_json::json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": crate::config::token_ttl(env),
        "appId": app.app_id,
    }))
}
