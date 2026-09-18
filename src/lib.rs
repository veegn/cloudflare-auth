//! cloudflare-auth — Rust Worker (workers-rs)

mod avatar;
mod authorize;
mod config;
mod password;
mod router;
mod util;

use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> worker::Result<Response> {
    router::handle(req, env, ctx).await
}
