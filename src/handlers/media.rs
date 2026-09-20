//! 头像 / 应用图标：R2 上传（强制裁剪压缩）与读取回退

use serde_json::json;
use worker::*;

use crate::avatar::render_identicon_svg;
use crate::db::{
    js_str, load_owned_app, load_user, db_first, db_run, public_app, public_user, AppRow,
    AvatarSeedRow,
};
use crate::http::{api_err, ok_json, svg_response, ApiResult};
use crate::logging;
use crate::media::{crop_resize_jpeg, APP_ICON_SIZE, AVATAR_SIZE};
use crate::media_r2::{app_icon_key, avatar_key, get_jpeg_response, put_jpeg};
use crate::session::require_auth;
use crate::validate::normalize_icon_url;

async fn read_upload_body(req: &mut Request) -> ApiResult<Vec<u8>> {
    req.bytes()
        .await
        .map_err(|_| api_err(400, "invalid_image", "Failed to read request body"))
}

/// POST /auth/avatar — Bearer，body=原始图片；服务端裁剪+压缩后写入 R2
pub async fn handle_upload_avatar(req: &mut Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let raw = read_upload_body(req).await?;
    let processed = crop_resize_jpeg(&raw, AVATAR_SIZE)?;
    let key = avatar_key(&ctx.user.id);
    put_jpeg(env, &key, processed).await?;
    // avatar_seed 变更以刷新前端缓存
    let seed = crate::avatar::generate_avatar_seed();
    db_run(
        env,
        "UPDATE users SET avatar_object_key = ?1, avatar_seed = ?2, updated_at = datetime('now') WHERE id = ?3",
        &[js_str(&key), js_str(&seed), js_str(&ctx.user.id)],
    )
    .await?;
    let user = load_user(env, &ctx.user.id).await?;
    logging::event(
        "avatar_uploaded",
        json!({ "user_id": user.id, "key": key }),
    );
    ok_json(json!({ "user": public_user(&user) }))
}

/// POST /auth/avatar — 恢复默认 identicon，删除 R2 自定义头像
pub async fn handle_avatar_refresh(req: &Request, env: &Env) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    if let Some(key) = ctx
        .user
        .avatar_object_key
        .clone()
        .filter(|s| !s.is_empty())
    {
        crate::media_r2::delete_quiet(env, &key).await;
    }
    let seed = crate::avatar::generate_avatar_seed();
    db_run(
        env,
        "UPDATE users SET avatar_object_key = NULL, avatar_seed = ?1, updated_at = datetime('now') WHERE id = ?2",
        &[js_str(&seed), js_str(&ctx.user.id)],
    )
    .await?;
    let user = load_user(env, &ctx.user.id).await?;
    logging::event("avatar_reset", json!({ "user_id": user.id }));
    ok_json(json!({ "user": public_user(&user) }))
}

/// GET /users/:id/avatar — R2 自定义图优先，否则 identicon
pub async fn handle_avatar(env: &Env, user_id: &str, url: &Url) -> ApiResult<Response> {
    let mut qseed = None;
    if let Some(q) = url.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if crate::http::url_decode(k) == "v" {
                    qseed = Some(crate::http::url_decode(v));
                }
            }
        }
    }
    let row = db_first::<AvatarSeedRow>(
        env,
        "SELECT id, avatar_seed, avatar_object_key FROM users WHERE id = ?",
        &[js_str(user_id)],
    )
    .await?
    .ok_or_else(|| api_err(404, "not_found", "Not found"))?;

    if let Some(key) = row.avatar_object_key.clone().filter(|s| !s.is_empty()) {
        if let Some(res) = get_jpeg_response(env, &key).await? {
            return Ok(res);
        }
    }

    let seed = row
        .avatar_seed
        .filter(|s| !s.is_empty())
        .or(qseed)
        .unwrap_or_else(|| row.id.clone());
    svg_response(render_identicon_svg(&seed, 128), 604800)
}

/// POST /apps/:id/icon — owner 上传应用图标（强制裁剪+压缩 → R2）
pub async fn handle_upload_app_icon(req: &mut Request, env: &Env, id: &str) -> ApiResult<Response> {
    let ctx = require_auth(req, env).await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    if app.status != "active" {
        return Err(api_err(400, "app_revoked", "App is revoked"));
    }
    let raw = read_upload_body(req).await?;
    let processed = crop_resize_jpeg(&raw, APP_ICON_SIZE)?;
    let key = app_icon_key(&app.app_id);
    put_jpeg(env, &key, processed).await?;
    db_run(
        env,
        "UPDATE apps SET icon_object_key = ?1, icon_url = NULL, updated_at = datetime('now') WHERE id = ?2",
        &[js_str(&key), js_str(&app.id)],
    )
    .await?;
    let app = load_owned_app(env, &ctx.user.id, id).await?;
    logging::event(
        "app_icon_uploaded",
        json!({ "app_id": app.app_id, "owner_id": app.owner_id, "key": key }),
    );
    ok_json(json!({ "app": public_app(&app) }))
}

/// GET /v1/apps/:app_id/icon 或 /apps/:id/icon
/// 优先级：R2 自定义图标 → 外部 icon_url 302 → identicon
pub async fn handle_app_icon(env: &Env, app_ref: &str, by_internal_id: bool) -> ApiResult<Response> {
    let app_ref = app_ref.trim();
    let app: AppRow = if by_internal_id {
        crate::db::load_app(env, app_ref).await?
    } else {
        db_first::<AppRow>(
            env,
            "SELECT * FROM apps WHERE app_id = ?",
            &[js_str(app_ref)],
        )
        .await?
        .ok_or_else(|| api_err(404, "not_found", "App not found"))?
    };

    if let Some(key) = app.icon_object_key.clone().filter(|s| !s.is_empty()) {
        if let Some(res) = get_jpeg_response(env, &key).await? {
            return Ok(res);
        }
    }

    if let Some(custom) = normalize_icon_url(app.icon_url.as_deref()) {
        return Response::redirect_with_status(
            url::Url::parse(&custom).map_err(|_| api_err(400, "invalid_icon", "Bad icon URL"))?,
            302,
        )
        .map_err(|_| api_err(500, "internal_error", "redirect failed"));
    }

    svg_response(render_identicon_svg(&app.app_id, 128), 3600)
}
