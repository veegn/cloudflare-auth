import type { Env } from "./types";

const DEFAULTS = {
  appName: "cloudflare-auth",
  serviceUrl: "https://auth.dayti.de",
  tokenTtlSeconds: 86400,
  passwordMinLength: 8,
  usernameMin: 3,
  usernameMax: 32,
  appNameMax: 64,
  appDescMax: 256,
  pbkdf2Iterations: 100_000,
  allowRegistration: true,
  requireAppCredentials: false,
} as const;

/** PBKDF2 迭代次数只允许不低于默认值，避免误配拉低安全性 */
const PBKDF2_MIN = 100_000;

function parseBool(raw: string | undefined, fallback: boolean): boolean {
  if (raw == null || raw === "") return fallback;
  const v = raw.trim().toLowerCase();
  if (v === "true" || v === "1" || v === "yes" || v === "on") return true;
  if (v === "false" || v === "0" || v === "no" || v === "off") return false;
  return fallback;
}

function parseIntClamp(
  raw: string | undefined,
  fallback: number,
  min: number,
  max = Number.MAX_SAFE_INTEGER
): number {
  const n = Number(raw);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, Math.floor(n)));
}

export function getAppName(env: Env): string {
  return (env.APP_NAME ?? "").trim() || DEFAULTS.appName;
}

export function getServiceUrl(env: Env): string {
  return (env.SERVICE_URL ?? "").trim().replace(/\/+$/, "") || DEFAULTS.serviceUrl;
}

export function getTokenTtlSeconds(env: Env): number {
  return parseIntClamp(env.TOKEN_TTL_SECONDS, DEFAULTS.tokenTtlSeconds, 1);
}

export function getPasswordMinLength(env: Env): number {
  return parseIntClamp(env.PASSWORD_MIN_LENGTH, DEFAULTS.passwordMinLength, 1, 128);
}

export function getUsernameBounds(env: Env): { min: number; max: number } {
  const min = parseIntClamp(env.USERNAME_MIN, DEFAULTS.usernameMin, 1, 64);
  let max = parseIntClamp(env.USERNAME_MAX, DEFAULTS.usernameMax, min, 64);
  if (max < min) max = min;
  return { min, max };
}

export function getAppNameMax(env: Env): number {
  return parseIntClamp(env.APP_NAME_MAX, DEFAULTS.appNameMax, 1, 128);
}

export function getAppDescMax(env: Env): number {
  return parseIntClamp(env.APP_DESC_MAX, DEFAULTS.appDescMax, 0, 512);
}

export function getPbkdf2Iterations(env: Env): number {
  return parseIntClamp(env.PBKDF2_ITERATIONS, DEFAULTS.pbkdf2Iterations, PBKDF2_MIN, 2_000_000);
}

export function isRegistrationAllowed(env: Env): boolean {
  return parseBool(env.ALLOW_REGISTRATION, DEFAULTS.allowRegistration);
}

export function isAppCredentialRequired(env: Env): boolean {
  return parseBool(env.REQUIRE_APP_CREDENTIALS, DEFAULTS.requireAppCredentials);
}

/** 供 /health 与运维核对的生效配置快照（不含密钥） */
export function getPublicConfig(env: Env) {
  const { min, max } = getUsernameBounds(env);
  return {
    appName: getAppName(env),
    serviceUrl: getServiceUrl(env),
    tokenTtlSeconds: getTokenTtlSeconds(env),
    passwordMinLength: getPasswordMinLength(env),
    usernameMin: min,
    usernameMax: max,
    appNameMax: getAppNameMax(env),
    appDescMax: getAppDescMax(env),
    pbkdf2Iterations: getPbkdf2Iterations(env),
    allowRegistration: isRegistrationAllowed(env),
    requireAppCredentials: isAppCredentialRequired(env),
  };
}
