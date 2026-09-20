//! 路由表：方法 + 路径 → handler
//!
//! 业务逻辑在 `handlers/*`；鉴权/配置/密码等见对应模块。

use worker::*;

use crate::config;
use crate::handlers;
use crate::http::{json_err, ok_json, preflight, url_decode, with_cors, ApiResult};
use crate::logging;

pub async fn handle(mut req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let started_ms = Date::now().as_millis();
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
    let method_label = logging::method_label(&method);

    if method == Method::Options {
        logging::http_request(method_label, &path, 204, Date::now().as_millis() - started_ms, None);
        return Ok(preflight());
    }

    let result = route(&mut req, &env, method, &path, &url, full_url).await;
    let duration_ms = Date::now().as_millis() - started_ms;
    let response = match result {
        Ok(r) => {
            let status = r.status_code();
            logging::http_request(method_label, &path, status, duration_ms, None);
            with_cors(r)
        }
        Err(e) => {
            logging::http_request(method_label, &path, e.status, duration_ms, Some(e.code));
            with_cors(json_err(e.status, e.code, &e.message))
        }
    };
    Ok(response)
}

async fn route(
    req: &mut Request,
    env: &Env,
    method: Method,
    path: &str,
    url: &Url,
    full_url: String,
) -> ApiResult<Response> {
    // ── 健康检查 ──────────────────────────────────────────
    if method == Method::Get && path == "/health" {
        return health(env);
    }

    // ── 账户 ──────────────────────────────────────────────
    if method == Method::Post && path == "/auth/register" {
        return handlers::account::handle_register(req, env).await;
    }
    if method == Method::Post && path == "/auth/login" {
        return handlers::account::handle_login(req, env).await;
    }
    if method == Method::Get && path == "/auth/me" {
        return handlers::account::handle_me(req, env).await;
    }
    if method == Method::Patch && path == "/auth/me" {
        return handlers::account::handle_update_me(req, env).await;
    }
    if method == Method::Post && path == "/auth/logout" {
        return handlers::account::handle_logout(req, env).await;
    }
    if method == Method::Post && path == "/auth/avatar/refresh" {
        return handlers::account::handle_avatar_refresh(req, env).await;
    }

    // ── 第三方用户信息 ────────────────────────────────────
    if method == Method::Get && (path == "/userinfo" || path == "/v1/userinfo") {
        return handlers::userinfo::handle_userinfo(req, env).await;
    }

    // ── 重定向授权 ────────────────────────────────────────
    if method == Method::Get && path == "/authorize" {
        return handlers::authorize_flow::handle_inspect_authorize(env, url).await;
    }
    if method == Method::Post && path == "/auth/authorize" {
        return handlers::authorize_flow::handle_complete_authorize(req, env).await;
    }
    if method == Method::Post && path == "/auth/token" {
        return handlers::authorize_flow::handle_token_exchange(req, env).await;
    }

    // ── 公开资源：头像 / 用户资料 / 应用图标 ──────────────
    if method == Method::Get {
        if let Some(rest) = path.strip_prefix("/users/") {
            if let Some(uid) = rest.strip_suffix("/avatar") {
                return handlers::account::handle_avatar(env, uid, url).await;
            }
            if let Some(uid) = rest.strip_suffix("/profile") {
                return handlers::userinfo::handle_user_profile_by_id(req, env, uid).await;
            }
        }
        if let Some(uid) = path.strip_prefix("/v1/users/") {
            return handlers::userinfo::handle_user_profile_by_id(req, env, uid).await;
        }
        if let Some(app_id) = path
            .strip_prefix("/v1/apps/")
            .and_then(|rest| rest.strip_suffix("/icon"))
        {
            return handlers::apps::handle_app_icon(env, app_id).await;
        }
    }

    // ── Applications ──────────────────────────────────────
    if method == Method::Post && path == "/apps" {
        return handlers::apps::handle_create_app(req, env).await;
    }
    if method == Method::Get && path == "/apps" {
        return handlers::apps::handle_list_apps(req, env).await;
    }
    if let Some(rest) = path.strip_prefix("/apps/") {
        let (id, action) = match rest.split_once('/') {
            Some((id, a)) => (id, Some(a)),
            None => (rest, None),
        };
        let id = url_decode(id);
        if method == Method::Get && action == Some("icon") {
            return handlers::apps::handle_app_icon(env, &id).await;
        }
        if method == Method::Get && action.is_none() {
            return handlers::apps::handle_get_app(req, env, &id).await;
        }
        if method == Method::Put && action.is_none() {
            return handlers::apps::handle_update_app(req, env, &id).await;
        }
        if method == Method::Post && action == Some("rotate-secret") {
            return handlers::apps::handle_rotate_secret(req, env, &id).await;
        }
        if (method == Method::Post || method == Method::Delete)
            && (action == Some("revoke") || action.is_none())
        {
            return handlers::apps::handle_revoke_app(req, env, &id).await;
        }
    }

    // ── 静态管理台 ────────────────────────────────────────
    handlers::assets::serve_assets(env, &full_url).await
}

fn health(env: &Env) -> ApiResult<Response> {
    let mut body = serde_json::json!({ "ok": true, "service": "cloudflare-auth" });
    if let Some(obj) = body.as_object_mut() {
        if let Some(cfg) = config::public_config(env).as_object() {
            for (k, v) in cfg {
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    ok_json(body)
}
