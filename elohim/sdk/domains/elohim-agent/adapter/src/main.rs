//! `eprfs-agent` — executable CLI over the elohim capability-package adapter.
//!
//! Three subcommands, each a thin shell around the library functions in
//! `elohim_agent_adapter`:
//!
//! - `manifest <package-root>`    -> print the `ProjectionManifest` as JSON
//! - `inspect <package-root>`     -> print entry/blob counts
//! - `materialize <root> <dest>`  -> round-trip the tree through eprfs onto disk
//!
//! This binary is the "is it actually executable" proof for the adapter: the
//! library alone cannot be run or installed, only linked against.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elohim_agent_adapter::{blobs_for_tree, manifest_from_package_tree};
use eprfs_core::{EntryKind, MaterializationPolicy};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("eprfs-agent: error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let command = args.get(1).map(String::as_str);

    match command {
        Some("manifest") => {
            let root = require_path_arg(args, 2, "manifest <package-root>")?;
            cmd_manifest(&root)
        }
        Some("inspect") => {
            let root = require_path_arg(args, 2, "inspect <package-root>")?;
            cmd_inspect(&root)
        }
        Some("materialize") => {
            let root = require_path_arg(args, 2, "materialize <package-root> <target-dir>")?;
            let target = require_path_arg(args, 3, "materialize <package-root> <target-dir>")?;
            cmd_materialize(&root, &target)
        }
        Some(other) => Err(format!("unknown subcommand '{other}'\n\n{}", usage())),
        None => Err(format!("missing subcommand\n\n{}", usage())),
    }
}

fn usage() -> String {
    "Usage:\n  \
     eprfs-agent manifest <package-root>\n  \
     eprfs-agent inspect <package-root>\n  \
     eprfs-agent materialize <package-root> <target-dir>"
        .to_string()
}

fn require_path_arg(args: &[String], index: usize, usage_line: &str) -> Result<PathBuf, String> {
    args.get(index)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing argument for `eprfs-agent {usage_line}`"))
}

fn cmd_manifest(root: &Path) -> Result<(), String> {
    let manifest = manifest_from_package_tree(root)
        .map_err(|source| format!("failed to build manifest for {}: {source}", root.display()))?;

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|source| format!("failed to serialize manifest: {source}"))?;

    println!("{json}");
    Ok(())
}

fn cmd_inspect(root: &Path) -> Result<(), String> {
    let manifest = manifest_from_package_tree(root)
        .map_err(|source| format!("failed to build manifest for {}: {source}", root.display()))?;

    let directories = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .count();
    let files = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .count();

    let blobs = blobs_for_tree(root)
        .map_err(|source| format!("failed to collect blobs for {}: {source}", root.display()))?;

    println!("package root: {}", root.display());
    println!("total entries: {}", manifest.entries.len());
    println!("directories: {directories}");
    println!("files: {files}");
    println!("blobs: {}", blobs.len());
    Ok(())
}

fn cmd_materialize(root: &Path, target: &Path) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| format!("failed to start async runtime: {source}"))?;

    runtime.block_on(materialize_async(root, target))
}

async fn materialize_async(root: &Path, target: &Path) -> Result<(), String> {
    let manifest = manifest_from_package_tree(root)
        .map_err(|source| format!("failed to build manifest for {}: {source}", root.display()))?;

    let blobs = blobs_for_tree(root)
        .map_err(|source| format!("failed to collect blobs for {}: {source}", root.display()))?;

    let storage = eprfs_storage::MemoryStorage::default();
    for (cid, bytes) in blobs {
        storage.insert_blob(cid, bytes::Bytes::from(bytes)).await;
    }

    let materializer = eprfs_local::LocalMaterializer::new(storage);
    let report = materializer
        .materialize(&manifest, target, MaterializationPolicy::LocalOnly)
        .await
        .map_err(|source| {
            format!(
                "failed to materialize {} into {}: {source}",
                root.display(),
                target.display()
            )
        })?;

    println!("materialized: {}", root.display());
    println!("target: {}", target.display());
    println!("directories: {}", report.directories);
    println!("files written: {}", report.files_written);
    println!("symlinks written: {}", report.symlinks_written);
    println!("link markers written: {}", report.link_markers_written);
    println!("files fetched: {}", report.files_fetched);
    println!("skipped (sparse): {}", report.skipped_sparse);
    Ok(())
}
