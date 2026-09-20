//! 注册 / 登录 / 个人资料 / 头像

use serde_json::{json, Value};
use worker::*;

use crate::avatar::generate_avatar_seed;
use crate::config;
use crate::db::{js_str, load_user, db_first, db_run, public_user};
use crate::http::{
    api_err, body_json, body_raw, body_str, ok_json, ApiResult,
};
use crate::logging;
use crate::password::{hash_password, random_id, verify_password};
use crate::session::{create_session, require_auth};
use crate::validate::{
    email_valid, ensure_app_credentials_if_required, username_valid,
};

/// 登录失败时仍走一次 PBKDF2，降低时序侧信道
const DUMMY_HASH: &str = "pbkdf2$100000$00000000000000000000000000000000$0000000000000000000000000000000000000000000000000000000000000000";

pub async fn handle_register(req: &mut Request, env: &Env) -> ApiResult<Response> {
    if !config::allow_registration(env) {
        return Err(api_err(
            403,
            "registration_disabled",
            "Registration is disabled",
        ));
    }
    let app = ensure_app_credentials_if_required(req, env).await?;
    let body: Value = body_json(req).await?;
    let email = body_str(&body, &["email"]).to_lowercase();
    let username = body_str(&body, &["username"]);
    let password = body_raw(&body, &["password"]);

    if !email_valid(&email) {
        return Err(api_err(400, "invalid_email", "Email is invalid"));
    }
    let (umin, umax) = config::username_bounds(env);
    if !username_valid(&username, umin, umax) {
        return Err(api_err(
            400,
            "invalid_username",
            format!("Username must be {umin}-{umax} chars of letters, numbers, underscore"),
        ));
    }
    let pwmin = config::password_min(env);
    if password.chars().count() < pwmin {
        return Err(api_err(
            400,
            "weak_password",
            format!("Password must be at least {pwmin} characters"),
        ));
    }

    let exists = db_first::<Value>(
        env,
        "SELECT id FROM users WHERE email = ?1 OR username = ?2 LIMIT 1",
        &[js_str(&email), js_str(&username)],
    )
    .await?;
    if exists.is_some() {
        return Err(api_err(
            409,
            "conflict",
            "Email or username already taken",
        ));
    }

    let id = random_id();
    let password_hash = hash_password(&password, config::pbkdf2_iterations(env));
    let seed = generate_avatar_seed();
    db_run(
        env,
        "INSERT INTO users (id, email, username, password_hash, avatar_seed) VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            js_str(&id),
            js_str(&email),
            js_str(&username),
            js_str(&password_hash),
            js_str(&seed),
        ],
    )
    .await?;
    let user = load_user(env, &id).await?;
    let app_id = app.map(|a| a.app_id);
    let (token, _) = create_session(env, &id, app_id.as_deref()).await?;
    logging::event(
        "user_registered",
        serde_json::json!({
            "user_id": user.id,
            "username": user.username,
            "appId": app_id,
        }),
    );
    ok_json(json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": config::token_ttl(env),
        "appId": app_id,
    }))
}

pub async fn handle_login(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let app = ensure_app_credentials_if_required(req, env).await?;
    let body: Value = body_json(req).await?;
    let email = body_str(&body, &["email"]).to_lowercase();
    let password = body_raw(&body, &["password"]);
    if email.is_empty() || password.is_empty() {
        return Err(api_err(
            400,
            "invalid_credentials",
            "Email and password are required",
        ));
    }
    let user = db_first::<crate::db::UserRow>(env, "SELECT * FROM users WHERE email = ?", &[js_str(&email)])
        .await?;
    let hash = user
        .as_ref()
        .map(|u| u.password_hash.clone())
        .unwrap_or_else(|| DUMMY_HASH.to_string());
    let ok = verify_password(&password, &hash);
    let Some(user) = user else {
        return Err(api_err(
            401,
            "invalid_credentials",
            "Invalid email or password",
        ));
    };
    if !ok {
        logging::event("login_failed", serde_json::json!({ "reason": "invalid_credentials" }));
        return Err(api_err(
            401,
            "invalid_credentials",
            "Invalid email or password",
        ));
    }
    let app_id = app.map(|a| a.app_id);
    let (token, _) = create_session(env, &user.id, app_id.as_deref()).await?;
    logging::event(
        "user_login",
        serde_json::json!({
            "user_id": user.id,
            "appId": app_id,
        }),
    );
    ok_json(json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": config::token_ttl(env),
        "appId": app_id,
    }))
}

pub async fn handle_me(req: &Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    ok_json(json!({
        "user": public_user(&ctx.user),
        "appId": ctx.app_id
    }))
}

pub async fn handle_update_me(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let user = ctx.user;
    let body: Value = body_json(req).await?;
    let mut next_email = user.email.clone();
    let mut next_username = user.username.clone();
    let mut next_hash = user.password_hash.clone();
    let mut changed = false;

    if let Some(email) = body.get("email").and_then(|v| v.as_str()) {
        let email = email.trim().to_lowercase();
        if !email_valid(&email) {
            return Err(api_err(400, "invalid_email", "Email is invalid"));
        }
        if email != user.email {
            let taken = db_first::<Value>(
                env,
                "SELECT id FROM users WHERE email = ? AND id != ? LIMIT 1",
                &[js_str(&email), js_str(&user.id)],
            )
            .await?;
            if taken.is_some() {
                return Err(api_err(409, "conflict", "Email already taken"));
            }
            next_email = email;
            changed = true;
        }
    }
    if let Some(username) = body.get("username").and_then(|v| v.as_str()) {
        let username = username.trim().to_string();
        let (umin, umax) = config::username_bounds(env);
        if !username_valid(&username, umin, umax) {
            return Err(api_err(
                400,
                "invalid_username",
                format!("Username must be {umin}-{umax} chars of letters, numbers, underscore"),
            ));
        }
        if username != user.username {
            let taken = db_first::<Value>(
                env,
                "SELECT id FROM users WHERE username = ? AND id != ? LIMIT 1",
                &[js_str(&username), js_str(&user.id)],
            )
            .await?;
            if taken.is_some() {
                return Err(api_err(409, "conflict", "Username already taken"));
            }
            next_username = username;
            changed = true;
        }
    }
    if let Some(new_pw) = body.get("newPassword").and_then(|v| v.as_str()) {
        if !new_pw.is_empty() {
            let current = body
                .get("currentPassword")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !verify_password(current, &user.password_hash) {
                return Err(api_err(
                    401,
                    "invalid_credentials",
                    "Current password is incorrect",
                ));
            }
            let pwmin = config::password_min(env);
            if new_pw.chars().count() < pwmin {
                return Err(api_err(
                    400,
                    "weak_password",
                    format!("Password must be at least {pwmin} characters"),
                ));
            }
            next_hash = hash_password(new_pw, config::pbkdf2_iterations(env));
            changed = true;
        }
    }
    if !changed {
        return ok_json(json!({ "user": public_user(&user), "changed": false }));
    }
    db_run(
        env,
        "UPDATE users SET email = ?1, username = ?2, password_hash = ?3, updated_at = datetime('now') WHERE id = ?4",
        &[
            js_str(&next_email),
            js_str(&next_username),
            js_str(&next_hash),
            js_str(&user.id),
        ],
    )
    .await?;
    let updated = load_user(env, &user.id).await?;
    ok_json(json!({ "user": public_user(&updated), "changed": true }))
}

pub async fn handle_logout(req: &Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    db_run(
        env,
        "DELETE FROM sessions WHERE id = ?",
        &[js_str(&ctx.session_id)],
    )
    .await?;
    logging::event(
        "session_revoked",
        serde_json::json!({ "user_id": ctx.user.id, "session_id": ctx.session_id }),
    );
    ok_json(json!({ "ok": true }))
}
