//! 重定向登录：inspect / complete / token exchange

use serde_json::Value;
use worker::*;

use crate::authorize;
use crate::http::{body_json, body_str, ApiResult};
use crate::logging;
use crate::session::require_auth;

/// GET /authorize 校验参数，返回应用卡片信息（不发 code）
pub async fn handle_inspect_authorize(env: &Env, url: &Url) -> ApiResult<Response> {
    Ok(authorize::inspect_authorize(env, authorize::query_from_url(url)).await)
}

/// 已登录用户确认授权 → redirectTo
pub async fn handle_complete_authorize(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let body: Value = body_json(req).await?;
    let client_id = body_str(&body, &["client_id", "clientId"]);
    let redirect_uri = body_str(&body, &["redirect_uri", "redirectUri"]);
    let state = body_str(&body, &["state"]);
    let rt = body
        .get("response_type")
        .or_else(|| body.get("responseType"))
        .and_then(|v| v.as_str())
        .unwrap_or("code")
        .to_string();
    let resp = authorize::complete_authorize(env, &ctx.user, &client_id, &redirect_uri, &state, &rt).await;
    if resp.is_ok() {
        logging::event(
            "authorize_completed",
            serde_json::json!({
                "user_id": ctx.user.id,
                "client_id": client_id,
                "response_type": rt,
            }),
        );
    }
    resp
}

/// 授权码换 token（App 凭证）
pub async fn handle_token_exchange(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let body: Value = body_json(req).await?;
    let code = body_str(&body, &["code"]);
    let client_id = body_str(&body, &["client_id", "clientId"]);
    let client_secret = body_str(&body, &["client_secret", "clientSecret", "appSecret"]);
    let resp = authorize::exchange_code(env, &code, &client_id, &client_secret).await;
    if resp.is_ok() {
        logging::event(
            "token_exchanged",
            serde_json::json!({ "client_id": client_id }),
        );
    } else if let Err(e) = &resp {
        logging::event(
            "token_exchange_failed",
            serde_json::json!({ "client_id": client_id, "error": e.code }),
        );
    }
    resp
}
