pub mod routes;
pub mod state;

use anyhow::Result;
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;

pub use routes::{build_router, build_router_with_static};
pub use state::AppState;

pub async fn serve(vault_root: PathBuf, port: u16, web_dir: Option<PathBuf>) -> Result<()> {
    let web_dir = web_dir.or_else(default_web_dir);
    let app = match web_dir {
        Some(web_dir) => {
            println!("serving web UI from {}", web_dir.display());
            build_router_with_static(AppState::new(vault_root), web_dir)
        }
        None => build_router(AppState::new(vault_root)),
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    println!("knowledge-agent listening on http://{}", local_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn default_web_dir() -> Option<PathBuf> {
    let candidate = PathBuf::from("web").join("dist");
    candidate.join("index.html").exists().then_some(candidate)
}
