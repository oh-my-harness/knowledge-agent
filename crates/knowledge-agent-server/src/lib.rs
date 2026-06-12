pub mod routes;
pub mod state;

use anyhow::{Result, bail};
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;

pub use routes::{build_router, build_router_with_static};
pub use state::AppState;

pub async fn serve(vault_root: PathBuf, port: u16, web_dir: Option<PathBuf>) -> Result<()> {
    let web_dir = resolve_web_dir(web_dir)?;
    let app = match web_dir {
        Some(web_dir) => {
            println!("serving web UI from {}", web_dir.display());
            build_router_with_static(AppState::new(vault_root), web_dir)
        }
        None => {
            println!("web UI not found; serving API only");
            build_router(AppState::new(vault_root))
        }
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    println!("knowledge-agent listening on http://{}", local_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn resolve_web_dir(web_dir: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(web_dir) = web_dir {
        if web_dir.join("index.html").exists() {
            return Ok(Some(web_dir));
        }
        bail!(
            "web UI directory must contain index.html: {}",
            web_dir.display()
        );
    }

    Ok(default_web_dir())
}

fn default_web_dir() -> Option<PathBuf> {
    default_web_dir_candidates()
        .into_iter()
        .find(|candidate| candidate.join("index.html").exists())
}

fn default_web_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("web").join("dist")];
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        candidates.push(exe_dir.join("web").join("dist"));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::resolve_web_dir;

    #[test]
    fn accepts_explicit_web_dir_with_index_html() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("index.html"), "<main></main>").expect("write index");

        let resolved = resolve_web_dir(Some(dir.path().to_path_buf()))
            .expect("web dir should be valid")
            .expect("web dir should be returned");

        assert_eq!(resolved, dir.path());
    }

    #[test]
    fn rejects_explicit_web_dir_without_index_html() {
        let dir = tempfile::tempdir().expect("tempdir");

        let err = resolve_web_dir(Some(dir.path().to_path_buf())).expect_err("missing index");

        assert!(err.to_string().contains("index.html"));
    }
}
