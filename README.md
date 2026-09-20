# cloudflare-auth

Cloudflare Workers（**Rust / workers-rs**）+ **D1** 鉴权服务。

提供：邮箱密码注册/登录、JWT 会话、AppID 多应用接入、重定向登录（含二次确认）、第三方用户信息查询、R2 头像/应用图标上传（服务端强制裁剪压缩）、随机头像，以及 Google 风格管理台。

**生产地址**：https://auth.dayti.de

**文档语言 / Docs**: 中文见本页 · English: [README.en.md](./README.en.md)  
**界面语言 / UI**: 管理台侧栏 **EN | 中文** 可切换（`public/i18n.js`）。

| 入口 | 说明 |
|------|------|
| `/` | 管理台（登录 / 账户 / Applications / Docs） |
| `/health` | 健康检查 + 当前生效配置 |
| `/auth/*`、`/apps/*`、`/userinfo`、`/authorize` | JSON API |

---

## 项目结构

```
cloudflare-auth/
├── src/                      # Rust Worker
│   ├── lib.rs                # #[event(fetch)] 入口与模块声明
│   ├── router.rs             # 路由表（方法+路径 → handler）
│   ├── handlers/             # 领域 handler
│   │   ├── account.rs        # 注册/登录/me/头像
│   │   ├── apps.rs           # Applications CRUD / secret / icon
│   │   ├── authorize_flow.rs # 确认授权 / code 换 token
│   │   ├── userinfo.rs       # 第三方用户信息
│   │   └── assets.rs         # 静态资源 + HTML 占位符注入
│   ├── http.rs               # ApiError、JSON 响应、CORS、body 工具
│   ├── db.rs                 # D1 访问、行类型、public_* 投影
│   ├── session.rs            # 会话签发 + Bearer 鉴权（AuthContext）
│   ├── time.rs               # Unix ↔ ISO-8601（Worker 安全）
│   ├── validate.rs           # 业务校验、App 凭证、redirect_uri
│   ├── logging.rs            # Workers Observability 结构化日志
│   ├── media.rs              # 上传图强制裁剪+压缩（JPEG）
│   ├── media_r2.rs           # R2 MEDIA 绑定读写
│   ├── password.rs           # PBKDF2 + HS256 JWT + 随机 ID
│   ├── authorize.rs          # OAuth-like 授权流（inspect/complete/exchange）
│   ├── config.rs             # wrangler [vars] 解析
│   └── avatar.rs             # identicon SVG
├── public/                   # 管理台静态资源
│   ├── index.html            # 壳 + CSS；{{APP_NAME}} / {{SERVICE_URL}} 由 Worker 注入
│   ├── console.js            # 控制台逻辑（IIFE，无 bundler）
│   └── i18n.js               # 中英词典 + 语言切换
├── schema.sql                # D1 表结构
├── Cargo.toml                # Rust crate（根目录）
├── wrangler.toml             # Worker / assets / D1 / vars / 自定义域
└── package.json              # npm scripts
```

---

## 技术栈

| 层 | 实现 |
|----|------|
| 运行时 | Cloudflare Workers |
| 服务端 | Rust（workers-rs 0.8，`features = ["d1"]`） |
| 数据库 | Cloudflare D1（SQLite） |
| 密码 / App Secret | PBKDF2-SHA256（默认 100k 迭代） |
| 会话 | HS256 JWT + D1 `sessions` 可撤销 |
| 前端 | 单页管理台，同源调用 API |
| 对象存储 | Cloudflare R2（头像 / 应用图标） |

### 媒体上传（R2）

| 项 | 说明 |
|----|------|
| Bucket / 绑定 | `cloudflare-auth-media` / `MEDIA` |
| 头像 | `POST /auth/avatar`（Bearer，body=图片字节） |
| 应用图标 | `POST /apps/:id/icon`（Bearer owner，body=图片字节） |
| 服务端处理 | **强制**中心裁剪正方形 → 256px → JPEG q≈82 |
| 存储键 | `avatars/{user_id}.jpg`、`apps/{app_id}.jpg` |
| 读取 | R2 自定义图优先，否则外部 `iconUrl` 302 / identicon |
| 恢复默认头像 | `POST /auth/avatar/refresh`（删 R2 对象） |

```powershell
npx wrangler r2 bucket create cloudflare-auth-media
npx wrangler d1 execute cloudflare-auth-db --remote --file=./migrations/002_media_r2.sql
```

---

## 环境与命令

**依赖**

- Rust + `wasm32-unknown-unknown`
- `cargo install worker-build`
- Node.js + 项目 devDependencies（wrangler 等）

```powershell
rustup target add wasm32-unknown-unknown
cargo install worker-build
npm install
npx wrangler login
```

**数据库**

```powershell
npm run db:create            # 首次创建 D1，把 database_id 写入 wrangler.toml
npm run db:schema:local
npm run db:schema:remote
npx wrangler d1 execute cloudflare-auth-db --remote --file=./migrations/001_apps_icon_url.sql
npx wrangler secret put JWT_SECRET
```

**开发 / 部署**

```powershell
npm run check                # cargo check --target wasm32-unknown-unknown
npm run build                # worker-build --release
npm run dev                  # build + wrangler dev
npm run deploy               # build + wrangler deploy
```

构建产物：`build/worker/shim.mjs`（由 `worker-build` 生成，勿手改）。

---

## 配置（`wrangler.toml`）

### 绑定与路由

| 项 | 值 | 说明 |
|----|-----|------|
| `main` | `build/worker/shim.mjs` | Rust 构建入口 |
| `routes` | `auth.dayti.de` | 自定义域 |
| `[assets]` | `./public`，`binding = ASSETS` | 管理台静态资源 |
| `run_worker_first` | `true` | Worker 先执行，便于 HTML 注入 |
| `[[d1_databases]]` | `DB` | D1 绑定名 |

### `[vars]` 业务配置

| 变量 | 默认 | 说明 |
|------|------|------|
| `APP_NAME` | `cloudflare-auth` | 管理台品牌名（`{{APP_NAME}}` 注入） |
| `SERVICE_URL` | `https://auth.dayti.de` | 对外地址；`userinfo` 中头像绝对 URL 前缀 |
| `TOKEN_TTL_SECONDS` | `86400` | 会话有效期（秒） |
| `ALLOW_REGISTRATION` | `true` | 是否开放注册 |
| `PASSWORD_MIN_LENGTH` | `8` | 密码最短长度 |
| `USERNAME_MIN` / `USERNAME_MAX` | `3` / `32` | 用户名长度（`[A-Za-z0-9_]`） |
| `REQUIRE_APP_CREDENTIALS` | `false` | `true` 时 register/login 必须带 App 凭证 |
| `APP_NAME_MAX` / `APP_DESC_MAX` | `64` / `256` | 应用名/描述上限 |
| `PBKDF2_ITERATIONS` | `100000` | 哈希迭代（低于 10 万会被钳制） |

### Secret

| Secret | 说明 |
|--------|------|
| `JWT_SECRET` | JWT 签名密钥，生产必设 |

`GET /health` 返回当前生效配置快照（不含密钥）。

### Workers Observability

`wrangler.toml` 已开启 Workers Logs（`[observability] enabled = true`，全量采样）。  
请求摘要与安全相关业务事件以 JSON 对象写入 `console.log`（见 `src/logging.rs`）。  
**不会**记录密码、JWT、App Secret。

查看：Cloudflare Dashboard → Workers & Pages → **cloudflare-auth** → **Observability**。

```toml
[observability]
enabled = true
head_sampling_rate = 1   # 可按量调低，如 0.1

[observability.logs]
invocation_logs = true
```

---

## 数据模型（`schema.sql`）

| 表 | 用途 |
|----|------|
| `users` | 账户：email、username、password_hash、avatar_seed |
| `sessions` | 会话：token_hash、expires_at、app_id（可绑定应用） |
| `apps` | 应用：app_id、app_secret_hash、redirect_uris、status |
| `auth_codes` | 重定向登录一次性授权码 |

---

## API 总览

统一前缀：`https://auth.dayti.de`  
响应均为 JSON；`cache-control: no-store`。

### 账户

| 方法 | 路径 | 鉴权 | 说明 |
|------|------|------|------|
| POST | `/auth/register` | 可选 App Header | 注册并返回 token |
| POST | `/auth/login` | 可选 App Header | 登录，返回 JWT |
| GET | `/auth/me` | Bearer | 当前用户 |
| PATCH | `/auth/me` | Bearer | 修改 email / username / password |
| POST | `/auth/logout` | Bearer | 吊销当前会话 |
| POST | `/auth/avatar/refresh` | Bearer | 重新生成默认头像 |
| GET | `/users/:id/avatar` | 公开 | 头像 SVG（identicon） |

**注册 / 登录请求体**

```json
// register
{ "email": "alice@example.com", "username": "alice", "password": "password123" }

// login
{ "email": "alice@example.com", "password": "password123" }
```

**成功响应**

```json
{
  "user": {
    "id": "…",
    "email": "…",
    "username": "…",
    "createdAt": "…",
    "avatarSeed": "…",
    "avatarUrl": "/users/<id>/avatar?v=<seed>"
  },
  "token": "eyJ…",
  "expiresIn": 86400,
  "appId": null
}
```

可选请求头（接入方）：

```
X-App-Id: app_xxx
X-App-Secret: sec_xxx
```

带上后会话绑定该 App；凭证非法返回 `401 invalid_app_credentials`。

**PATCH /auth/me**

```json
{
  "email": "new@example.com",
  "username": "newname",
  "currentPassword": "old-pass",
  "newPassword": "new-pass-9"
}
```

- 改密码时必须提供 `currentPassword`
- 邮箱/用户名冲突：`409`

### AppID（多应用接入）

| 方法 | 路径 | 鉴权 | 说明 |
|------|------|------|------|
| POST | `/apps` | Bearer | 创建应用，返回 appId + 一次性 secret |
| GET | `/apps` | Bearer | 我的应用列表 |
| GET | `/apps/:id` | Bearer | 应用详情 |
| PUT | `/apps/:id` | Bearer | 修改 name / description / redirectUris |
| POST | `/apps/:id/rotate-secret` | Bearer | 轮换 Secret（旧的立即失效） |
| POST | `/apps/:id/revoke` | Bearer | 吊销应用 |
| DELETE | `/apps/:id` | Bearer | 同 revoke |

**创建请求体**

```json
{
  "name": "Acme Dashboard",
  "description": "Production web",
  "redirectUris": ["https://app.example.com/callback"],
  "iconUrl": "https://cdn.example.com/icon.png"
}
```

- `iconUrl`：可选，`http(s)` 图片地址；用于登录/二次确认页与应用列表展示
- 未设置时使用自动生成 identicon：`GET /v1/apps/:app_id/icon`（或 `/apps/:app_id/icon`）
- `PUT /apps/:id` 可更新 `iconUrl`；传 `null` 清除自定义图标
- 响应中的 `app.iconUrl` / `app.iconPath` 供前端直接使用

**创建响应（secret 仅此一次）**

```json
{
  "app": { "id": "…", "appId": "app_xxx", "name": "…", "status": "active", "redirectUris": ["…"] },
  "appSecret": "sec_xxx",
  "warning": "Store appSecret now. It will not be shown again."
}
```

`:id` 使用应用内部 `id`（UUID），不是 `app_xxx`。

### 重定向登录（接管第三方登录）

业务方把用户跳到 auth，登录/确认后再跳回自己的回调地址。

#### 1. 业务方发起

```
GET https://auth.dayti.de/?client_id=app_xxx
  &redirect_uri=https%3A%2F%2Fapp.example.com%2Fcallback
  &response_type=code
  &state=<random>
```

`redirect_uri` 必须在应用的 `redirectUris` 白名单中（精确匹配）。

#### 2. 管理台交互（按登录态）

| 用户状态 | 界面行为 |
|----------|----------|
| 未登录 | 登录/注册页 + 应用名/描述卡片；完成后跳转 |
| **已登录** | **二次确认页**：Continue as xxx / Use another account / Sign out |
| 本页刚完成登录 | 自动完成授权并跳转 |

二次确认（已登录）：

- **Continue as {username}** → `POST /auth/authorize`，跳转 `redirect_uri`
- **Use another account** → 清本地会话，回到登录（保留 client_id 等参数）
- **Sign out** → `POST /auth/logout` 后回登录

#### 3. 校验接口（可选）

```
GET /authorize?client_id=&redirect_uri=&response_type=&state=
```

```json
{
  "ok": true,
  "app": { "appId": "app_xxx", "name": "…", "description": "…" },
  "redirectUri": "https://…",
  "responseType": "code"
}
```

#### 4. 完成授权

```
POST /auth/authorize
Authorization: Bearer <user_token>
{
  "client_id": "app_xxx",
  "redirect_uri": "https://app.example.com/callback",
  "state": "…",
  "response_type": "code"
}
```

→ `{ "redirectTo": "https://app.example.com/callback?code=ac_…&state=…", "responseType": "code" }`

`response_type=token` 时 JWT 放在 URL fragment（适合无后端 SPA）。

#### 5. 后端换 token（code 模式）

```
POST /auth/token
{
  "code": "ac_…",
  "client_id": "app_xxx",
  "client_secret": "sec_xxx"
}
```

→ `{ "user": {…}, "token": "eyJ…", "expiresIn": 86400, "appId": "app_xxx" }`

- 授权码 **5 分钟**有效、**单次使用**
- code 与 client 不匹配 / 已用 / 过期：`400 invalid_grant`

### 第三方用户信息

用于业务侧展示：**用户 ID、邮箱、头像**。

| 方法 | 路径 | 鉴权 | 说明 |
|------|------|------|------|
| GET | `/userinfo` | Bearer user token | 当前登录用户资料 |
| GET | `/v1/userinfo` | 同上 | 别名 |
| GET | `/v1/users/:id` | X-App-Id + X-App-Secret | 按 userId 查询（服务端） |
| GET | `/users/:id/profile` | 同上 | 等价路径 |

**GET /userinfo**

```bash
curl https://auth.dayti.de/userinfo \
  -H "Authorization: Bearer <user_token>" \
  -H "X-App-Id: app_xxx" \
  -H "X-App-Secret: sec_xxx"
```

```json
{
  "sub": "user-id",
  "id": "user-id",
  "email": "alice@example.com",
  "username": "alice",
  "avatarSeed": "…",
  "avatarUrl": "https://auth.dayti.de/users/<id>/avatar?v=<seed>",
  "avatarPath": "/users/<id>/avatar?v=<seed>",
  "createdAt": "…"
}
```

- 仅需 user token 即可
- 若附带 App 凭证，则校验 token 必须由该 App 签发，否则 `403 token_app_mismatch`
- `avatarUrl` 可直接用于 `<img src>`

**按 userId 查询（服务端）**

```bash
curl https://auth.dayti.de/v1/users/<userId> \
  -H "X-App-Id: app_xxx" \
  -H "X-App-Secret: sec_xxx"
```

无 App 凭证：`401 invalid_app_credentials`。

### 健康检查

```
GET /health
```

返回 `ok`、`appName`、`serviceUrl` 及全部 `[vars]` 生效值。

---

## 管理台功能（`/`）

| 模块 | 功能 |
|------|------|
| Account | 登录 / 注册 / Profile（含 Edit profile、头像刷新） |
| Applications | 申请 AppID、查看/编辑、轮换 Secret、吊销、Redirect URIs |
| Docs | 开发者（API / AppID / 重定向登录）与用户说明 |
| 重定向登录 | 第三方应用信息卡片；已登录二次确认 |

品牌名与 API Base 由 `APP_NAME`、`SERVICE_URL` 注入 HTML。

---

## 接入方推荐流程

```text
1) 开发者在管理台注册账号
2) Applications → Create application
   填写 name、redirectUris，保存 App ID + App Secret（服务端）
3a) 简单 API 接入：后端用 X-App-Id/Secret 调 register/login
3b) 托管登录：业务页 302 到 auth 带 client_id/redirect_uri
    用户登录或确认 → 跳回 callback?code=
    后端 POST /auth/token 换 user JWT
4) 业务接口用 Bearer user token
5) 展示资料：GET /userinfo 或 GET /v1/users/:id
```

---

## 错误码（常见）

| HTTP | error | 场景 |
|------|-------|------|
| 400 | `invalid_json` / `invalid_email` / `invalid_username` / `weak_password` | 参数校验 |
| 400 | `invalid_request` / `invalid_client` / `unsupported_response_type` | authorize 参数/应用 |
| 400 | `invalid_grant` | 授权码无效/过期/已用 |
| 401 | `invalid_credentials` | 登录失败或改密当前密码错误 |
| 401 | `unauthorized` | 缺/错/过期/已撤销 token |
| 401 | `invalid_app_credentials` | App 凭证缺失或错误 |
| 403 | `token_app_mismatch` | userinfo 时 token 非该 App 签发 |
| 403 | `registration_disabled` | 已关闭注册 |
| 404 | `not_found` | 资源不存在 |
| 409 | `conflict` | 邮箱/用户名占用 |

---

## 安全说明

- 密码与 App Secret 使用 PBKDF2-SHA256，不存明文
- 登录失败不区分「用户不存在」
- 会话与应用均可服务端吊销；Secret 轮换后旧值立即失效
- 授权码 5 分钟单次；`redirect_uri` 白名单精确匹配
- 已登录 + 第三方跳转必须二次确认，降低误授权风险
- App Secret 仅放在接入方服务端，勿写入前端/移动端包
- WASM 中时间使用 `worker::Date`，不要使用 `std::time::SystemTime::now`
- 生产必须设置足够长的 `JWT_SECRET`
- 建议后续增加：速率限制、Turnstile、httpOnly Cookie 等

---

## 运维备忘

| 事项 | 说明 |
|------|------|
| 自定义域 | `wrangler.toml` → `routes`，zone 需在同一 CF 账号 |
| 静态资源 | `public/`；`index.html` 中 `{{APP_NAME}}`、`{{SERVICE_URL}}` 由 Worker 替换；行为脚本在 `public/console.js` |
| 重新部署 | `npm run deploy` |
| 核对配置 | `curl https://auth.dayti.de/health` |
| D1 迁移 | 新表/新列用 `schema.sql` 或 `wrangler d1 execute --remote` |

---

## License

私有项目，未公开授权时请勿外传。
