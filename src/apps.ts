import { getAppDescMax, getAppNameMax, getPbkdf2Iterations } from "./config";
import { hashPassword, randomHex, randomId, verifyPassword } from "./password";
import type { AppRow, Env, PublicApp } from "./types";

const APP_ID_RE = /^app_[a-f0-9]{24}$/;

export type AppError = Error & { status: number; code: string };

function appError(status: number, code: string, message: string): AppError {
  return Object.assign(new Error(message), { status, code });
}

export function toPublicApp(row: AppRow): PublicApp {
  return {
    id: row.id,
    appId: row.app_id,
    name: row.name,
    description: row.description,
    status: row.status,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    secretRotatedAt: row.secret_rotated_at,
  };
}

export function validateAppId(appId: string): boolean {
  return APP_ID_RE.test(appId);
}

export function generateAppId(): string {
  return `app_${randomHex(12)}`;
}

async function issueAppSecret(env: Env): Promise<{ secret: string; hash: string }> {
  const secret = `sec_${randomHex(24)}`;
  const hash = await hashPassword(secret, getPbkdf2Iterations(env));
  return { secret, hash };
}

async function loadAppById(env: Env, id: string): Promise<AppRow> {
  const row = await env.DB.prepare(`SELECT * FROM apps WHERE id = ?`)
    .bind(id)
    .first<AppRow>();
  if (!row) {
    throw appError(500, "internal_error", "App record missing after write");
  }
  return row;
}

export async function createApp(
  env: Env,
  ownerId: string,
  name: string,
  description: string
): Promise<{ app: AppRow; appSecret: string }> {
  const trimmedName = (name ?? "").trim();
  const trimmedDesc = (description ?? "").trim();
  const nameMax = getAppNameMax(env);
  const descMax = getAppDescMax(env);

  if (!trimmedName || trimmedName.length > nameMax) {
    throw appError(400, "invalid_name", `Name is required (max ${nameMax} chars)`);
  }
  if (trimmedDesc.length > descMax) {
    throw appError(400, "invalid_description", `Description max ${descMax} chars`);
  }

  const id = randomId();
  const appId = generateAppId();
  const { secret, hash } = await issueAppSecret(env);

  await env.DB.prepare(
    `INSERT INTO apps (id, app_id, app_secret_hash, name, description, owner_id, status)
     VALUES (?, ?, ?, ?, ?, ?, 'active')`
  )
    .bind(id, appId, hash, trimmedName, trimmedDesc, ownerId)
    .run();

  return { app: await loadAppById(env, id), appSecret: secret };
}

export async function listApps(env: Env, ownerId: string): Promise<AppRow[]> {
  const { results } = await env.DB.prepare(
    `SELECT * FROM apps WHERE owner_id = ? ORDER BY created_at DESC`
  )
    .bind(ownerId)
    .all<AppRow>();
  return results ?? [];
}

export async function getOwnedApp(env: Env, ownerId: string, id: string): Promise<AppRow | null> {
  const row = await env.DB.prepare(`SELECT * FROM apps WHERE id = ? AND owner_id = ?`)
    .bind(id, ownerId)
    .first<AppRow>();
  return row ?? null;
}

export async function rotateAppSecret(
  env: Env,
  ownerId: string,
  id: string
): Promise<{ app: AppRow; appSecret: string }> {
  const app = await requireOwnedActiveApp(env, ownerId, id);
  const { secret, hash } = await issueAppSecret(env);
  const now = new Date().toISOString();

  await env.DB.prepare(
    `UPDATE apps SET app_secret_hash = ?, secret_rotated_at = ?, updated_at = ? WHERE id = ?`
  )
    .bind(hash, now, now, app.id)
    .run();

  return { app: await loadAppById(env, app.id), appSecret: secret };
}

export async function revokeApp(env: Env, ownerId: string, id: string): Promise<AppRow> {
  const app = await getOwnedApp(env, ownerId, id);
  if (!app) {
    throw appError(404, "not_found", "App not found");
  }
  if (app.status === "revoked") return app;

  const now = new Date().toISOString();
  await env.DB.prepare(`UPDATE apps SET status = 'revoked', updated_at = ? WHERE id = ?`)
    .bind(now, id)
    .run();

  return loadAppById(env, id);
}

async function requireOwnedActiveApp(env: Env, ownerId: string, id: string): Promise<AppRow> {
  const app = await getOwnedApp(env, ownerId, id);
  if (!app) {
    throw appError(404, "not_found", "App not found");
  }
  if (app.status !== "active") {
    throw appError(400, "app_revoked", "App is revoked");
  }
  return app;
}

/**
 * 校验接入方 App 凭证（X-App-Id + X-App-Secret）。
 * 返回 null 表示未携带凭证；抛错表示凭证非法或不完整。
 */
export async function resolveAppCredentials(
  env: Env,
  appId: string | null,
  appSecret: string | null
): Promise<AppRow | null> {
  if (!appId && !appSecret) return null;

  if (!appId || !appSecret || !validateAppId(appId)) {
    throw appError(401, "invalid_app_credentials", "Invalid app credentials");
  }

  const app = await env.DB.prepare(`SELECT * FROM apps WHERE app_id = ?`)
    .bind(appId)
    .first<AppRow>();
  if (!app || app.status !== "active") {
    throw appError(401, "invalid_app_credentials", "Invalid app credentials");
  }

  const ok = await verifyPassword(appSecret, app.app_secret_hash);
  if (!ok) {
    throw appError(401, "invalid_app_credentials", "Invalid app credentials");
  }

  return app;
}
