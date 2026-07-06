//! Dry-run: parse every .claude/agents/*.md under a root, project both runtimes,
//! and report drift vs the on-disk .claude surface. Usage:
//!   cargo run -p eprfs-agent --example project_agents -- <repo-root>

use std::path::PathBuf;

use eprfs_agent::{project, CanonicalAgent, ProjectionBinding};
use eprfs_local::{has_drift, verify_projection};
use eprfs_storage::MemoryStorage;

#[tokio::main]
async fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let agents_dir = root.join(".claude/agents");

    let mut agents = Vec::new();
    let mut read = tokio::fs::read_dir(&agents_dir)
        .await
        .expect("read .claude/agents");
    while let Some(ent) = read.next_entry().await.unwrap() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let src = tokio::fs::read_to_string(&path).await.unwrap();
        match CanonicalAgent::parse(&src) {
            Ok(agent) => agents.push(agent),
            Err(e) => eprintln!("skip {}: {e}", path.display()),
        }
    }
    println!("parsed {} agent capabilities", agents.len());

    let storage = MemoryStorage::default();
    let manifest = project(&agents, &[ProjectionBinding::claude_agent()], &storage)
        .await
        .expect("project");

    let drifts = verify_projection(&manifest, &root).await.expect("verify");
    let dirty = drifts
        .iter()
        .filter(|d| d.status != eprfs_core::LocalOverlayStatus::Clean)
        .count();
    println!(
        "projected {} entries; {} drifted vs on-disk .claude (drift={})",
        manifest.entries.len(),
        dirty,
        has_drift(&drifts)
    );
}
