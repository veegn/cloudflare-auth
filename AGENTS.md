# AGENTS.md

Rust auth service on **Cloudflare Workers + D1** (not TypeScript). Production: https://auth.dayti.de. Full API/product docs live in `README.md` — follow that when behavior is unclear.

## Commands

```powershell
npm run check          # cargo check --target wasm32-unknown-unknown  (fast verify)
npm run build          # worker-build --release → build/worker/shim.mjs
npm run dev            # build + wrangler dev
npm run deploy         # build + wrangler deploy
npm run db:schema:local
npm run db:schema:remote
```

- Prerequisites: Rust `wasm32-unknown-unknown`, `cargo install worker-build`, `npm install`, `npx wrangler login`.
- `wrangler.toml` `main` is **generated** (`build/worker/shim.mjs`). Never edit it; rebuild via `worker-build`.
- There is **no** test suite / lint / typecheck script beyond `npm run check`.
- Schema changes: `schema.sql` uses `CREATE TABLE IF NOT EXISTS`. New columns on **existing** D1 need explicit `ALTER TABLE` via `wrangler d1 execute --remote` (scripts do not migrate live DBs). Known migration: `migrations/001_apps_icon_url.sql` (`apps.icon_url`).

## Layout

| Path | Role |
|------|------|
| `src/lib.rs` | `#[event(fetch)]` → `router::handle` + module list |
| `src/router.rs` | Route table only (method+path → `handlers::*`) |
| `src/handlers/` | Domain handlers: `account`, `apps`, `authorize_flow`, `userinfo`, `assets` |
| `src/http.rs` | `ApiError`/`ApiResult`, JSON responses, CORS, body helpers |
| `src/db.rs` | D1 helpers, row types, `public_user`/`public_app` |
| `src/session.rs` | `create_session` + `require_auth` → `AuthContext` |
| `src/time.rs` | Unix ↔ ISO-8601 (use these, not SystemTime) |
| `src/validate.rs` | Field validation, app credentials, redirect_uris |
| `src/config.rs` | Parses `wrangler.toml` `[vars]` |
| `src/password.rs` | PBKDF2 + HS256 JWT + random ids |
| `src/authorize.rs` | OAuth-like inspect/complete/exchange (sessions live in `session.rs`) |
| `src/avatar.rs` | Identicon SVG |
| `public/index.html` | Management console shell (markup + CSS; placeholders injected by Worker) |
| `public/console.js` | Console behavior (IIFE, no bundler); loaded via `<script src="/console.js">` |
| `schema.sql` | D1: `users`, `sessions`, `apps`, `auth_codes` |

Worker binding: **`env.DB`** (D1), **`env.ASSETS`** (static). Secrets: `JWT_SECRET` via `wrangler secret put`.

## Worker / Workers quirks

- **Time in WASM**: use `worker::Date::now()`, **not** `std::time::SystemTime::now()` (panics under Workers). Signing/JWT/expiry already use Date.
- `run_worker_first = true`: every request hits Rust first; unmatched paths fall through to `serve_assets`, which injects `{{APP_NAME}}` / `{{SERVICE_URL}}` into HTML from `[vars]`.
- After changing `[vars]` or HTML placeholders, **redeploy** — they are not live-editable on disk for production.
- Custom domain `auth.dayti.de` is in `wrangler.toml` `routes`; `workers_dev = false`.
- `public/index.html` + `public/console.js` call APIs **same-origin** (`api('/auth/…')`). Do not hardcode a second host in the UI.
- Brand/config placeholders live only in **HTML** (`{{APP_NAME}}`, `{{SERVICE_URL}}`); `console.js` reads the brand from DOM (`.brand span`).
- When editing `public/index.html` or `console.js` on Windows, watch **CRLF** and UTF-8 (em-dash/ellipsis/CJK have broken JS strings before). Prefer exact `indexOf` patches or a UTF-8 Python script over loose regex.

## Auth / AppID contract (do not invent alternatives)

- End-user auth: `Authorization: Bearer <JWT>`; sessions revocable in `sessions` table.
- App integration headers: **`X-App-Id`** + **`X-App-Secret`** on register/login when used.
- App public id format: `app_<24 hex>`; routes that take `:id` for CRUD use the **internal UUID** (`apps.id`), not `app_xxx`, except **public icon**: `GET /v1/apps/:app_id/icon` (and `/apps/:app_id/icon`).
- App Secret / password hash format: `pbkdf2$iterations$saltHex$hashHex`. `PBKDF2_ITERATIONS` is clamped **min 100000**.
- Redirect login: `GET /authorize` (validate), `POST /auth/authorize` (Bearer + body → `redirectTo`), `POST /auth/token` (code + client secret). Auth codes: **5 minutes**, single-use. `redirect_uri` must match `apps.redirect_uris` exactly.
- Already signed-in + third-party redirect: **console requires user confirm** (Continue / switch / sign out) — do not auto-`/auth/authorize` on boot.
- Third-party profile: `GET /userinfo` (Bearer user token; optional app headers enforce token↔app match), `GET /v1/users/:id` (App credentials only).
- CORS: Worker sets `access-control-allow-origin: *` on API responses.

## Config knobs (`wrangler.toml` `[vars]`)

`APP_NAME`, `SERVICE_URL`, `TOKEN_TTL_SECONDS`, `ALLOW_REGISTRATION`, `PASSWORD_MIN_LENGTH`, `USERNAME_MIN/MAX`, `REQUIRE_APP_CREDENTIALS`, `APP_NAME_MAX`, `APP_DESC_MAX`, `PBKDF2_ITERATIONS`. Surface all of these on `GET /health` for verification after deploy.

## Operational gotchas

- Deploy = production for `auth.dayti.de`. Smoke-test `/health` + a register/login path after deploy.
- App Secret is returned **once** (create/rotate). Do not log or commit secrets; `.dev.vars` is gitignored.
- `PBKDF2_ITERATIONS` changes affect **new** hashes only; verification uses the iteration count stored in each hash.
