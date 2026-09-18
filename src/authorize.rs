use crate::password::{random_hex, sign_token, verify_password};
use crate::util::*;

const CODE_TTL_SECS: i64 = 300;

#[derive(serde::Deserialize)]
pub struct AuthorizeQuery {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub response_type: Option<String>,
}

fn normalize_uri(uri: &str) -> String {
    uri.trim().to_string()
}

fn parse_rt(raw: Option<&str>) -> ApiResult<String> {
    match raw.unwrap_or("code").to_ascii_lowercase().as_str() {
        "code" => Ok("code".into()),
        "token" => Ok("token".into()),
        _ => Err(api_err(
            400,
            "unsupported_response_type",
            "response_type must be code or token",
        )),
    }
}

pub async fn load_active_app(env: &worker::Env, client_id: &str) -> ApiResult<AppRow> {
    let app = db_first::<AppRow>(
        env,
        "SELECT * FROM apps WHERE app_id = ?",
        &[js_str(client_id)],
    )
    .await?
    .ok_or_else(|| api_err(400, "invalid_client", "Unknown or revoked client"))?;
    if app.status != "active" {
        return Err(api_err(400, "invalid_client", "Unknown or revoked client"));
    }
    Ok(app)
}

fn redirect_allowed(app: &AppRow, redirect_uri: &str) -> bool {
    parse_redirect_uris(Some(&app.redirect_uris))
        .iter()
        .any(|u| u == redirect_uri)
}

pub async fn inspect_authorize(
    env: &worker::Env,
    query: AuthorizeQuery,
) -> worker::Response {
    let client_id = query.client_id.unwrap_or_default().trim().to_string();
    let redirect_uri = normalize_uri(query.redirect_uri.as_deref().unwrap_or(""));
    if client_id.is_empty() {
        return crate::util::json_err(400, "invalid_request", "client_id is required");
    }
    if redirect_uri.is_empty() {
        return crate::util::json_err(400, "invalid_request", "redirect_uri is required");
    }
    let rt = match parse_rt(query.response_type.as_deref()) {
        Ok(v) => v,
        Err((s, c, m)) => return crate::util::json_err(s, c, &m),
    };
    let app = match load_active_app(env, &client_id).await {
        Ok(a) => a,
        Err((s, c, m)) => return crate::util::json_err(s, c, &m),
    };
    if !redirect_allowed(&app, &redirect_uri) {
        return crate::util::json_err(
            400,
            "invalid_request",
            "redirect_uri is not registered for this app",
        );
    }
    crate::util::json_ok(serde_json::json!({
        "ok": true,
        "app": {
            "appId": app.app_id,
            "name": app.name,
            "description": app.description,
        },
        "redirectUri": redirect_uri,
        "state": query.state.unwrap_or_default(),
        "responseType": rt,
    }))
    .unwrap_or_else(|_| crate::util::json_err(500, "internal_error", "serialize failed"))
}

pub async fn complete_authorize(
    env: &worker::Env,
    user: &UserRow,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    response_type: &str,
) -> ApiResult<worker::Response> {
    let rt = parse_rt(Some(response_type))?;
    let redirect_uri = normalize_uri(redirect_uri);
    if client_id.trim().is_empty() || redirect_uri.is_empty() {
        return Err(api_err(
            400,
            "invalid_request",
            "client_id and redirect_uri are required",
        ));
    }
    let app = load_active_app(env, client_id.trim()).await?;
    if !redirect_allowed(&app, &redirect_uri) {
        return Err(api_err(
            400,
            "invalid_request",
            "redirect_uri is not registered for this app",
        ));
    }

    if rt == "code" {
        let code = format!("ac_{}", random_hex(24));
        let expires_at = format_unix_iso(now_ts() + CODE_TTL_SECS);
        db_run(
            env,
            "INSERT INTO auth_codes (code, app_id, user_id, redirect_uri, expires_at, used) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            &[
                js_str(&code),
                js_str(&app.app_id),
                js_str(&user.id),
                js_str(&redirect_uri),
                js_str(&expires_at),
            ],
        )
        .await?;
        let mut url = format!("{redirect_uri}?code={code}");
        if !state.is_empty() {
            url.push_str(&format!("&state={state}"));
        }
        return ok_json(serde_json::json!({
            "redirectTo": url,
            "responseType": "code",
        }));
    }

    let (token, _) = create_session(env, &user.id, Some(&app.app_id)).await?;
    let ttl = crate::config::token_ttl(env);
    let mut frag = format!("access_token={token}&token_type=Bearer&expires_in={ttl}");
    if !state.is_empty() {
        frag.push_str(&format!("&state={state}"));
    }
    ok_json(serde_json::json!({
        "redirectTo": format!("{redirect_uri}#{frag}"),
        "responseType": "token",
    }))
}

pub async fn exchange_code(
    env: &worker::Env,
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> ApiResult<worker::Response> {
    let code = code.trim();
    let client_id = client_id.trim();
    if code.is_empty() || client_id.is_empty() || client_secret.is_empty() {
        return Err(api_err(
            400,
            "invalid_request",
            "code, client_id and client_secret are required",
        ));
    }

    #[derive(serde::Deserialize)]
    struct CodeRow {
        app_id: String,
        user_id: String,
        expires_at: String,
        used: i64,
    }

    let row = db_first::<CodeRow>(env, "SELECT * FROM auth_codes WHERE code = ?", &[js_str(code)])
        .await?
        .ok_or_else(|| {
            api_err(
                400,
                "invalid_grant",
                "Authorization code is invalid or already used",
            )
        })?;
    if row.used != 0 || row.app_id != client_id {
        return Err(api_err(
            400,
            "invalid_grant",
            "Authorization code is invalid or already used",
        ));
    }
    if parse_iso_pub(&row.expires_at) < now_ts() {
        return Err(api_err(400, "invalid_grant", "Authorization code expired"));
    }

    let app = load_active_app(env, client_id).await?;
    if !verify_password(client_secret, &app.app_secret_hash) {
        return Err(api_err(401, "invalid_client", "Invalid client secret"));
    }

    let user = load_user(env, &row.user_id).await?;
    db_run(
        env,
        "UPDATE auth_codes SET used = 1 WHERE code = ? AND used = 0",
        &[js_str(code)],
    )
    .await?;

    let (token, _) = create_session(env, &user.id, Some(&app.app_id)).await?;
    ok_json(serde_json::json!({
        "user": public_user(&user),
        "token": token,
        "expiresIn": crate::config::token_ttl(env),
        "appId": app.app_id,
    }))
}

pub async fn create_session(
    env: &worker::Env,
    user_id: &str,
    app_id: Option<&str>,
) -> ApiResult<(String, String)> {
    let ttl = crate::config::token_ttl(env);
    let sid = uuid_v4();
    let token = sign_token(&crate::config::jwt_secret(env), &sid, ttl);
    let token_hash = crate::password::hmac_sha256_b64url(&crate::config::jwt_secret(env), &token);
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

pub fn now_ts() -> i64 {
    (worker::Date::now().as_millis() / 1000) as i64
}

fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("random");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex = crate::password::bytes_to_hex(&b);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn uuid_v4_pub() -> String {
    uuid_v4()
}

pub fn format_unix_iso(ts: i64) -> String {
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub fn parse_iso_pub(s: &str) -> i64 {
    let s = s.trim().replace('T', " ").replace('Z', "");
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.len() < 2 {
        return 0;
    }
    let date: Vec<i64> = parts[0]
        .split('-')
        .filter_map(|x| x.parse().ok())
        .collect();
    let time: Vec<i64> = parts[1]
        .split(':')
        .filter_map(|x| x.parse().ok())
        .collect();
    if date.len() < 3 || time.len() < 2 {
        return 0;
    }
    let (h, mi, se) = (time[0], time[1], *time.get(2).unwrap_or(&0));
    days_from_civil(date[0], date[1] as u32, date[2] as u32) * 86400 + h * 3600 + mi * 60 + se
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
