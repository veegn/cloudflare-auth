import {
  getAppName,
  getPasswordMinLength,
  getPbkdf2Iterations,
  getPublicConfig,
  getServiceUrl,
  getUsernameBounds,
  isAppCredentialRequired,
  isRegistrationAllowed,
} from "./config";
import {
  createApp,
  getOwnedApp,
  listApps,
  resolveAppCredentials,
  revokeApp,
  rotateAppSecret,
  toPublicApp,
  type AppError,
} from "./apps";
import { hashPassword, randomId, verifyPassword } from "./password";
import {
  cleanupExpiredSessions,
  createSession,
  findSession,
  getTokenTtl,
  revokeSession,
  toPublicUser,
  verifyToken,
} from "./token";
import type { AppRow, CreateAppBody, Env, LoginBody, RegisterBody, UserRow } from "./types";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** 用户不存在时也跑一遍 PBKDF2，降低时序侧信道 */
const DUMMY_PASSWORD_HASH =
  "pbkdf2$100000$00000000000000000000000000000000$0000000000000000000000000000000000000000000000000000000000000000";

type AuthResult =
  | { user: UserRow; sessionId: string; appId: string | null }
  | { response: Response };

type AppContextResult = { app: AppRow | null } | { response: Response };

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname.replace(/\/+$/, "") || "/";

    try {
      return await route(request, env, ctx, path);
    } catch (err) {
      return toErrorResponse(err);
    }
  },
} satisfies ExportedHandler<Env>;

async function route(
  request: Request,
  env: Env,
  ctx: ExecutionContext,
  path: string
): Promise<Response> {
  const { method } = request;

  if (method === "GET" && path === "/health") {
    return json({
      ok: true,
      service: "cloudflare-auth",
      ...getPublicConfig(env),
    });
  }

  // —— Auth ——
  if (method === "POST" && path === "/auth/register") return handleRegister(request, env);
  if (method === "POST" && path === "/auth/login") return handleLogin(request, env, ctx);
  if (method === "GET" && path === "/auth/me") return handleMe(request, env);
  if (method === "POST" && path === "/auth/logout") return handleLogout(request, env);

  // —— Applications ——
  if (method === "POST" && path === "/apps") return handleCreateApp(request, env);
  if (method === "GET" && path === "/apps") return handleListApps(request, env);

  const appRoute = matchAppRoute(path);
  if (appRoute) {
    const { id, action } = appRoute;
    if (method === "GET" && !action) return handleGetApp(request, env, id);
    if (method === "POST" && action === "rotate-secret") {
      return handleRotateSecret(request, env, id);
    }
    if ((method === "POST" || method === "DELETE") && (action === "revoke" || !action)) {
      return handleRevokeApp(request, env, id);
    }
  }

  // 其余路径：静态资源（管理页），HTML 注入 APP_NAME / SERVICE_URL
  return serveAssets(request, env);
}

function matchAppRoute(path: string): { id: string; action?: string } | null {
  const m = /^\/apps\/([^/]+)(?:\/(rotate-secret|revoke))?$/.exec(path);
  if (!m) return null;
  return { id: decodeURIComponent(m[1]), action: m[2] };
}

// ---------------------------------------------------------------------------
// Static assets / branding
// ---------------------------------------------------------------------------

async function serveAssets(request: Request, env: Env): Promise<Response> {
  const res = await env.ASSETS.fetch(request);
  const contentType = res.headers.get("content-type") || "";
  if (!contentType.includes("text/html") || res.status >= 400) {
    return res;
  }

  const html = await res.text();
  const replaced = html
    .replaceAll("{{APP_NAME}}", getAppName(env))
    .replaceAll("{{SERVICE_URL}}", getServiceUrl(env))
    .replaceAll("https://cloudflare-auth.veegn.workers.dev", getServiceUrl(env));

  const headers = new Headers(res.headers);
  headers.set("cache-control", "no-store");
  return new Response(replaced, {
    status: res.status,
    statusText: res.statusText,
    headers,
  });
}

// ---------------------------------------------------------------------------
// Auth handlers
// ---------------------------------------------------------------------------

async function handleRegister(request: Request, env: Env): Promise<Response> {
  if (!isRegistrationAllowed(env)) {
    return json(
      { error: "registration_disabled", message: "Registration is disabled" },
      403
    );
  }

  const appCtx = await readAppContext(request, env);
  if ("response" in appCtx) return appCtx.response;

  const body = await readJson<RegisterBody>(request);
  if (body instanceof Response) return body;

  const email = (body.email ?? "").trim().toLowerCase();
  const username = (body.username ?? "").trim();
  const password = body.password ?? "";

  const validation = validateRegisterInput(env, email, username, password);
  if (validation) return validation;

  const existing = await env.DB.prepare(
    `SELECT id FROM users WHERE email = ? OR username = ? LIMIT 1`
  )
    .bind(email, username)
    .first<{ id: string }>();
  if (existing) {
    return json({ error: "conflict", message: "Email or username already taken" }, 409);
  }

  const id = randomId();
  const passwordHash = await hashPassword(password, getPbkdf2Iterations(env));

  await env.DB.prepare(
    `INSERT INTO users (id, email, username, password_hash) VALUES (?, ?, ?, ?)`
  )
    .bind(id, email, username, passwordHash)
    .run();

  const user = await env.DB.prepare(`SELECT * FROM users WHERE id = ?`)
    .bind(id)
    .first<UserRow>();
  if (!user) {
    return json({ error: "internal_error", message: "User create failed" }, 500);
  }

  const token = await createSession(env, id, appCtx.app?.app_id ?? null);
  return authSessionResponse(user, token, env, appCtx.app, 201);
}

function validateRegisterInput(
  env: Env,
  email: string,
  username: string,
  password: string
): Response | null {
  if (!EMAIL_RE.test(email)) {
    return json({ error: "invalid_email", message: "Email is invalid" }, 400);
  }

  const { min, max } = getUsernameBounds(env);
  const usernameRe = new RegExp(`^[A-Za-z0-9_]{${min},${max}}$`);
  if (!usernameRe.test(username)) {
    return json(
      {
        error: "invalid_username",
        message: `Username must be ${min}-${max} chars of letters, numbers, underscore`,
      },
      400
    );
  }

  const passwordMin = getPasswordMinLength(env);
  if (password.length < passwordMin) {
    return json(
      {
        error: "weak_password",
        message: `Password must be at least ${passwordMin} characters`,
      },
      400
    );
  }

  return null;
}

async function handleLogin(
  request: Request,
  env: Env,
  ctx: ExecutionContext
): Promise<Response> {
  const appCtx = await readAppContext(request, env);
  if ("response" in appCtx) return appCtx.response;

  const body = await readJson<LoginBody>(request);
  if (body instanceof Response) return body;

  const email = (body.email ?? "").trim().toLowerCase();
  const password = body.password ?? "";
  if (!email || !password) {
    return json(
      { error: "invalid_credentials", message: "Email and password are required" },
      400
    );
  }

  const user = await env.DB.prepare(`SELECT * FROM users WHERE email = ?`)
    .bind(email)
    .first<UserRow>();

  const ok = await verifyPassword(password, user?.password_hash ?? DUMMY_PASSWORD_HASH);
  if (!user || !ok) {
    return json(
      { error: "invalid_credentials", message: "Invalid email or password" },
      401
    );
  }

  const token = await createSession(env, user.id, appCtx.app?.app_id ?? null);
  ctx.waitUntil(cleanupExpiredSessions(env).catch(() => {}));
  return authSessionResponse(user, token, env, appCtx.app, 200);
}

function authSessionResponse(
  user: UserRow,
  token: string,
  env: Env,
  app: AppRow | null,
  status: number
): Response {
  return json(
    {
      user: toPublicUser(user),
      token,
      expiresIn: getTokenTtl(env),
      appId: app?.app_id ?? null,
    },
    status
  );
}

async function handleMe(request: Request, env: Env): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  return json({ user: toPublicUser(auth.user), appId: auth.appId }, 200);
}

async function handleLogout(request: Request, env: Env): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  await revokeSession(env, auth.sessionId);
  return json({ ok: true }, 200);
}

// ---------------------------------------------------------------------------
// Application handlers
// ---------------------------------------------------------------------------

async function handleCreateApp(request: Request, env: Env): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;

  const body = await readJson<CreateAppBody>(request);
  if (body instanceof Response) return body;

  try {
    const { app, appSecret } = await createApp(
      env,
      auth.user.id,
      body.name ?? "",
      body.description ?? ""
    );
    return json(
      {
        app: toPublicApp(app),
        appSecret,
        warning: "Store appSecret now. It will not be shown again.",
      },
      201
    );
  } catch (err) {
    return toErrorResponse(err);
  }
}

async function handleListApps(request: Request, env: Env): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  const rows = await listApps(env, auth.user.id);
  return json({ apps: rows.map(toPublicApp) }, 200);
}

async function handleGetApp(request: Request, env: Env, id: string): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  const app = await getOwnedApp(env, auth.user.id, id);
  if (!app) return json({ error: "not_found", message: "App not found" }, 404);
  return json({ app: toPublicApp(app) }, 200);
}

async function handleRotateSecret(request: Request, env: Env, id: string): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  try {
    const { app, appSecret } = await rotateAppSecret(env, auth.user.id, id);
    return json({
      app: toPublicApp(app),
      appSecret,
      warning: "Previous secret is invalid. Store the new appSecret now.",
    });
  } catch (err) {
    return toErrorResponse(err);
  }
}

async function handleRevokeApp(request: Request, env: Env, id: string): Promise<Response> {
  const auth = await requireAuth(request, env);
  if ("response" in auth) return auth.response;
  try {
    const app = await revokeApp(env, auth.user.id, id);
    return json({ app: toPublicApp(app) }, 200);
  } catch (err) {
    return toErrorResponse(err);
  }
}

// ---------------------------------------------------------------------------
// Shared guards / helpers
// ---------------------------------------------------------------------------

async function readAppContext(request: Request, env: Env): Promise<AppContextResult> {
  try {
    const app = await resolveAppCredentials(
      env,
      request.headers.get("X-App-Id"),
      request.headers.get("X-App-Secret")
    );

    if (!app && isAppCredentialRequired(env)) {
      return {
        response: json(
          {
            error: "invalid_app_credentials",
            message: "X-App-Id and X-App-Secret are required",
          },
          401
        ),
      };
    }

    return { app };
  } catch (err) {
    return { response: toErrorResponse(err) };
  }
}

async function requireAuth(request: Request, env: Env): Promise<AuthResult> {
  const header = request.headers.get("Authorization") ?? "";
  const match = /^Bearer\s+(.+)$/i.exec(header);
  if (!match) {
    return unauthorized("Missing Authorization: Bearer <token>");
  }

  const payload = await verifyToken(env, match[1]);
  if (!payload) {
    return unauthorized("Invalid or expired token");
  }

  const session = await findSession(env, payload.sessionId);
  if (!session) {
    return unauthorized("Session revoked");
  }

  if (session.expires_at < new Date().toISOString()) {
    await revokeSession(env, session.id);
    return unauthorized("Session expired");
  }

  const user = await env.DB.prepare(`SELECT * FROM users WHERE id = ?`)
    .bind(session.user_id)
    .first<UserRow>();
  if (!user) {
    return unauthorized("User not found");
  }

  return { user, sessionId: session.id, appId: session.app_id ?? null };
}

function unauthorized(message: string): AuthResult {
  return { response: json({ error: "unauthorized", message }, 401) };
}

async function readJson<T>(request: Request): Promise<T | Response> {
  try {
    return (await request.json()) as T;
  } catch {
    return json({ error: "invalid_json", message: "Body must be JSON" }, 400);
  }
}

function toErrorResponse(err: unknown): Response {
  if (isAppError(err)) {
    return json({ error: err.code, message: err.message }, err.status);
  }
  console.error("unhandled error", err);
  return json({ error: "internal_error", message: "Internal server error" }, 500);
}

function isAppError(err: unknown): err is AppError {
  return (
    !!err &&
    typeof err === "object" &&
    "status" in err &&
    "code" in err &&
    typeof (err as AppError).status === "number" &&
    typeof (err as AppError).code === "string"
  );
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
