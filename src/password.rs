//! PBKDF2 密码哈希、HS256 JWT、随机 ID

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn random_hex(byte_len: usize) -> String {
    let mut buf = vec![0u8; byte_len];
    getrandom::getrandom(&mut buf).expect("random");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// 32 hex 字符的业务主键
pub fn random_id() -> String {
    random_hex(16)
}

/// UUID v4（会话 id 等）
pub fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("random");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex = bytes_to_hex(&b);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// 存储格式：`pbkdf2$iterations$saltHex$hashHex`
pub fn hash_password(password: &str, iterations: u32) -> String {
    let salt_hex = random_hex(16);
    let salt = hex_to_bytes(&salt_hex);
    let mut out = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut out);
    format!("pbkdf2${iterations}${salt_hex}${}", bytes_to_hex(&out))
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2" {
        return false;
    }
    let Ok(iterations) = parts[1].parse::<u32>() else {
        return false;
    };
    if iterations == 0 {
        return false;
    }
    let salt = hex_to_bytes(parts[2]);
    let expected = parts[3];
    let mut out = vec![0u8; expected.len() / 2];
    if out.is_empty() || out.len() * 2 != expected.len() {
        return false;
    }
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut out);
    let actual = bytes_to_hex(&out);
    timing_safe_eq(actual.as_bytes(), expected.as_bytes())
}

pub fn timing_safe_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    if hex.len() % 2 != 0 {
        return Vec::new();
    }
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

pub fn hmac_sha256_b64url(secret: &str, data: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(data.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

pub fn b64url(input: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(input)
}

/// HS256 JWT，payload: `{ sid, iat, exp }`
pub fn sign_token(secret: &str, session_id: &str, ttl_seconds: i64) -> String {
    let now = (worker::Date::now().as_millis() / 1000) as i64;
    let header = br#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = format!(
        r#"{{"sid":"{session_id}","iat":{now},"exp":{}}}"#,
        now + ttl_seconds
    );
    let data = format!("{}.{}", b64url(header), b64url(payload.as_bytes()));
    let sig = hmac_sha256_b64url(secret, &data);
    format!("{data}.{sig}")
}

pub fn verify_token(secret: &str, token: &str) -> Option<(String, i64)> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let data = format!("{}.{}", parts[0], parts[1]);
    let expected = hmac_sha256_b64url(secret, &data);
    if !timing_safe_eq(parts[2].as_bytes(), expected.as_bytes()) {
        return None;
    }
    let payload_json = String::from_utf8(
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1].replace('-', "+").replace('_', "/"))
            .ok()?,
    )
    .ok()?;
    let v: serde_json::Value = serde_json::from_str(&payload_json).ok()?;
    let sid = v.get("sid")?.as_str()?.to_string();
    let exp = v.get("exp")?.as_i64()?;
    let now = (worker::Date::now().as_millis() / 1000) as i64;
    if exp < now {
        return None;
    }
    Some((sid, exp))
}
