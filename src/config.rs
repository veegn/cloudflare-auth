use worker::Env;

const DEFAULT_APP_NAME: &str = "cloudflare-auth";
const DEFAULT_SERVICE_URL: &str = "https://auth.dayti.de";
const DEFAULT_TTL: i64 = 86400;
const PBKDF2_MIN: u32 = 100_000;

fn var(env: &Env, key: &str) -> Option<String> {
    env.var(key).ok().map(|v| v.to_string())
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw {
        None => fallback,
        Some(v) => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => true,
            "false" | "0" | "no" | "off" => false,
            _ => fallback,
        },
    }
}

fn parse_int(raw: Option<String>, fallback: i64, min: i64, max: i64) -> i64 {
    let n = raw
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(fallback);
    n.clamp(min, max)
}

pub fn app_name(env: &Env) -> String {
    var(env, "APP_NAME")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_string())
}

pub fn service_url(env: &Env) -> String {
    var(env, "SERVICE_URL")
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SERVICE_URL.to_string())
}

pub fn token_ttl(env: &Env) -> i64 {
    parse_int(var(env, "TOKEN_TTL_SECONDS"), DEFAULT_TTL, 1, i64::MAX)
}

pub fn password_min(env: &Env) -> usize {
    parse_int(var(env, "PASSWORD_MIN_LENGTH"), 8, 1, 128) as usize
}

pub fn username_bounds(env: &Env) -> (usize, usize) {
    let min = parse_int(var(env, "USERNAME_MIN"), 3, 1, 64) as usize;
    let max = parse_int(var(env, "USERNAME_MAX"), 32, min as i64, 64) as usize;
    (min, max.max(min))
}

pub fn app_name_max(env: &Env) -> usize {
    parse_int(var(env, "APP_NAME_MAX"), 64, 1, 128) as usize
}

pub fn app_desc_max(env: &Env) -> usize {
    parse_int(var(env, "APP_DESC_MAX"), 256, 0, 512) as usize
}

pub fn pbkdf2_iterations(env: &Env) -> u32 {
    parse_int(var(env, "PBKDF2_ITERATIONS"), PBKDF2_MIN as i64, PBKDF2_MIN as i64, 2_000_000)
        as u32
}

pub fn allow_registration(env: &Env) -> bool {
    parse_bool(var(env, "ALLOW_REGISTRATION"), true)
}

pub fn require_app_credentials(env: &Env) -> bool {
    parse_bool(var(env, "REQUIRE_APP_CREDENTIALS"), false)
}

pub fn jwt_secret(env: &Env) -> String {
    var(env, "JWT_SECRET").unwrap_or_else(|| "local-dev-secret-change-me-please-32bytes+".into())
}

pub fn public_config(env: &Env) -> serde_json::Value {
    let (umin, umax) = username_bounds(env);
    serde_json::json!({
        "appName": app_name(env),
        "serviceUrl": service_url(env),
        "tokenTtlSeconds": token_ttl(env),
        "passwordMinLength": password_min(env),
        "usernameMin": umin,
        "usernameMax": umax,
        "appNameMax": app_name_max(env),
        "appDescMax": app_desc_max(env),
        "pbkdf2Iterations": pbkdf2_iterations(env),
        "allowRegistration": allow_registration(env),
        "requireAppCredentials": require_app_credentials(env),
    })
}
