import { randomHex, verifyPassword } from "./password";
import { createSession, toPublicUser } from "./token";
import { findAppByAppId, parseRedirectUris } from "./apps";
import type { AppRow, AuthCodeRow, Env, PublicUser, UserRow } from "./types";

/** 授权码有效期（秒） */
const CODE_TTL_SECONDS = 300;

export type AuthorizeError = Error & { status: number; code: string };

function authzError(status: number, code: string, message: string): AuthorizeError {
  return Object.assign(new Error(message), { status, code });
}

function serviceOrigin(env: Env, request: Request): string {
  // 优先配置，其次当前请求域名（便于自定义域）
  const configured = (env.SERVICE_URL ?? "").trim().replace(/\/+$/, "");
  if (configured) return configured;
  return new URL(request.url).origin;
}

export function buildAuthorizeUrl(
  env: Env,
  request: Request,
  params: { clientId: string; redirectUri: string; state?: string; responseType?: string }
): string {
  const origin = serviceOrigin(env, request);
  const url = new URL("/", origin);
  url.searchParams.set("client_id", params.clientId);
  url.searchParams.set("redirect_uri", params.redirectUri);
  url.searchParams.set("response_type", params.responseType || "code");
  if (params.state) url.searchParams.set("state", params.state);
  return url.toString();
}

function normalizeRedirectUri(uri: string): string {
  return (uri ?? "").trim();
}

function redirectUriAllowed(app: AppRow, redirectUri: string): boolean {
  const allowed = parseRedirectUris(app.redirect_uris);
  const target = normalizeRedirectUri(redirectUri);
  return allowed.some((u) => u === target);
}

function appendQuery(redirectUri: string, params: Record<string, string | undefined>): string {
  const url = new URL(redirectUri);
  for (const [k, v] of Object.entries(params)) {
    if (v != null && v !== "") url.searchParams.set(k, v);
  }
  return url.toString();
}

function appendFragment(redirectUri: string, params: Record<string, string | undefined>): string {
  const url = new URL(redirectUri);
  const parts: string[] = [];
  for (const [k, v] of Object.entries(params)) {
    if (v != null && v !== "") parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(v)}`);
  }
  const existing = url.hash.replace(/^#/, "");
  url.hash = existing ? `${existing}&${parts.join("&")}` : parts.join("&");
  return url.toString();
}

export async function validateAuthorizeRequest(
  env: Env,
  query: URLSearchParams
): Promise<{ app: AppRow; clientId: string; redirectUri: string; state: string; responseType: "code" | "token" }> {
  const clientId = (query.get("client_id") || query.get("app_id") || "").trim();
  const redirectUri = normalizeRedirectUri(query.get("redirect_uri") || "");
  const state = query.get("state") || "";
  const responseTypeRaw = (query.get("response_type") || "code").toLowerCase();

  if (!clientId) {
    throw authzError(400, "invalid_request", "client_id is required");
  }
  if (!redirectUri) {
    throw authzError(400, "invalid_request", "redirect_uri is required");
  }
  if (responseTypeRaw !== "code" && responseTypeRaw !== "token") {
    throw authzError(400, "unsupported_response_type", "response_type must be code or token");
  }

  const app = await findAppByAppId(env, clientId);
  if (!app || app.status !== "active") {
    throw authzError(400, "invalid_client", "Unknown or revoked client");
  }
  if (!redirectUriAllowed(app, redirectUri)) {
    throw authzError(400, "invalid_request", "redirect_uri is not registered for this app");
  }

  return {
    app,
    clientId,
    redirectUri,
    state,
    responseType: responseTypeRaw,
  };
}

/** GET /authorize — 校验参数，供登录页展示“继续前往 xxx” */
export async function inspectAuthorize(
  env: Env,
  request: Request
): Promise<Response> {
  const url = new URL(request.url);
  try {
    const v = await validateAuthorizeRequest(env, url.searchParams);
    return json({
      ok: true,
      app: {
        appId: v.app.app_id,
        name: v.app.name,
        description: v.app.description,
      },
      redirectUri: v.redirectUri,
      state: v.state,
      responseType: v.responseType,
      authorizeUrl: buildAuthorizeUrl(env, request, {
        clientId: v.clientId,
        redirectUri: v.redirectUri,
        state: v.state,
        responseType: v.responseType,
      }),
    });
  } catch (err) {
    const e = err as AuthorizeError;
    return json({ ok: false, error: e.code || "invalid_request", message: e.message }, e.status || 400);
  }
}

/**
 * POST /auth/authorize — 用户已登录后完成授权，返回业务方跳转 URL。
 * Bearer user token + { client_id, redirect_uri, state, response_type }
 */
export async function completeAuthorize(
  env: Env,
  user: UserRow,
  body: {
    clientId: string;
    redirectUri: string;
    state?: string;
    responseType?: string;
  }
): Promise<{ redirectTo: string; responseType: "code" | "token" }> {
  const clientId = (body.clientId || "").trim();
  const redirectUri = normalizeRedirectUri(body.redirectUri || "");
  const state = body.state || "";
  const responseTypeRaw = (body.responseType || "code").toLowerCase();
  const responseType = responseTypeRaw === "token" ? "token" : "code";

  const app = await findAppByAppId(env, clientId);
  if (!app || app.status !== "active") {
    throw authzError(400, "invalid_client", "Unknown or revoked client");
  }
  if (!redirectUriAllowed(app, redirectUri)) {
    throw authzError(400, "invalid_request", "redirect_uri is not registered for this app");
  }

  if (responseType === "code") {
    const code = `ac_${randomHex(24)}`;
    const expiresAt = new Date(Date.now() + CODE_TTL_SECONDS * 1000).toISOString();
    await env.DB.prepare(
      `INSERT INTO auth_codes (code, app_id, user_id, redirect_uri, expires_at, used)
       VALUES (?, ?, ?, ?, ?, 0)`
    )
      .bind(code, app.app_id, user.id, redirectUri, expiresAt)
      .run();

    return {
      redirectTo: appendQuery(redirectUri, { code, state }),
      responseType,
    };
  }

  // token 模式：直接为该用户在 App 下签发会话 token
  const token = await createSession(env, user.id, app.app_id);
  return {
    redirectTo: appendFragment(redirectUri, {
      access_token: token,
      token_type: "Bearer",
      expires_in: String(Number(env.TOKEN_TTL_SECONDS || 86400)),
      state,
    }),
    responseType,
  };
}

/** POST /auth/token — 后端用 code + App Secret 换取用户 JWT */
export async function exchangeAuthCode(
  env: Env,
  body: {
    code: string;
    clientId: string;
    clientSecret: string;
  }
): Promise<{ user: PublicUser; token: string; expiresIn: number; appId: string }> {
  const code = (body.code || "").trim();
  const clientId = (body.clientId || "").trim();
  const clientSecret = body.clientSecret || "";

  if (!code || !clientId || !clientSecret) {
    throw authzError(400, "invalid_request", "code, client_id and client_secret are required");
  }

  const row = await env.DB.prepare(`SELECT * FROM auth_codes WHERE code = ?`)
    .bind(code)
    .first<AuthCodeRow>();
  if (!row || row.used) {
    throw authzError(400, "invalid_grant", "Authorization code is invalid or already used");
  }
  if (row.expires_at < new Date().toISOString()) {
    throw authzError(400, "invalid_grant", "Authorization code expired");
  }
  if (row.app_id !== clientId) {
    throw authzError(400, "invalid_grant", "Authorization code was not issued to this client");
  }

  const app = await findAppByAppId(env, clientId);
  if (!app || app.status !== "active") {
    throw authzError(400, "invalid_client", "Unknown or revoked client");
  }

  const secretOk = await verifyPassword(clientSecret, app.app_secret_hash);
  if (!secretOk) {
    throw authzError(401, "invalid_client", "Invalid client secret");
  }

  const user = await env.DB.prepare(`SELECT * FROM users WHERE id = ?`)
    .bind(row.user_id)
    .first<UserRow>();
  if (!user) {
    throw authzError(400, "invalid_grant", "User no longer exists");
  }

  // 单次使用
  await env.DB.prepare(`UPDATE auth_codes SET used = 1 WHERE code = ? AND used = 0`)
    .bind(code)
    .run();

  const token = await createSession(env, user.id, app.app_id);
  const expiresIn = Number(env.TOKEN_TTL_SECONDS || 86400);

  return {
    user: toPublicUser(user),
    token,
    expiresIn,
    appId: app.app_id,
  };
}

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

export function isAuthorizeError(err: unknown): err is AuthorizeError {
  return (
    !!err &&
    typeof err === "object" &&
    "status" in err &&
    "code" in err &&
    typeof (err as AuthorizeError).status === "number"
  );
}
