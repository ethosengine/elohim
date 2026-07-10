//! Domain adapter: elohim capability-package tree -> `eprfs_core::ProjectionManifest`.
//!
//! This crate is the *domain* seam. All meaning about what an elohim
//! capability-package tree is lives here; `eprfs-core` stays domain-neutral and
//! only enforces the structural projection invariants (see `eprfs/CLAUDE.md`:
//! `domain adapter -> eprfs -> elohim-storage`).
//!
//! Content addressing is single-sourced through [`eprfs_core::BlobCid::compute`]
//! (CIDv1 / dag-cbor / sha2-256) — this crate never re-implements a CID.

use std::path::Path;

use eprfs_core::{
    BlobCid, EntryKind, EprRef, EprfsError, ProjectionEntry, ProjectionId, ProjectionManifest,
    ProjectionPath, ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
    Result,
};
use serde_json::{json, Value};

/// Namespace stamped on every projection source this adapter emits.
const NAMESPACE: &str = "elohim-agent";

/// Build a validated [`ProjectionManifest`] from a capability-package tree on disk.
///
/// Walks `root` deterministically (entries sorted by file name at every level).
/// Directories project as [`EntryKind::Directory`] with a `Container` source and
/// no blob; files project as [`EntryKind::File`] with `blob =
/// BlobCid::compute(bytes)`, a `Content` source, byte size, and
/// [`ProjectionStatus::Local`]. The returned manifest is guaranteed to satisfy
/// [`ProjectionManifest::validate`].
pub fn manifest_from_package_tree(root: &Path) -> Result<ProjectionManifest> {
    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;

    let manifest = ProjectionManifest {
        root: ProjectionRoot {
            id: ProjectionId::new("elohim-agent-packages"),
            root: EprRef::new("epr:elohim-agent/packages"),
        },
        entries,
        metadata: Value::Null,
    };

    // Fail loudly rather than hand back a structurally-invalid projection.
    manifest.validate()?;
    Ok(manifest)
}

/// Collect `(BlobCid, bytes)` for every file in the tree — the raw material a
/// later round-trip test seeds into storage before materializing the manifest.
/// Order is deterministic (same sorted walk as [`manifest_from_package_tree`]).
pub fn blobs_for_tree(root: &Path) -> Result<Vec<(BlobCid, Vec<u8>)>> {
    let mut blobs = Vec::new();
    collect_blobs(root, &mut blobs)?;
    Ok(blobs)
}

/// Emit a static, content-addressed "composition graph" for every elohim
/// skill/agent package under `package_root/{skills,agents}/*.json` — the CID
/// edge from the native package to its projected Claude/Codex artifacts, plus
/// who composed it. This is the verifiable "how was every skill composed"
/// artifact: every CID in the output is [`BlobCid::compute`] over real bytes,
/// never a re-implemented hash.
///
/// `projections_root`, when given, is the root of the projection tree (e.g.
/// `.epr-meta/elohim/projections`) — a skill's Claude projection is expected
/// at `<projections_root>/claude/skills/<id>/SKILL.md`, an agent's at
/// `<projections_root>/claude/agents/<id>.md` (and the `codex/` mirror of
/// each). A missing projection file yields `null`, not an error — projection
/// coverage is a fact the graph reports, not a precondition it enforces.
/// When `projections_root` is `None`, every node's `projections.claude` and
/// `projections.codex` are `null`.
///
/// `composed_by` is stamped onto every node's `composedBy` field verbatim —
/// the model/agent that authored the package, supplied by the caller.
///
/// Nodes are sorted by `(kind, id)` for a deterministic, diffable artifact.
pub fn compose_graph_from_package_tree(
    package_root: &Path,
    projections_root: Option<&Path>,
    composed_by: &str,
) -> Result<Value> {
    let mut nodes = collect_compose_nodes(
        package_root,
        projections_root,
        composed_by,
        "SkillPackage",
        "skills",
        true,
    )?;
    nodes.extend(collect_compose_nodes(
        package_root,
        projections_root,
        composed_by,
        "AgentPackage",
        "agents",
        false,
    )?);

    nodes.sort_by(|(a_key, _), (b_key, _)| a_key.cmp(b_key));

    Ok(json!({
        "generatedBy": "eprfs-agent compose-graph",
        "composedBy": composed_by,
        "nodes": nodes.into_iter().map(|(_, node)| node).collect::<Vec<_>>(),
    }))
}

/// Walk `package_root/<dir_name>/*.json`, emitting one compose-graph node per
/// package file. `is_skill` selects the projection path shape (skill ->
/// `<id>/SKILL.md` directory, agent -> `<id>.md` file).
fn collect_compose_nodes(
    package_root: &Path,
    projections_root: Option<&Path>,
    composed_by: &str,
    kind: &str,
    dir_name: &str,
    is_skill: bool,
) -> Result<Vec<((String, String), Value)>> {
    let dir = package_root.join(dir_name);
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }

    for child in sorted_children(&dir)? {
        if is_dir(&child)? || child.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let bytes = read_bytes(&child)?;
        let package_cid = BlobCid::compute(&bytes);
        let package: Value = serde_json::from_slice(&bytes)?;

        let metadata = package.get("metadata").cloned().unwrap_or(Value::Null);
        let id = metadata
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default();
        let source_runtime = metadata
            .get("sourceRuntime")
            .and_then(Value::as_str)
            .map(str::to_string);
        let master = metadata
            .get("master")
            .and_then(Value::as_str)
            .map(str::to_string);

        let (claude_cid, codex_cid) = match projections_root {
            Some(root) => (
                projection_cid(root, "claude", &id, is_skill)?,
                projection_cid(root, "codex", &id, is_skill)?,
            ),
            None => (None, None),
        };

        let node = json!({
            "kind": kind,
            "id": id,
            "sourceRuntime": source_runtime,
            "master": master,
            "packageCid": package_cid.to_string(),
            "projections": {
                "claude": claude_cid,
                "codex": codex_cid,
            },
            "composedBy": composed_by,
        });

        out.push(((kind.to_string(), id), node));
    }

    Ok(out)
}

/// The content-address of a single projected artifact, or `None` when the
/// file doesn't exist — absence is a reportable fact, not a failure.
fn projection_cid(
    projections_root: &Path,
    runtime: &str,
    id: &str,
    is_skill: bool,
) -> Result<Option<String>> {
    let path = if is_skill {
        projections_root
            .join(runtime)
            .join("skills")
            .join(id)
            .join("SKILL.md")
    } else {
        projections_root
            .join(runtime)
            .join("agents")
            .join(format!("{id}.md"))
    };

    if !path.exists() {
        return Ok(None);
    }

    let bytes = read_bytes(&path)?;
    Ok(Some(BlobCid::compute(&bytes).to_string()))
}

/// Sorted directory listing — the single source of deterministic ordering shared
/// by both walks.
fn sorted_children(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut children: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|source| EprfsError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry.map(|e| e.path()).map_err(|source| EprfsError::Io {
                path: dir.to_path_buf(),
                source,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    children.sort();
    Ok(children)
}

/// True for a directory (does not follow symlinks — a symlink projects as a file
/// via its own bytes, never as a container).
fn is_dir(path: &Path) -> Result<bool> {
    let meta = std::fs::symlink_metadata(path).map_err(|source| EprfsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(meta.is_dir())
}

/// Read a file's bytes, mapping I/O failure onto the eprfs error surface.
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| EprfsError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// The domain-neutral source id for an entry: `<Kind>:<file-name>`.
fn source_id(kind: &str, path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{kind}:{name}")
}

fn walk(base: &Path, current: &Path, entries: &mut Vec<ProjectionEntry>) -> Result<()> {
    for child in sorted_children(current)? {
        let rel = child
            .strip_prefix(base)
            .map_err(|_| EprfsError::InvalidProjectionPath(child.display().to_string()))?;
        let path = ProjectionPath::new(rel)?;

        if is_dir(&child)? {
            entries.push(ProjectionEntry {
                path,
                kind: EntryKind::Directory,
                source: Some(ProjectionSource::new(
                    NAMESPACE,
                    ProjectionSourceKind::Container,
                    source_id("Dir", &child),
                )),
                epr: None,
                blob: None,
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Local,
                metadata: Value::Null,
            });
            walk(base, &child, entries)?;
        } else {
            let bytes = read_bytes(&child)?;
            let mut entry = ProjectionEntry::file(path, BlobCid::compute(&bytes));
            entry.source = Some(ProjectionSource::new(
                NAMESPACE,
                ProjectionSourceKind::Content,
                source_id("File", &child),
            ));
            entry.size_bytes = Some(bytes.len() as u64);
            entry.status = ProjectionStatus::Local;
            entries.push(entry);
        }
    }
    Ok(())
}

fn collect_blobs(current: &Path, blobs: &mut Vec<(BlobCid, Vec<u8>)>) -> Result<()> {
    for child in sorted_children(current)? {
        if is_dir(&child)? {
            collect_blobs(&child, blobs)?;
        } else {
            let bytes = read_bytes(&child)?;
            blobs.push((BlobCid::compute(&bytes), bytes));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A throwaway on-disk tree of exactly 1 directory + 2 files, cleaned on drop.
    struct TempTree {
        root: std::path::PathBuf,
    }

    impl TempTree {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "elohim-agent-adapter-{}-{}",
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    const A_BYTES: &[u8] = b"package: skill\nname: alpha\n";
    const B_BYTES: &[u8] = b"# nested readme\n";

    fn seed(tree: &TempTree) {
        // top-level file
        fs::write(tree.root.join("manifest.yaml"), A_BYTES).unwrap();
        // one directory containing one file
        let sub = tree.root.join("docs");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("readme.md"), B_BYTES).unwrap();
    }

    #[test]
    fn maps_tree_to_valid_manifest() {
        let tree = TempTree::new();
        seed(&tree);

        let manifest = manifest_from_package_tree(&tree.root).expect("manifest builds");

        assert!(
            manifest.validate().is_ok(),
            "manifest must satisfy eprfs invariants"
        );
        assert_eq!(manifest.entries.len(), 3, "1 dir + 2 files == 3 entries");

        // The top-level file's blob is the canonical CID of its bytes.
        let expected = BlobCid::compute(A_BYTES);
        let file_entry = manifest
            .entries
            .iter()
            .find(|e| e.path.as_path() == Path::new("manifest.yaml"))
            .expect("top-level file entry present");
        assert_eq!(file_entry.kind, EntryKind::File);
        assert_eq!(file_entry.blob.as_ref(), Some(&expected));
        assert_eq!(file_entry.status, ProjectionStatus::Local);
        assert_eq!(file_entry.size_bytes, Some(A_BYTES.len() as u64));
    }

    #[test]
    fn blobs_for_tree_covers_every_file() {
        let tree = TempTree::new();
        seed(&tree);

        let blobs = blobs_for_tree(&tree.root).expect("blobs collected");
        assert_eq!(blobs.len(), 2, "two files -> two blobs");

        let expected_a = BlobCid::compute(A_BYTES);
        let expected_b = BlobCid::compute(B_BYTES);
        assert!(blobs
            .iter()
            .any(|(cid, bytes)| cid == &expected_a && bytes == A_BYTES));
        assert!(blobs
            .iter()
            .any(|(cid, bytes)| cid == &expected_b && bytes == B_BYTES));
    }

    /// Deterministic walk collecting `(relative_path, bytes)` for every file
    /// under `root` — the independent read-back used to prove materialization
    /// reproduced the source tree byte-identically.
    fn read_all_files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        fn walk_files(base: &Path, current: &Path, out: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
            let mut children: Vec<std::path::PathBuf> = fs::read_dir(current)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            children.sort();
            for child in children {
                if child.is_dir() {
                    walk_files(base, &child, out);
                } else {
                    let rel = child.strip_prefix(base).unwrap().to_path_buf();
                    let bytes = fs::read(&child).unwrap();
                    out.push((rel, bytes));
                }
            }
        }

        let mut out = Vec::new();
        walk_files(root, root, &mut out);
        out
    }

    #[tokio::test]
    async fn materializes_package_tree_byte_identical() {
        // A small fixture proves the mechanism without walking the real
        // 59-package tree: one nested directory, three files with distinct
        // bytes (including an empty file, a boundary the walk must not choke
        // on).
        let tree = TempTree::new();
        fs::write(tree.root.join("top.txt"), b"top-level bytes\n").unwrap();
        let nested = tree.root.join("nested").join("deeper");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("leaf.md"), b"# nested leaf\ncontent here\n").unwrap();
        fs::write(nested.join("empty.txt"), b"").unwrap();

        let manifest = manifest_from_package_tree(&tree.root).expect("manifest builds");

        let storage = eprfs_storage::MemoryStorage::default();
        for (cid, bytes) in blobs_for_tree(&tree.root).expect("blobs collected") {
            storage.insert_blob(cid, bytes::Bytes::from(bytes)).await;
        }

        let target = std::env::temp_dir().join(format!(
            "eaa-roundtrip-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = tokio::fs::remove_dir_all(&target).await;

        let materializer = eprfs_local::LocalMaterializer::new(storage);
        materializer
            .materialize(
                &manifest,
                &target,
                eprfs_core::MaterializationPolicy::LocalOnly,
            )
            .await
            .expect("materialization succeeds");

        // Every source file exists at target with identical bytes.
        let source_files = read_all_files(&tree.root);
        assert_eq!(source_files.len(), 3, "top.txt + leaf.md + empty.txt");
        for (rel, bytes) in source_files {
            let materialized = tokio::fs::read(target.join(&rel))
                .await
                .unwrap_or_else(|e| panic!("missing materialized file {}: {e}", rel.display()));
            assert_eq!(materialized, bytes, "mismatch: {}", rel.display());
        }

        let _ = tokio::fs::remove_dir_all(&target).await;
    }
}
