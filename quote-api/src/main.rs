use std::net::SocketAddr;

use anyhow::Context;
use tokio::signal::unix::{SignalKind, signal};

const DEFAULT_BIND: &str = "127.0.0.1:3095";
const DEFAULT_ORIGIN: &str = "https://milxliv.github.io";

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    // Startup and fatal errors only. Requests are never logged.
    tracing_subscriber::fmt().with_target(false).init();

    let bind: SocketAddr = std::env::var("NEARME_BIND")
        .unwrap_or_else(|_| DEFAULT_BIND.to_owned())
        .parse()
        .context("NEARME_BIND is not a socket address")?;
    let origin =
        std::env::var("NEARME_ALLOWED_ORIGIN").unwrap_or_else(|_| DEFAULT_ORIGIN.to_owned());

    let app = near_me_quote::app(&origin).context("NEARME_ALLOWED_ORIGIN is not a header value")?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("cannot bind {bind}"))?;
    tracing::info!("listening on {bind}; allowed origin {origin}");
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
