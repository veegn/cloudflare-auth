//! Applications CRUD / Secret 轮换 / 公开图标

use serde_json::{json, Value};
use worker::*;

use crate::avatar::render_identicon_svg;
use crate::config;
use crate::db::{
    js_str, load_owned_app, db_all, db_run, public_app, AppRow,
};
use crate::http::{api_err, body_json, body_str, ok_json, svg_response, ApiResult};
use crate::logging;
use crate::password::{hash_password, random_hex, random_id};
use crate::session::require_auth;
use crate::validate::{normalize_icon_url, serialize_redirect_uris};

pub async fn handle_create_app(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let body: Value = body_json(req).await?;
    let name = body_str(&body, &["name"]);
    let description = body_str(&body, &["description"]);
    let redirect_uris: Vec<String> = body
        .get("redirectUris")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let icon_url = normalize_icon_url(
        body.get("iconUrl")
            .or_else(|| body.get("icon_url"))
            .and_then(|v| v.as_str()),
    );

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
        "INSERT INTO apps (id, app_id, app_secret_hash, name, description, owner_id, status, redirect_uris, icon_url) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8)",
        &[
            js_str(&id),
            js_str(&app_id),
            js_str(&secret_hash),
            js_str(&name),
            js_str(&description),
            js_str(&ctx.user.id),
            js_str(&redirect_json),
            js_str(icon_url.as_deref().unwrap_or("")),
        ],
    )
    .await?;
    let app = load_owned_app(env, &ctx.user.id, &id).await?;
    logging::event(
        "app_created",
        serde_json::json!({ "app_id": app.app_id, "owner_id": app.owner_id, "name": app.name }),
    );
    ok_json(json!({
        "app": public_app(&app),
        "appSecret": secret,
        "warning": "Store appSecret now. It will not be shown again.",
    }))
}

pub async fn handle_list_apps(req: &Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let rows: Vec<AppRow> = db_all(
        env,
        "SELECT * FROM apps WHERE owner_id = ? ORDER BY created_at DESC",
        &[js_str(&ctx.user.id)],
    )
    .await?;
    let apps: Vec<Value> = rows.iter().map(public_app).collect();
    ok_json(json!({ "apps": apps }))
}

pub async fn handle_get_app(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    ok_json(json!({ "app": public_app(&app) }))
}

pub async fn handle_update_app(req: &mut Request, env: &Env, id: &str) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
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

    // 自定义图标：显式传 iconUrl 才更新（null 表示清除）
    let mut icon_url = app.icon_url.clone();
    if let Some(raw) = body.get("iconUrl") {
        if raw.is_null() {
            icon_url = None;
        } else if let Some(s) = raw.as_str() {
            icon_url = normalize_icon_url(Some(s));
        }
    }

    db_run(
        env,
        "UPDATE apps SET name = ?1, description = ?2, redirect_uris = ?3, icon_url = ?4, updated_at = datetime('now') WHERE id = ?5",
        &[
            js_str(&name),
            js_str(&description),
            js_str(&redirect_json),
            js_str(icon_url.as_deref().unwrap_or("")),
            js_str(id),
        ],
    )
    .await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    ok_json(json!({ "app": public_app(&app) }))
}

pub async fn handle_rotate_secret(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
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
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    logging::event(
        "app_secret_rotated",
        serde_json::json!({ "app_id": app.app_id, "owner_id": app.owner_id }),
    );
    ok_json(json!({
        "app": public_app(&app),
        "appSecret": secret,
        "warning": "Previous secret is invalid. Store the new appSecret now.",
    }))
}

pub async fn handle_revoke_app(req: &Request, env: &Env, id: &str) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    if app.status != "revoked" {
        db_run(
            env,
            "UPDATE apps SET status = 'revoked', updated_at = datetime('now') WHERE id = ?",
            &[js_str(id)],
        )
        .await?;
        logging::event(
            "app_revoked",
            serde_json::json!({ "app_id": app.app_id, "owner_id": app.owner_id }),
        );
    }
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    ok_json(json!({ "app": public_app(&app) }))
}

/// 公开应用图标：自定义 URL 则 302，否则按 appId 生成 identicon
pub async fn handle_app_icon(env: &Env, app_id: &str) -> ApiResult<Response> {
    let app_id = app_id.trim();
    let app = crate::db::db_first::<AppRow>(
        env,
        "SELECT * FROM apps WHERE app_id = ?",
        &[js_str(app_id)],
    )
    .await?
    .ok_or_else(|| api_err(404, "not_found", "App not found"))?;

    if let Some(custom) = normalize_icon_url(app.icon_url.as_deref()) {
        return Response::redirect_with_status(
            url::Url::parse(&custom).map_err(|_| api_err(400, "invalid_icon", "Bad icon URL"))?,
            302,
        )
        .map_err(|_| api_err(500, "internal_error", "redirect failed"));
    }

    svg_response(render_identicon_svg(&app.app_id, 128), 3600)
}
