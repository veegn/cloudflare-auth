# cloudflare-auth 产品设计

**视觉方向**：Quiet Utility（开发者工具感极简鉴权）  
**源视觉**：`design/option-quiet-utility.png`  
**后端能力**：Worker + D1，邮箱密码注册 / 登录、JWT 会话、账户信息查询、注销

---

## 1. 产品定位

| 项 | 内容 |
|----|------|
| 产品名 | cloudflare-auth |
| 一句话 | 跑在 Cloudflare Workers 上的轻量邮箱鉴权与账户存储 |
| 核心用户 | ① 接入方终端用户（登录/注册）② 集成开发者（查看账户与会话、复制 API Base） |
| 价值 | 零运维鉴权：PBKDF2 密码、可撤销会话、D1 结构化账户，开箱即用 |

**不做**：社交登录、邮箱验证、找回密码、多租户 RBAC、计费。这些留在后续迭代。

---

## 2. 信息架构

```
cloudflare-auth
├── /auth          未登录
│   ├── 登录
│   └── 注册
├── /docs          文档入口（两种身份）
│   ├── 开发者      API / AppID 接入说明
│   └── 用户        账户与会话说明
└── /account       已登录
    ├── Profile        账户字段 + 会话状态 + API Base
    ├── Applications   AppID 申请与管理
    └── Sign out       结束当前会话
```

侧栏（Quiet Utility）始终展示品牌 + Account / Docs；登录后出现 Applications 与 Sign out。登录/注册表单下方提供文档入口链接。Profile 可跳转 Applications。

---

## 3. 设计系统（Quiet Utility）

| Token | 值 | 用途 |
|-------|----|------|
| `--bg` | `#F5F6F8` | 页面底 |
| `--surface` | `#FFFFFF` | 主内容底（整页底面，非浮卡） |
| `--ink` | `#0A0A0A` | 主文案 |
| `--ink-2` | `#5C5C5C` | 次文案 |
| `--ink-3` | `#8A8A8A` | 辅助/占位 |
| `--line` | `#E4E4E7` | 1px 分隔线 |
| `--accent` | `#0070F3` | 主 CTA、Focus |
| `--accent-ink` | `#FFFFFF` | 主按钮文字 |
| `--ok` | `#12B76A` | 会话 Active 圆点 |
| `--danger` | `#D92D20` | 错误文案 |

**字体**：Inter（UI）+ JetBrains Mono / ui-monospace（ID、URL、日期）  
**字号**：H1 32–36 / 区块标题 18–20 / 正文 15 / 辅助 13  
**圆角**：输入与按钮 8–10px；状态胶囊全圆  
**密度**：内容列 max-width ≈ 720px，左对齐；侧栏 220px  

**原则**：整页底面优先，不用「大卡片套小卡片」；分隔用 1px 线；强调色只用于主操作与焦点。

---

## 4. 屏幕清单

| ID | 屏幕 | 主目标 | 主操作 |
|----|------|--------|--------|
| S1 | Login | 登录 | Sign in |
| S2 | Register | 创建账户 | Create account |
| S3 | Profile | 查看账户与会话，复制 API Base | Copy API Base URL |
| S4 | Docs home | 选择身份文档入口 | For developers / For users |
| S5 | Developer guide | API + AppID 接入说明 | 参考接口与示例 |
| S6 | User guide | 账户与会话说明 | 按步骤注册/登录/注销 |
| S7 | Applications | 申请/管理 AppID | Create application |
| S8 | （注销后） | 回到登录 | — |

**S3 字段**（与 D1 `users` / `sessions` 对齐）：

- Email、Username、Created、User ID  
- Session status：Active · Expires in 24h + 精确过期时间  
- API Base URL：可复制；说明 regenerate 会作废当前 token  

---

## 5. 文案与校验

| 场景 | 文案 |
|------|------|
| 登录失败 | Invalid email or password |
| 邮箱格式 | Email is invalid |
| 用户名 | 3–32 位字母数字下划线 |
| 密码 | 至少 8 位 |
| 冲突 | Email or username already taken |
| 未授权 | Session expired / revoked → 回 S1 |

---

## 6. 与 API 映射

| UI 动作 | API |
|---------|-----|
| 注册提交 | `POST /auth/register`（可选 `X-App-Id` / `X-App-Secret`） |
| 登录提交 | `POST /auth/login`（可选 `X-App-Id` / `X-App-Secret`） |
| 进入 Profile | `GET /auth/me` |
| Sign out | `POST /auth/logout` |
| Copy API Base | 前端读环境配置（如 `https://…/v1`） |
| 创建应用 | `POST /apps`（Bearer 用户 token） |
| 应用列表 | `GET /apps` |
| 轮换 Secret | `POST /apps/:id/rotate-secret` |
| 吊销应用 | `POST /apps/:id/revoke` |

Token 存 `localStorage`（原型）；生产建议 httpOnly Cookie 或安全存储。App Secret 仅创建/轮换时返回一次。

---

## 7. 非功能

- 错误不泄露用户是否存在（登录统一 401）  
- 主按钮 loading 禁用重复提交  
- Focus ring：`2px solid #0070F3`  
- 最小宽度：侧栏在 &lt;900px 折叠为顶栏品牌  

---

## 8. 交付物

| 文件 | 说明 |
|------|------|
| `design/option-quiet-utility.png` | 源视觉 |
| `design/PRODUCT.md` | 本文 |
| `design/INTERACTIONS.md` | 状态机与页面流转图 |
| `prototype/index.html` | 可交互原型（S1–S6） |
| `design-qa.md` | 视觉对照 QA |
