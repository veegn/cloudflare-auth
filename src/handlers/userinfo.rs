//! 第三方用户信息：/userinfo、按 userId 查询

use serde_json::{json, Value};
use worker::*;

use crate::config;
use crate::db::{load_user, UserRow};
use crate::http::{api_err, ok_json, ApiResult};
use crate::session::require_auth;
use crate::validate::{require_app_credentials, resolve_app_credentials};

/// 第三方展示用：用户 ID / 邮箱 / 头像（绝对 URL）
/// 鉴权：Bearer 用户 token；若带 X-App-Id/Secret 则校验会话属于该 App。
pub async fn handle_userinfo(req: &Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;

    let app_id = req.headers().get("x-app-id").ok().flatten();
    let app_secret = req.headers().get("x-app-secret").ok().flatten();
    if app_id.is_some() || app_secret.is_some() {
        match resolve_app_credentials(env, app_id.as_deref(), app_secret.as_deref()).await? {
            Some(app) => {
                // 会话必须绑定到该 App（重定向登录 / 带凭证登录时写入）
                if ctx.app_id.as_deref() != Some(app.app_id.as_str()) {
                    return Err(api_err(
                        403,
                        "token_app_mismatch",
                        "Token was not issued for this application",
                    ));
                }
            }
            None => {
                return Err(api_err(
                    401,
                    "invalid_app_credentials",
                    "Invalid app credentials",
                ));
            }
        }
    }

    ok_json(userinfo_payload(env, &ctx.user))
}

/// 第三方服务端：App 凭证 + userId → 基础信息（无用户 token）
pub async fn handle_user_profile_by_id(
    req: &Request,
    env: &Env,
    user_id: &str,
) -> ApiResult<Response> {
    require_app_credentials(req, env).await?;

    let user_id = user_id.trim();
    if user_id.is_empty() {
        return Err(api_err(400, "invalid_request", "user id is required"));
    }
    let user = match load_user(env, user_id).await {
        Ok(u) => u,
        Err(_) => return Err(api_err(404, "not_found", "User not found")),
    };
    ok_json(userinfo_payload(env, &user))
}

fn userinfo_payload(env: &Env, user: &UserRow) -> Value {
    let base = config::service_url(env);
    let seed = user
        .avatar_seed
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| user.id.clone());
    let avatar_path = format!("/users/{}/avatar?v={}", user.id, seed);
    json!({
        "sub": user.id,
        "id": user.id,
        "email": user.email,
        "username": user.username,
        "avatarSeed": seed,
        "avatarUrl": format!("{base}{avatar_path}"),
        "avatarPath": avatar_path,
        "createdAt": user.created_at,
    })
}
