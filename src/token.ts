import { getTokenTtlSeconds } from "./config";
import { sha256Hex } from "./password";
import type { Env, PublicUser, SessionRow, UserRow } from "./types";

const encoder = new TextEncoder();

export function toPublicUser(row: UserRow): PublicUser {
  return {
    id: row.id,
    email: row.email,
    username: row.username,
    createdAt: row.created_at,
  };
}

/** 轻量 JWT（HS256），payload 仅放会话 id + 过期时间 */
export async function signToken(env: Env, sessionId: string, ttlSeconds: number): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const header = { alg: "HS256", typ: "JWT" };
  const payload = { sid: sessionId, iat: now, exp: now + ttlSeconds };

  const headerB64 = b64url(JSON.stringify(header));
  const payloadB64 = b64url(JSON.stringify(payload));
  const data = `${headerB64}.${payloadB64}`;
  const sig = await hmacSign(env.JWT_SECRET, data);
  return `${data}.${sig}`;
}

export async function verifyToken(
  env: Env,
  token: string
): Promise<{ sessionId: string; exp: number } | null> {
  const parts = token.split(".");
  if (parts.length !== 3) return null;
  const [headerB64, payloadB64, sig] = parts;
  const expected = await hmacSign(env.JWT_SECRET, `${headerB64}.${payloadB64}`);
  if (!(await timingSafeEqualStr(sig, expected))) return null;

  let payload: { sid?: string; exp?: number };
  try {
    payload = JSON.parse(atob(payloadB64.replace(/-/g, "+").replace(/_/g, "/")));
  } catch {
    return null;
  }
  if (!payload.sid || typeof payload.exp !== "number") return null;
  if (payload.exp < Math.floor(Date.now() / 1000)) return null;
  return { sessionId: payload.sid, exp: payload.exp };
}

async function hmacSign(secret: string, data: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const sig = await crypto.subtle.sign("HMAC", key, encoder.encode(data));
  return b64urlBytes(new Uint8Array(sig));
}

async function timingSafeEqualStr(a: string, b: string): Promise<boolean> {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
}

function b64url(input: string): string {
  return b64urlBytes(encoder.encode(input));
}

function b64urlBytes(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function getTokenTtl(env: Env): number {
  return getTokenTtlSeconds(env);
}

export async function createSession(env: Env, userId: string, appId?: string | null): Promise<string> {
  const ttl = getTokenTtl(env);
  const sessionId = crypto.randomUUID();
  const token = await signToken(env, sessionId, ttl);
  const tokenHash = await sha256Hex(token);
  const expiresAt = new Date(Date.now() + ttl * 1000).toISOString();

  await env.DB.prepare(
    `INSERT INTO sessions (id, user_id, token_hash, expires_at, app_id) VALUES (?, ?, ?, ?, ?)`
  )
    .bind(sessionId, userId, tokenHash, expiresAt, appId ?? null)
    .run();

  return token;
}

export async function revokeSession(env: Env, sessionId: string): Promise<void> {
  await env.DB.prepare(`DELETE FROM sessions WHERE id = ?`).bind(sessionId).run();
}

export async function findSession(env: Env, sessionId: string): Promise<SessionRow | null> {
  const row = await env.DB.prepare(`SELECT * FROM sessions WHERE id = ?`)
    .bind(sessionId)
    .first<SessionRow>();
  return row ?? null;
}

export async function cleanupExpiredSessions(env: Env): Promise<void> {
  await env.DB.prepare(`DELETE FROM sessions WHERE expires_at < datetime('now')`).run();
}
