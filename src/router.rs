use serde_json::{json, Value};
use worker::*;

use crate::avatar::{generate_avatar_seed, render_identicon_svg};
use crate::authorize;
use crate::config;
use crate::password::{hash_password, random_hex, random_id, verify_password};
use crate::util::*;

pub async fn handle(mut req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let url = req.url()?;
    let full_url = url.to_string();
    let path = {
        let p = url.path().trim_end_matches('/').to_string();
        if p.is_empty() {
            "/".into()
        } else {
            p
        }
    };
    let method = req.method();

    if method == Method::Options {
        return Ok(preflight());
    }

    let result = route(&mut req, &env, method, &path, &url, full_url).await;
    Ok(with_cors(match result {
        Ok(r) => r,
        Err(e) => json_err(e.0, e.1, &e.2),
    }))
}

fn preflight() -> Response {
    Response::empty()
        .unwrap()
        .with_status(204)
        .with_headers(preflight_headers())
}

fn preflight_headers() -> Headers {
    let mut h = Headers::new();
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

fn with_cors(mut res: Response) -> Response {
    res.headers_mut().set("access-control-allow-origin", "*").ok();
    res
}

async fn route(
    req: &mut Request,
    env: &Env,
    method: Method,
    path: &str,
    url: &Url,
    full_url: String,
) -> ApiResult<Response> {
    if method == Method::Get && path == "/health" {
        let mut body = json!({ "ok": true, "service": "cloudflare-auth" });
        if let Some(obj) = body.as_object_mut() {
            if let Some(cfg) = config::public_config(env).as_object() {
                for (k, v) in cfg {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        return ok_json(body);
    }

    if method == Method::Post && path == "/auth/register" {
        return handle_register(req, env).await;
    }
    if method == Method::Post && path == "/auth/login" {
        return handle_login(req, env).await;
    }
    if method == Method::Get && path == "/auth/me" {
        return handle_me(req, env).await;
    }
    if method == Method::Patch && path == "/auth/me" {
        return handle_update_me(req, env).await;
    }
    if method == Method::Post && path == "/auth/logout" {
        return handle_logout(req, env).await;
    }
    if method == Method::Post && path == "/auth/avatar/refresh" {
        return handle_avatar_refresh(req, env).await;
    }

    if method == Method::Get && path == "/authorize" {
        return Ok(authorize::inspect_authorize(env, query_authorize(url)).await);
    }
    if method == Method::Post && path == "/auth/authorize" {
        return handle_complete_authorize(req, env).await;
    }
    if method == Method::Post && path == "/auth/token" {
        return handle_token_exchange(req, env).await;
    }

    if method == Method::Get {
        if let Some(rest) = path.strip_prefix("/users/") {
            if let Some(uid) = rest.strip_suffix("/avatar") {
                return handle_avatar(env, uid, url).await;
            }
        }
    }

    if method == Method::Post && path == "/apps" {
        return handle_create_app(req, env).await;
    }
    if method == Method::Get && path == "/apps" {
        return handle_list_apps(req, env).await;
    }

    if path.starts_with("/apps/") {
        let rest = &path["/apps/".len()..];
        let (id, action) = match rest.split_once('/') {
            Some((id, a)) => (id, Some(a)),
            None => (rest, None),
        };
        let id = url_decode(id);
        if method == Method::Get && action.is_none() {
            return handle_get_app(req, env, &id).await;
        }
        if method == Method::Put && action.is_none() {
            return handle_update_app(req, env, &id).await;
        }
        if method == Method::Post && action == Some("rotate-secret") {
            return handle_rotate_secret(req, env, &id).await;
        }
        if (method == Method::Post || method == Method::Delete)
            && (action == Some("revoke") || action.is_none())
        {
            return handle_revoke_app(req, env, &id).await;
        }
    }

    serve_assets(env, &full_url).await
}

fn url_decode(s: &str) -> String {
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

fn query_authorize(url: &Url) -> authorize::AuthorizeQuery {
    let mut map = std::collections::HashMap::new();
    if let Some(q) = url.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(url_decode(k), url_decode(v));
            }
        }
    }
    authorize::AuthorizeQuery {
        client_id: map
            .get("client_id")
            .cloned()
            .or_else(|| map.get("app_id").cloned()),
        redirect_uri: map.get("redirect_uri").cloned(),
        state: map.get("state").cloned(),
        response_type: map.get("response_type").cloned(),
    }
}

async fn body_json<T: serde::de::DeserializeOwned>(req: &mut Request) -> ApiResult<T> {
    req.json::<T>()
        .await
        .map_err(|_| api_err(400, "invalid_json", "Body must be JSON"))
}

async fn app_ctx(req: &Request, env: &Env) -> ApiResult<Option<AppRow>> {
    let id = req.headers().get("x-app-id").ok().flatten();
    let secret = req.headers().get("x-app-secret").ok().flatten();
    resolve_app_credentials(env, id.as_deref(), secret.as_deref()).await
}

async fn require_auth(req: &Request, env: &Env) -> ApiResult<(UserRow, String, Option<String>)> {
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
    let (sid, _) = crate::password::verify_token(&config::jwt_secret(env), &token)
        .ok_or_else(|| api_err(401, "unauthorized", "Invalid or expired token"))?;

    #[derive(serde::Deserialize)]
    struct Sess {
        id: String,
        user_id: String,
        expires_at: String,
        app_id: Option<String>,
    }
    let session = db_first::<Sess>(env, "SELECT * FROM sessions WHERE id = ?", &[js_str(&sid)])
        .await?
        .ok_or_else(|| api_err(401, "unauthorized", "Session revoked"))?;
    if authorize::parse_iso_pub(&session.expires_at) < authorize::now_ts() {
        let _ = db_run(env, "DELETE FROM sessions WHERE id = ?", &[js_str(&session.id)]).await;
        return Err(api_err(401, "unauthorized", "Session expired"));
    }
    let user = load_user(env, &session.user_id).await?;
    Ok((user, session.id, session.app_id))
}

async fn handle_register(req: &mut Request, env: &Env) -> ApiResult<Response> {
    if !config::allow_registration(env) {
        return Err(api_err(
            403,
            "registration_disabled",
            "Registration is disabled",
        ));
    }
    let app = app_ctx(req, env).await?;
    if config::require_app_credentials(env) && app.is_none() {
        return Err(api_err(
            401,
            "invalid_app_credentials",
            "X-App-Id and X-App-Secret are required",
        ));
    }
    let body: Value = body_json(req).await?;
    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let username = body
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let password = body
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

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
    let (token, _) = authorize::create_session(env, &id, app_id.as_deref()).await?;
    ok_json(json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": config::token_ttl(env),
        "appId": app_id,
    }))
}

async fn handle_login(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let app = app_ctx(req, env).await?;
    if config::require_app_credentials(env) && app.is_none() {
        return Err(api_err(
            401,
            "invalid_app_credentials",
            "X-App-Id and X-App-Secret are required",
        ));
    }
    let body: Value = body_json(req).await?;
    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let password = body
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if email.is_empty() || password.is_empty() {
        return Err(api_err(
            400,
            "invalid_credentials",
            "Email and password are required",
        ));
    }
    let user = db_first::<UserRow>(env, "SELECT * FROM users WHERE email = ?", &[js_str(&email)])
        .await?;
    let dummy = "pbkdf2$100000$00000000000000000000000000000000$0000000000000000000000000000000000000000000000000000000000000000";
    let hash = user
        .as_ref()
        .map(|u| u.password_hash.clone())
        .unwrap_or_else(|| dummy.to_string());
    let ok = verify_password(&password, &hash);
    let Some(user) = user else {
        return Err(api_err(
            401,
            "invalid_credentials",
            "Invalid email or password",
        ));
    };
    if !ok {
        return Err(api_err(
            401,
            "invalid_credentials",
            "Invalid email or password",
        ));
    }
    let app_id = app.map(|a| a.app_id);
    let (token, _) = authorize::create_session(env, &user.id, app_id.as_deref()).await?;
    ok_json(json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": config::token_ttl(env),
        "appId": app_id,
    }))
}

async fn handle_me(req: &Request, env: &Env) -> ApiResult<Response> {
    let (user, _, appId) = require_auth(req, env).await?;
    ok_json(json!({ "user": public_user(&user), "appId": appId }))
}

async fn handle_update_me(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
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

async fn handle_logout(req: &Request, env: &Env) -> ApiResult<Response> {
    let (_, sid, _) = require_auth(req, env).await?;
    db_run(env, "DELETE FROM sessions WHERE id = ?", &[js_str(&sid)]).await?;
    ok_json(json!({ "ok": true }))
}

async fn handle_avatar_refresh(req: &Request, env: &Env) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let seed = generate_avatar_seed();
    db_run(
        env,
        "UPDATE users SET avatar_seed = ?1, updated_at = datetime('now') WHERE id = ?2",
        &[js_str(&seed), js_str(&user.id)],
    )
    .await?;
    let user = load_user(env, &user.id).await?;
    ok_json(json!({ "user": public_user(&user) }))
}

async fn handle_avatar(env: &Env, user_id: &str, url: &Url) -> ApiResult<Response> {
    let mut qseed = None;
    if let Some(q) = url.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if url_decode(k) == "v" {
                    qseed = Some(url_decode(v));
                }
            }
        }
    }
    #[derive(serde::Deserialize)]
    struct A {
        id: String,
        avatar_seed: Option<String>,
    }
    let row = db_first::<A>(
        env,
        "SELECT id, avatar_seed FROM users WHERE id = ?",
        &[js_str(user_id)],
    )
    .await?
    .ok_or_else(|| api_err(404, "not_found", "Not found"))?;
    let seed = row
        .avatar_seed
        .filter(|s| !s.is_empty())
        .or(qseed)
        .unwrap_or_else(|| row.id.clone());
    let svg = render_identicon_svg(&seed, 128);
    let mut headers = Headers::new();
    headers
        .set("content-type", "image/svg+xml; charset=utf-8")
        .ok();
    headers
        .set("cache-control", "public, max-age=604800")
        .ok();
    Response::from_body(ResponseBody::Body(svg.into_bytes().into()))
        .map(|r| r.with_headers(headers))
        .map_err(|_| api_err(500, "internal_error", "avatar response failed"))
}

async fn handle_create_app(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let body: Value = body_json(req).await?;
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let redirect_uris: Vec<String> = body
        .get("redirectUris")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let name_max = config::app_name_max(env);
    let desc_max = config::app_desc_max(env);
    if name.is_empty() || name.chars().count() > name_max {
        return Err(api_err(
            400,
            "invalid_name",
            format!("Name is required (max {name_max} chars)"),
        ));
    }
    if description.chars().count() > desc_max {
        return Err(api_err(
            400,
            "invalid_description",
            format!("Description max {desc_max} chars"),
        ));
    }
    let id = random_id();
    let app_id = format!("app_{}", random_hex(12));
    let secret = format!("sec_{}", random_hex(24));
    let secret_hash = hash_password(&secret, config::pbkdf2_iterations(env));
    let redirect_json = serialize_redirect_uris(&redirect_uris);
    db_run(
        env,
        "INSERT INTO apps (id, app_id, app_secret_hash, name, description, owner_id, status, redirect_uris) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
        &[
            js_str(&id),
            js_str(&app_id),
            js_str(&secret_hash),
            js_str(&name),
            js_str(&description),
            js_str(&user.id),
            js_str(&redirect_json),
        ],
    )
    .await?;
    let app = load_app(env, &id).await?;
    ok_json(json!({
        "app": public_app(&app),
        "appSecret": secret,
        "warning": "Store appSecret now. It will not be shown again.",
    }))
}

async fn handle_list_apps(req: &Request, env: &Env) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let rows = db_all_values(
        env,
        "SELECT id FROM apps WHERE owner_id = ? ORDER BY created_at DESC",
        &[js_str(&user.id)],
    )
    .await?;
    let mut apps = Vec::new();
    for row in rows {
        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
            if let Ok(app) = load_app(env, id).await {
                apps.push(public_app(&app));
            }
        }
    }
    ok_json(json!({ "apps": apps }))
}

async fn handle_get_app(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let app = load_app(env, id).await?;
    if app.owner_id != user.id {
        return Err(api_err(404, "not_found", "App not found"));
    }
    ok_json(json!({ "app": public_app(&app) }))
}

async fn handle_update_app(req: &mut Request, env: &Env, id: &str) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let mut app = load_app(env, id).await?;
    if app.owner_id != user.id {
        return Err(api_err(404, "not_found", "App not found"));
    }
    let body: Value = body_json(req).await?;
    let mut name = app.name.clone();
    let mut description = app.description.clone();
    let mut redirect_json = app.redirect_uris.clone();
    if let Some(v) = body.get("name").and_then(|v| v.as_str()) {
        name = v.trim().to_string();
        let name_max = config::app_name_max(env);
        if name.is_empty() || name.chars().count() > name_max {
            return Err(api_err(
                400,
                "invalid_name",
                format!("Name is required (max {name_max} chars)"),
            ));
        }
    }
    if let Some(v) = body.get("description").and_then(|v| v.as_str()) {
        description = v.trim().to_string();
    }
    if let Some(arr) = body.get("redirectUris").and_then(|v| v.as_array()) {
        let uris: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect();
        redirect_json = serialize_redirect_uris(&uris);
    }
    db_run(
        env,
        "UPDATE apps SET name = ?1, description = ?2, redirect_uris = ?3, updated_at = datetime('now') WHERE id = ?4",
        &[js_str(&name), js_str(&description), js_str(&redirect_json), js_str(id)],
    )
    .await?;
    app = load_app(env, id).await?;
    ok_json(json!({ "app": public_app(&app) }))
}

async fn handle_rotate_secret(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let app = load_app(env, id).await?;
    if app.owner_id != user.id {
        return Err(api_err(404, "not_found", "App not found"));
    }
    if app.status != "active" {
        return Err(api_err(400, "app_revoked", "App is revoked"));
    }
    let secret = format!("sec_{}", random_hex(24));
    let hash = hash_password(&secret, config::pbkdf2_iterations(env));
    db_run(
        env,
        "UPDATE apps SET app_secret_hash = ?1, secret_rotated_at = datetime('now'), updated_at = datetime('now') WHERE id = ?2",
        &[js_str(&hash), js_str(id)],
    )
    .await?;
    let app = load_app(env, id).await?;
    ok_json(json!({
        "app": public_app(&app),
        "appSecret": secret,
        "warning": "Previous secret is invalid. Store the new appSecret now.",
    }))
}

async fn handle_revoke_app(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let app = load_app(env, id).await?;
    if app.owner_id != user.id {
        return Err(api_err(404, "not_found", "App not found"));
    }
    if app.status != "revoked" {
        db_run(
            env,
            "UPDATE apps SET status = 'revoked', updated_at = datetime('now') WHERE id = ?",
            &[js_str(id)],
        )
        .await?;
    }
    let app = load_app(env, id).await?;
    ok_json(json!({ "app": public_app(&app) }))
}

async fn handle_complete_authorize(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let (user, _, _) = require_auth(req, env).await?;
    let body: Value = body_json(req).await?;
    let client_id = body
        .get("client_id")
        .or_else(|| body.get("clientId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let redirect_uri = body
        .get("redirect_uri")
        .or_else(|| body.get("redirectUri"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let state = body.get("state").and_then(|v| v.as_str()).unwrap_or("");
    let rt = body
        .get("response_type")
        .or_else(|| body.get("responseType"))
        .and_then(|v| v.as_str())
        .unwrap_or("code");
    authorize::complete_authorize(env, &user, client_id, redirect_uri, state, rt).await
}

async fn handle_token_exchange(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let body: Value = body_json(req).await?;
    let code = body.get("code").and_then(|v| v.as_str()).unwrap_or("");
    let client_id = body
        .get("client_id")
        .or_else(|| body.get("clientId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let client_secret = body
        .get("client_secret")
        .or_else(|| body.get("clientSecret"))
        .or_else(|| body.get("appSecret"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    authorize::exchange_code(env, code, client_id, client_secret).await
}

async fn serve_assets(env: &Env, url: &str) -> ApiResult<Response> {
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
    let mut headers = Headers::new();
    headers
        .set("content-type", "text/html; charset=utf-8")
        .ok();
    headers.set("cache-control", "no-store").ok();
    Response::from_body(ResponseBody::Body(replaced.into_bytes().into()))
        .map(|r| r.with_headers(headers))
        .map_err(|_| api_err(500, "internal_error", "Asset response failed"))
}
