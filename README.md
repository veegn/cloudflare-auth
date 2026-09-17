# cloudflare-auth

基于 Cloudflare Workers + D1 的简单鉴权服务：注册、登录、会话（JWT + 可撤销 session）、获取当前用户，以及 **AppID 申请**（多应用接入）。

**生产地址**：https://auth.dayti.de

- 管理页（静态资源）：`/`
- API：`/auth/*`、`/apps/*`、`/health`

## 功能

- `POST /auth/register` — 注册（email / username / password）；可选 `X-App-Id` / `X-App-Secret`
- `POST /auth/login` — 登录，返回 JWT；可选 `X-App-Id` / `X-App-Secret`
- `GET /auth/me` — 查询当前用户（需 Bearer token）
- `POST /auth/logout` — 注销当前会话
- `GET /health` — 健康检查 + 当前生效配置快照
- `POST /apps` — 申请应用（返回 App ID + 一次性 App Secret）
- `GET /apps` — 列出我的应用
- `GET /apps/:id` — 应用详情
- `POST /apps/:id/rotate-secret` — 轮换 App Secret
- `POST /apps/:id/revoke`（或 `DELETE /apps/:id`）— 吊销应用

账户与应用信息存储在 D1：`users`、`sessions`、`apps`。

## 配置项（`wrangler.toml` `[vars]`）

| 变量 | 默认 | 说明 |
|------|------|------|
| `APP_NAME` | `cloudflare-auth` | 管理页品牌名 |
| `SERVICE_URL` | `https://auth.dayti.de` | 对外服务地址 |
| `TOKEN_TTL_SECONDS` | `86400` | 会话有效期（秒） |
| `ALLOW_REGISTRATION` | `true` | 是否开放注册 |
| `PASSWORD_MIN_LENGTH` | `8` | 密码最短长度 |
| `USERNAME_MIN` / `USERNAME_MAX` | `3` / `32` | 用户名长度（字符集为字母数字下划线） |
| `REQUIRE_APP_CREDENTIALS` | `false` | `true` 时 register/login 必须带 App 凭证 |
| `APP_NAME_MAX` / `APP_DESC_MAX` | `64` / `256` | 应用名/描述长度上限 |
| `PBKDF2_ITERATIONS` | `100000` | 密码哈希迭代次数（只升不降，低于 100000 会被钳制） |
| `JWT_SECRET` | — | 必须用 `wrangler secret put JWT_SECRET` 设置 |

`GET /health` 返回当前生效配置（不含密钥），便于核对。修改后执行 `npm run deploy`。

## 准备

1. 安装依赖：

```powershell
npm install
```

2. 登录 Cloudflare（首次）：

```powershell
npx wrangler login
```

3. 创建 D1 数据库并写入 schema：

```powershell
npm run db:create
# 将输出的 database_id 填入 wrangler.toml
npm run db:schema:local
npm run db:schema:remote
```

4. 设置 JWT 密钥：

```powershell
npx wrangler secret put JWT_SECRET
```

## 本地开发

```powershell
npm run dev
```

默认在 `http://127.0.0.1:8787`。

## 部署

```powershell
npm run deploy
```

自定义域名在 `wrangler.toml` 的 `routes` 中配置（当前 `auth.dayti.de`）。

## API 示例

### 注册

```bash
curl -X POST https://auth.dayti.de/auth/register \
  -H "content-type: application/json" \
  -d '{"email":"alice@example.com","username":"alice","password":"password123"}'
```

### 登录

```bash
curl -X POST https://auth.dayti.de/auth/login \
  -H "content-type: application/json" \
  -d '{"email":"alice@example.com","password":"password123"}'
```

### 当前用户

```bash
curl https://auth.dayti.de/auth/me \
  -H "Authorization: Bearer <token>"
```

### 注销

```bash
curl -X POST https://auth.dayti.de/auth/logout \
  -H "Authorization: Bearer <token>"
```

### 申请 AppID

```bash
curl -X POST https://auth.dayti.de/apps \
  -H "content-type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"name":"Acme Dashboard","description":"Prod web"}'
# 返回 appId 与 appSecret（secret 仅此一次）
```

### 使用 App 凭证登录（接入方）

```bash
curl -X POST https://auth.dayti.de/auth/login \
  -H "content-type: application/json" \
  -H "X-App-Id: app_xxx" \
  -H "X-App-Secret: sec_xxx" \
  -d '{"email":"alice@example.com","password":"password123"}'
```

### 轮换 / 吊销

```bash
curl -X POST https://auth.dayti.de/apps/<internal-id>/rotate-secret \
  -H "Authorization: Bearer <token>"

curl -X POST https://auth.dayti.de/apps/<internal-id>/revoke \
  -H "Authorization: Bearer <token>"
```

## AppID 接入流程

每个接入产品（建议按环境拆分：dev / staging / prod）使用独立 App ID，密钥可单独轮换或吊销，互不影响。

### 步骤

| 步骤 | 做什么 | 接口 / 入口 |
|------|--------|-------------|
| 1 | 开发者注册并登录管理页 | `POST /auth/register` 或 `/` |
| 2 | 申请应用，保存一次性 Secret | Applications → Create，或 `POST /apps` |
| 3 | 在**服务端**配置 `APP_ID` / `APP_SECRET` | 环境变量 / Secret Manager |
| 4 | 终端用户注册/登录时，后端附带 App Header | `POST /auth/register` \| `/auth/login` |
| 5 | 用返回的 user token 调业务接口 | `Authorization: Bearer <user_token>` |
| 6 | 密钥泄露 → 轮换；产品下线 → 吊销 | `/apps/:id/rotate-secret` \| `/revoke` |

### 注意

- **Secret 只放服务端**，不要写进前端/移动端包。
- 一个产品一个 App ID；多环境请分别申请。
- 不带 `X-App-*` 时仍可直接调用（第一方场景）；若设 `REQUIRE_APP_CREDENTIALS=true` 则强制携带。
- 管理页 Docs → For developers → **App ID integration guide** 含完整分步与排错表。

## 安全说明

- 密码使用 PBKDF2-SHA256（默认 100k 迭代，可配置且只升不降）存储，不存明文。
- 登录失败时对不存在用户也做一次哈希校验，降低时序侧信道。
- Token 为 HS256 JWT；服务端 `sessions` 表可随时撤销会话。
- App Secret 同样以 PBKDF2 存储，仅创建/轮换时返回一次；吊销后立即拒绝接入。
- 生产环境务必用 `wrangler secret put JWT_SECRET` 设置足够长的随机密钥。
- 正式上线前建议加上速率限制 / Turnstile / HTTPS only cookie 等增强措施。
