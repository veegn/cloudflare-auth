# cloudflare-auth

Cloudflare Workers (**Rust / workers-rs**) + **D1** authentication service.

Provides: email/password registration & sign-in, JWT sessions, multi-app AppID integration, redirect login (with confirm), third-party userinfo, R2 avatar/app-icon upload (server-side crop + compress), Google-style management console.

**Production**: https://auth.dayti.de

| Entry | Description |
|------|-------------|
| `/` | Console (sign-in / account / Applications / Docs) |
| `/health` | Health check + active config snapshot |
| `/auth/*`, `/apps/*`, `/userinfo`, `/authorize` | JSON API |

**Languages / 语言**: UI and in-app docs support **English / 中文** (sidebar `EN | 中文`).  
Chinese project docs: [README.md](./README.md)

---

## Project layout

```
cloudflare-auth/
├── src/                      # Rust Worker
│   ├── lib.rs                # #[event(fetch)] entry + modules
│   ├── router.rs             # Route table
│   ├── handlers/             # Domain handlers
│   ├── http.rs / db.rs / session.rs / time.rs / validate.rs / logging.rs
│   ├── media.rs / media_r2.rs  # Image pipeline + R2
│   ├── password.rs / authorize.rs / config.rs / avatar.rs
├── public/
│   ├── index.html            # Console shell + CSS
│   ├── console.js            # Console behavior
│   └── i18n.js               # en / zh dictionaries
├── schema.sql
├── migrations/               # Explicit D1 ALTERs
├── wrangler.toml
└── package.json
```

---

## Stack

| Layer | Implementation |
|-------|----------------|
| Runtime | Cloudflare Workers |
| Server | Rust (workers-rs 0.8, `d1`) |
| Database | Cloudflare D1 (SQLite) |
| Object storage | Cloudflare R2 (`MEDIA` binding) |
| Password / App Secret | PBKDF2-SHA256 (default 100k) |
| Session | HS256 JWT + revocable `sessions` rows |
| Observability | Workers Logs (`[observability]`) |
| Frontend | Single-page console, same-origin API, i18n en/zh |

---

## Commands

```powershell
rustup target add wasm32-unknown-unknown
cargo install worker-build
npm install
npx wrangler login

npm run check          # cargo check --target wasm32-unknown-unknown
npm run build          # worker-build --release
npm run dev            # build + wrangler dev
npm run deploy         # build + wrangler deploy
npm run db:schema:local
npm run db:schema:remote
npx wrangler r2 bucket create cloudflare-auth-media
npx wrangler secret put JWT_SECRET
```

Migrations (existing D1 needs explicit `ALTER`):

```powershell
npx wrangler d1 execute cloudflare-auth-db --remote --file=./migrations/001_apps_icon_url.sql
npx wrangler d1 execute cloudflare-auth-db --remote --file=./migrations/002_media_r2.sql
```

---

## Configuration (`wrangler.toml` `[vars]`)

| Variable | Default | Description |
|----------|---------|-------------|
| `APP_NAME` | `cloudflare-auth` | Brand name injected into HTML |
| `SERVICE_URL` | `https://auth.dayti.de` | Public base URL |
| `TOKEN_TTL_SECONDS` | `86400` | Session TTL |
| `ALLOW_REGISTRATION` | `true` | Open registration |
| `PASSWORD_MIN_LENGTH` | `8` | Minimum password length |
| `USERNAME_MIN` / `USERNAME_MAX` | `3` / `32` | Username length |
| `REQUIRE_APP_CREDENTIALS` | `false` | Require App headers on register/login |
| `APP_NAME_MAX` / `APP_DESC_MAX` | `64` / `256` | App field limits |
| `PBKDF2_ITERATIONS` | `100000` | Hash iterations (clamped ≥ 100k) |

Secret: `JWT_SECRET` (required in production).

---

## Data model

| Table | Purpose |
|-------|---------|
| `users` | email, username, password_hash, avatar_seed, avatar_object_key |
| `sessions` | token_hash, expires_at, app_id (revocable) |
| `apps` | app_id, app_secret_hash, redirect_uris, icon_url, icon_object_key |
| `auth_codes` | one-time redirect-login codes (5 minutes) |

---

## API overview

Base: `https://auth.dayti.de` · JSON · `cache-control: no-store`

### Account

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/auth/register` | optional App headers | Register + return token |
| POST | `/auth/login` | optional App headers | Sign in |
| GET | `/auth/me` | Bearer | Current user |
| PATCH | `/auth/me` | Bearer | Update profile / password |
| POST | `/auth/logout` | Bearer | Revoke session |
| POST | `/auth/avatar` | Bearer | Upload avatar (raw image; server crop+compress → R2) |
| POST | `/auth/avatar/refresh` | Bearer | Reset to identicon |
| GET | `/users/:id/avatar` | public | Avatar (R2 or identicon SVG) |

### Applications

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/apps` | Bearer | Create app (returns `appSecret` once) |
| GET | `/apps` | Bearer | List owned apps |
| GET/PUT | `/apps/:id` | Bearer owner | Get / update |
| POST | `/apps/:id/rotate-secret` | Bearer owner | Rotate secret |
| POST | `/apps/:id/revoke` | Bearer owner | Revoke app |
| POST | `/apps/:id/icon` | Bearer owner | Upload icon (raw image; server crop+compress → R2) |
| GET | `/v1/apps/:app_id/icon` | public | Icon (R2 → external URL → identicon) |

### Redirect login & third-party

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/authorize` | public | Validate client + redirect_uri |
| POST | `/auth/authorize` | Bearer | Confirm; returns `redirectTo` |
| POST | `/auth/token` | App credentials | Exchange `code` for JWT |
| GET | `/userinfo` | Bearer (+ optional App headers) | User profile for third parties |
| GET | `/v1/users/:id` | App credentials | Profile by user id |

App headers: `X-App-Id` + `X-App-Secret` (`app_<24 hex>` public id).

### Media pipeline (R2)

Uploads are **raw image bodies**. The Worker **always** center-crops to a square, resizes to **256px**, and JPEG-compresses (~q82) before writing R2.

- Bucket / binding: `cloudflare-auth-media` / `MEDIA`
- Keys: `avatars/{user_id}.jpg`, `apps/{app_id}.jpg`

---

## Observability

Workers Logs enabled in `wrangler.toml`. Structured JSON events via `src/logging.rs` (never logs passwords/JWT/App Secrets).

Dashboard → Workers & Pages → **cloudflare-auth** → **Observability**.

---

## Console i18n

- Dictionaries: `public/i18n.js` (`en`, `zh`)
- Switcher: sidebar **EN | 中文** (persisted in `localStorage.cfa_lang`)
- `console.js` uses `I18N.t('key')` for toasts and dynamic labels
- HTML uses `data-i18n` / `data-i18n-placeholder` / `data-i18n-title`

---

## Operational notes

- Deploy = production for `auth.dayti.de`. Smoke `/health` + register/login after deploy.
- App Secret is returned **once** (create/rotate). Do not log or commit secrets.
- `PBKDF2_ITERATIONS` changes affect **new** hashes only.
- Workers time: use `worker::Date::now()`, not `std::time::SystemTime`.
- Cloudflare may return **1010** for non-browser UAs; browser access is normal.
