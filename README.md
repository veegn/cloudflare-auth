# cloudflare-auth

Cloudflare Workers（**Rust**）+ D1 鉴权服务：注册 / 登录 / 会话、AppID 多应用接入、重定向登录、用户头像，以及 Google 风格管理台。

**生产**：https://auth.dayti.de

| 入口 | 说明 |
|------|------|
| `/` | 管理页（`public/`） |
| `/health` | 健康检查 + 配置快照 |
| `/auth/*` `/apps/*` `/authorize` | JSON API |

## 项目结构

```
cloudflare-auth/
├── src/                 # Rust Worker（workers-rs）
│   ├── lib.rs           # fetch 入口
│   ├── router.rs        # HTTP 路由
│   ├── config.rs        # 环境变量
│   ├── password.rs      # PBKDF2 + JWT
│   ├── authorize.rs     # 重定向登录
│   ├── avatar.rs        # identicon
│   └── util.rs          # D1 / 校验辅助
├── public/              # 管理台静态页
├── schema.sql           # D1 表结构
├── Cargo.toml           # Rust crate
├── wrangler.toml        # Worker / D1 / vars / 自定义域
└── package.json         # npm scripts（wrangler / worker-build）
```

## 依赖

- Rust + `wasm32-unknown-unknown`
- `cargo install worker-build`
- Node + wrangler（`npm install`）

```powershell
rustup target add wasm32-unknown-unknown
cargo install worker-build
npm install
```

## 常用命令

```powershell
npm run check          # cargo check (wasm)
npm run build          # worker-build --release
npm run deploy         # build + wrangler deploy
npm run db:schema:local
npm run db:schema:remote
```

## 配置（`wrangler.toml` `[vars]`）

| 变量 | 默认 | 说明 |
|------|------|------|
| `APP_NAME` | cloudflare-auth | 品牌名（注入 HTML） |
| `SERVICE_URL` | https://auth.dayti.de | 对外地址 |
| `TOKEN_TTL_SECONDS` | 86400 | 会话 TTL |
| `ALLOW_REGISTRATION` | true | 开放注册 |
| `PASSWORD_MIN_LENGTH` | 8 | 密码长度 |
| `USERNAME_MIN/MAX` | 3/32 | 用户名长度 |
| `REQUIRE_APP_CREDENTIALS` | false | 强制 App 凭证 |
| `PBKDF2_ITERATIONS` | 100000 | 哈希迭代（只升不降） |
| `JWT_SECRET` | （secret） | `wrangler secret put JWT_SECRET` |

## API 摘要

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/auth/register` | 注册（可选 X-App-Id/Secret） |
| POST | `/auth/login` | 登录 → JWT |
| GET/PATCH | `/auth/me` | 查询 / 修改账户 |
| GET | `/userinfo` | 第三方用户信息：id / email / 头像 URL |
| GET | `/v1/users/:id` | 按 userId 查询（需 App 凭证） |
| POST | `/auth/logout` | 注销会话 |
| POST | `/apps` / PUT `/apps/:id` | AppID 申请 / 修改 |
| POST | `/apps/:id/rotate-secret` \| `/revoke` | 轮换 / 吊销 |
| GET | `/authorize` | 校验 client_id + redirect_uri |
| POST | `/auth/authorize` | 登录后返回 redirectTo |
| POST | `/auth/token` | code + secret → 用户 JWT |
| GET | `/users/:id/avatar` | 头像 SVG |

## D1

`schema.sql`：`users` · `sessions` · `apps` · `auth_codes`

## 安全

- PBKDF2-SHA256 存密码与 App Secret
- 登录错误不区分「用户不存在」
- 会话与 App 可服务端撤销
- 授权码 5 分钟单次；redirect_uri 白名单精确匹配
- WASM 时间使用 `worker::Date`（勿用 `SystemTime::now`）
