use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use worker::*;

pub type ApiError = (u16, &'static str, String);
pub type ApiResult<T> = std::result::Result<T, ApiError>;

pub fn api_err(status: u16, code: &'static str, message: impl Into<String>) -> ApiError {
    (status, code, message.into())
}

pub fn json_ok(value: Value) -> std::result::Result<Response, worker::Error> {
    let mut res = Response::from_json(&value)?;
    if let Ok(h) = res.headers_mut().set("cache-control", "no-store") {
        let _ = h;
    }
    Ok(res)
}

pub fn json_err(status: u16, code: &str, message: &str) -> Response {
    let body = json!({ "error": code, "message": message });
    match Response::from_json(&body) {
        Ok(res) => res.with_status(status),
        Err(_) => Response::error("error", status).unwrap_or_else(|_| Response::ok("").unwrap()),
    }
}

pub fn ok_json(value: Value) -> ApiResult<Response> {
    json_ok(value).map_err(|_| api_err(500, "internal_error", "serialize failed"))
}

pub fn email_valid(email: &str) -> bool {
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !email.chars().any(|c| c.is_whitespace())
}

pub fn username_valid(name: &str, min: usize, max: usize) -> bool {
    let len = name.chars().count();
    len >= min
        && len <= max
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn parse_redirect_uris(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(20)
        .collect()
}

pub fn serialize_redirect_uris(list: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    for item in list {
        let uri = item.trim();
        if uri.is_empty() || uri.len() > 2048 || out.iter().any(|x| x == uri) {
            continue;
        }
        out.push(uri.to_string());
        if out.len() >= 20 {
            break;
        }
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

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
        "createdAt": row.created_at,
        "updatedAt": row.updated_at,
        "secretRotatedAt": row.secret_rotated_at,
    })
}

pub async fn db_first<T: serde::de::DeserializeOwned>(
    env: &Env,
    sql: &str,
    args: &[JsValue],
) -> ApiResult<Option<T>> {
    let db = env
        .d1("DB")
        .map_err(|_| api_err(500, "internal_error", "DB unavailable"))?;
    let stmt = db
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    stmt.first::<T>(None)
        .await
        .map_err(|_| api_err(500, "internal_error", "DB query failed"))
}

pub async fn db_run(env: &Env, sql: &str, args: &[JsValue]) -> ApiResult<()> {
    let db = env
        .d1("DB")
        .map_err(|_| api_err(500, "internal_error", "DB unavailable"))?;
    let stmt = db
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    stmt.run()
        .await
        .map_err(|_| api_err(500, "internal_error", "DB execution failed"))?;
    Ok(())
}

pub async fn db_all_values(env: &Env, sql: &str, args: &[JsValue]) -> ApiResult<Vec<Value>> {
    let db = env
        .d1("DB")
        .map_err(|_| api_err(500, "internal_error", "DB unavailable"))?;
    let stmt = db
        .prepare(sql)
        .bind(args)
        .map_err(|_| api_err(500, "internal_error", "DB bind failed"))?;
    let result = stmt
        .all()
        .await
        .map_err(|_| api_err(500, "internal_error", "DB query failed"))?;
    result
        .results::<Value>()
        .map_err(|_| api_err(500, "internal_error", "DB map failed"))
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

pub async fn resolve_app_credentials(
    env: &Env,
    app_id: Option<&str>,
    app_secret: Option<&str>,
) -> ApiResult<Option<AppRow>> {
    match (app_id, app_secret) {
        (None, None) => Ok(None),
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
            let app = db_first::<AppRow>(
                env,
                "SELECT * FROM apps WHERE app_id = ?",
                &[js_str(id)],
            )
            .await?
            .ok_or_else(|| {
                api_err(401, "invalid_app_credentials", "Invalid app credentials")
            })?;
            if app.status != "active"
                || !crate::password::verify_password(secret, &app.app_secret_hash)
            {
                return Err(api_err(
                    401,
                    "invalid_app_credentials",
                    "Invalid app credentials",
                ));
            }
            Ok(Some(app))
        }
        _ => Err(api_err(
            401,
            "invalid_app_credentials",
            "Invalid app credentials",
        )),
    }
}
