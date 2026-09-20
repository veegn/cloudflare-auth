//! D1 访问、行类型与对外投影

use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use worker::*;

use crate::http::{api_err, ApiResult};
use crate::validate::{normalize_icon_url, parse_redirect_uris};

pub fn js_str(s: &str) -> JsValue {
    JsValue::from_str(s)
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct AppRow {
    pub id: String,
    pub app_id: String,
    pub app_secret_hash: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub status: String,
    pub redirect_uris: String,
    pub icon_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub secret_rotated_at: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub avatar_seed: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct SessionRow {
    pub id: String,
    pub user_id: String,
    pub expires_at: String,
    pub app_id: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct AuthCodeRow {
    pub app_id: String,
    pub user_id: String,
    pub expires_at: String,
    pub used: i64,
}

#[derive(serde::Deserialize, Debug)]
pub struct AvatarSeedRow {
    pub id: String,
    pub avatar_seed: Option<String>,
}

pub fn public_user(row: &UserRow) -> Value {
    let seed = row.avatar_seed.clone().unwrap_or_else(|| row.id.clone());
    json!({
        "id": row.id,
        "email": row.email,
        "username": row.username,
        "createdAt": row.created_at,
        "avatarSeed": seed,
        "avatarUrl": format!("/users/{}/avatar?v={}", row.id, seed),
    })
}

pub fn public_app(row: &AppRow) -> Value {
    json!({
        "id": row.id,
        "appId": row.app_id,
        "name": row.name,
        "description": row.description,
        "status": row.status,
        "redirectUris": parse_redirect_uris(Some(&row.redirect_uris)),
        "iconUrl": normalize_icon_url(row.icon_url.as_deref()),
        "iconPath": format!("/v1/apps/{}/icon", row.app_id),
        "createdAt": row.created_at,
        "updatedAt": row.updated_at,
        "secretRotatedAt": row.secret_rotated_at,
    })
}

fn d1(env: &Env) -> Result<worker::d1::D1Database, crate::http::ApiError> {
    env.d1("DB")
        .map_err(|_| api_err(500, "internal_error", "DB unavailable"))
}

pub async fn db_first<T: serde::de::DeserializeOwned>(
    env: &Env,
    sql: &str,
    args: &[JsValue],
) -> ApiResult<Option<T>> {
    let stmt = d1(env)?
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    stmt.first::<T>(None)
        .await
        .map_err(|_| api_err(500, "internal_error", "DB query failed"))
}

pub async fn db_all<T: serde::de::DeserializeOwned>(
    env: &Env,
    sql: &str,
    args: &[JsValue],
) -> ApiResult<Vec<T>> {
    let stmt = d1(env)?
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    let result = stmt
        .all()
        .await
        .map_err(|_| api_err(500, "internal_error", "DB query failed"))?;
    result
        .results::<T>()
        .map_err(|_| api_err(500, "internal_error", "DB map failed"))
}

pub async fn db_run(env: &Env, sql: &str, args: &[JsValue]) -> ApiResult<()> {
    let stmt = d1(env)?
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    stmt.run()
        .await
        .map_err(|_| api_err(500, "internal_error", "DB execution failed"))?;
    Ok(())
}

pub async fn load_user(env: &Env, id: &str) -> ApiResult<UserRow> {
    db_first::<UserRow>(env, "SELECT * FROM users WHERE id = ?", &[js_str(id)])
        .await?
        .ok_or_else(|| api_err(404, "not_found", "User not found"))
}

pub async fn load_app(env: &Env, id: &str) -> ApiResult<AppRow> {
    db_first::<AppRow>(env, "SELECT * FROM apps WHERE id = ?", &[js_str(id)])
        .await?
        .ok_or_else(|| api_err(404, "not_found", "App not found"))
}

/// 加载应用并校验 owner；非 owner 一律 404（不泄露存在性）
pub async fn load_owned_app(env: &Env, owner_id: &str, app_id: &str) -> ApiResult<AppRow> {
    let app = load_app(env, app_id).await?;
    if app.owner_id != owner_id {
        return Err(api_err(404, "not_found", "App not found"));
    }
    Ok(app)
}
