pub mod routes;
pub mod state;

use anyhow::Result;
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;

pub use routes::build_router;
pub use state::AppState;

pub async fn serve(vault_root: PathBuf, port: u16) -> Result<()> {
    let app = build_router(AppState::new(vault_root));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    println!("knowledge-agent listening on http://{}", local_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
