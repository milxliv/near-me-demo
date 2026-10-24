use std::net::SocketAddr;

use anyhow::Context;
use near_me_quote::wikiquote::Wikiquote;
use tokio::signal::unix::{SignalKind, signal};

const DEFAULT_BIND: &str = "127.0.0.1:3095";
const DEFAULT_ORIGIN: &str = "https://milxliv.github.io";

fn main() -> anyhow::Result<()> {
    // Startup and fatal errors only. Requests are never logged.
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .init();
    // Small and bounded: DNS lookups run on the blocking pool.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .context("cannot start the runtime")?
        .block_on(serve())
}

async fn serve() -> anyhow::Result<()> {
    let bind: SocketAddr = std::env::var("NEARME_BIND")
        .unwrap_or_else(|_| DEFAULT_BIND.to_owned())
        .parse()
        .context("NEARME_BIND is not a socket address")?;
    let origin =
        std::env::var("NEARME_ALLOWED_ORIGIN").unwrap_or_else(|_| DEFAULT_ORIGIN.to_owned());

    // NEARME_LIVE_LOOKUP=off answers from the curated list only.
    let live = match std::env::var("NEARME_LIVE_LOOKUP").as_deref() {
        Ok("off") => None,
        _ => Some(Wikiquote::new().context("cannot build the Wikiquote client")?),
    };
    let lookup = if live.is_some() { "on" } else { "off" };

    let app =
        near_me_quote::app(&origin, live).context("NEARME_ALLOWED_ORIGIN is not a header value")?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("cannot bind {bind}"))?;
    tracing::info!("listening on {bind}; allowed origin {origin}; live lookup {lookup}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .context("server stopped")?;
    Ok(())
}

/// Resolves on SIGTERM (systemd stop) or Ctrl-C.
async fn shutdown() {
    match signal(SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = term.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        }
        Err(err) => {
            tracing::warn!("no SIGTERM handler ({err}); stopping on Ctrl-C only");
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}
