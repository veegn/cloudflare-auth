//! 会话签发与 Bearer 鉴权

use worker::*;

use crate::config;
use crate::db::{js_str, load_user, db_first, db_run, SessionRow, UserRow};
use crate::http::{api_err, ApiResult};
use crate::password::{hmac_sha256_b64url, sign_token, verify_token};
use crate::time::{format_unix_iso, now_ts, parse_iso_pub};

/// 鉴权上下文：用户 + 会话 id + 会话绑定的 App（如有）
pub struct AuthContext {
    pub user: UserRow,
    pub session_id: String,
    pub app_id: Option<String>,
}

/// 创建会话并返回 `(jwt, session_id)`
pub async fn create_session(
    env: &Env,
    user_id: &str,
    app_id: Option<&str>,
) -> ApiResult<(String, String)> {
    let ttl = config::token_ttl(env);
    let secret = config::jwt_secret(env);
    let sid = crate::password::uuid_v4();
    let token = sign_token(&secret, &sid, ttl);
    let token_hash = hmac_sha256_b64url(&secret, &token);
    let expires = format_unix_iso(now_ts() + ttl);
    db_run(
        env,
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, app_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            js_str(&sid),
            js_str(user_id),
            js_str(&token_hash),
            js_str(&expires),
            js_str(app_id.unwrap_or("")),
        ],
    )
    .await?;
    Ok((token, sid))
}

/// 校验 `Authorization: Bearer` + 会话未撤销/未过期
pub async fn require_auth(req: &Request, env: &Env) -> ApiResult<AuthContext> {
    let header = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .unwrap_or_default();
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| api_err(401, "unauthorized", "Missing Authorization: Bearer <token>"))?;
    let (sid, _) = verify_token(&config::jwt_secret(env), &token)
        .ok_or_else(|| api_err(401, "unauthorized", "Invalid or expired token"))?;

    let session = db_first::<SessionRow>(env, "SELECT * FROM sessions WHERE id = ?", &[js_str(&sid)])
        .await?
        .ok_or_else(|| api_err(401, "unauthorized", "Session revoked"))?;
    if parse_iso_pub(&session.expires_at) < now_ts() {
        let _ = db_run(env, "DELETE FROM sessions WHERE id = ?", &[js_str(&session.id)]).await;
        return Err(api_err(401, "unauthorized", "Session expired"));
    }
    let user = load_user(env, &session.user_id).await?;
    Ok(AuthContext {
        user,
        session_id: session.id,
        app_id: session.app_id,
    })
}
