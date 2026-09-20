//! cloudflare-auth — Rust Worker (workers-rs)
//!
//! 模块职责：
//! - `router`：路由表
//! - `handlers`：各领域 HTTP 处理
//! - `http` / `db` / `session` / `time` / `validate` / `logging`：横切支撑
//! - `password` / `config` / `avatar` / `authorize`：密码与授权流

mod avatar;
mod authorize;
mod config;
mod db;
mod handlers;
mod http;
mod logging;
mod password;
mod router;
mod session;
mod time;
mod validate;

use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> worker::Result<Response> {
    router::handle(req, env, ctx).await
}
