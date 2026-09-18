export interface Env {
  DB: D1Database;
  JWT_SECRET: string;
  /** 会话 TTL（秒），默认 86400 */
  TOKEN_TTL_SECONDS?: string;
  /** 系统展示名称 */
  APP_NAME?: string;
  /** 对外服务地址（文档/管理页展示） */
  SERVICE_URL?: string;
  /** 密码最短长度，默认 8 */
  PASSWORD_MIN_LENGTH?: string;
  /** 用户名最短长度，默认 3 */
  USERNAME_MIN?: string;
  /** 用户名最长长度，默认 32 */
  USERNAME_MAX?: string;
  /** 应用名最长长度，默认 64 */
  APP_NAME_MAX?: string;
  /** 应用描述最长长度，默认 256 */
  APP_DESC_MAX?: string;
  /** PBKDF2 迭代次数，默认 100000（只升不降） */
  PBKDF2_ITERATIONS?: string;
  /** 是否开放注册，默认 true */
  ALLOW_REGISTRATION?: string;
  /** register/login 是否强制 App 凭证，默认 false */
  REQUIRE_APP_CREDENTIALS?: string;
  ASSETS: Fetcher;
}

export interface UserRow {
  id: string;
  email: string;
  username: string;
  password_hash: string;
  /** 头像随机种子；空则按用户 id 推导 */
  avatar_seed: string | null;
  created_at: string;
  updated_at: string;
}

export interface PublicUser {
  id: string;
  email: string;
  username: string;
  createdAt: string;
  /** 头像接口路径，客户端拼 origin */
  avatarUrl: string;
  avatarSeed: string;
}

export interface SessionRow {
  id: string;
  user_id: string;
  token_hash: string;
  expires_at: string;
  created_at: string;
  app_id: string | null;
}

export interface AppRow {
  id: string;
  app_id: string;
  app_secret_hash: string;
  name: string;
  description: string;
  owner_id: string;
  status: "active" | "revoked";
  /** JSON 字符串，如 ["https://app.example.com/callback"] */
  redirect_uris: string;
  created_at: string;
  updated_at: string;
  secret_rotated_at: string | null;
}

export interface PublicApp {
  id: string;
  appId: string;
  name: string;
  description: string;
  status: "active" | "revoked";
  redirectUris: string[];
  createdAt: string;
  updatedAt: string;
  secretRotatedAt: string | null;
}

export interface AuthCodeRow {
  code: string;
  app_id: string;
  user_id: string;
  redirect_uri: string;
  expires_at: string;
  used: number;
  created_at: string;
}

export interface RegisterBody {
  email: string;
  username: string;
  password: string;
}

export interface LoginBody {
  email: string;
  password: string;
}

export interface CreateAppBody {
  name: string;
  description?: string;
  redirectUris?: string[];
}

export interface UpdateAppBody {
  redirectUris?: string[];
  name?: string;
  description?: string;
}

export interface AuthorizeCompleteBody {
  clientId?: string;
  client_id?: string;
  redirectUri?: string;
  redirect_uri?: string;
  state?: string;
  responseType?: string;
  response_type?: string;
}

export interface TokenExchangeBody {
  code?: string;
  clientId?: string;
  client_id?: string;
  clientSecret?: string;
  client_secret?: string;
  appSecret?: string;
  grant_type?: string;
}
