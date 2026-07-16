//! Domain adapter: elohim capability-package tree -> `eprfs_core::ProjectionManifest`.
//!
//! This crate is the *domain* seam. All meaning about what an elohim
//! capability-package tree is lives here; `eprfs-core` stays domain-neutral and
//! only enforces the structural projection invariants (see `eprfs/CLAUDE.md`:
//! `domain adapter -> eprfs -> elohim-storage`).
//!
//! Content addressing is single-sourced through [`eprfs_core::BlobCid::compute`]
//! (CIDv1 / dag-cbor / sha2-256) — this crate never re-implements a CID.

use std::path::{Path, PathBuf};

use eprfs_core::{
    BlobCid, CompositionGraph, CompositionNode, DerivationEdge, EntryKind, EprRef, EprfsError,
    ProjectionEntry, ProjectionId, ProjectionManifest, ProjectionPath, ProjectionRoot,
    ProjectionSource, ProjectionSourceKind, ProjectionStatus, Result,
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

/// How a kind's projection fixtures are laid out under the projections-root
/// mirror. This is the ONLY per-kind projection knowledge in the producer;
/// everything else is generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionLayout {
    /// `<projRoot>/<runtime>/<dir>/<id>/<leaf>` — a per-id directory holding the
    /// projected file (skills -> `SKILL.md`, agentdocs -> the doc's own name).
    NestedById,
    /// `<projRoot>/<runtime>/<dir>/<leaf>` — the projected file sits flat in the
    /// kind dir (agents -> `<id>.md`, hooks -> `<id>.py`).
    Flat,
}

/// One row of the kind-agnostic compose table. Adding a future artifact kind is
/// exactly one row here — the walk, node/edge emission, and CID logic below are
/// all generic. `dir` names both the package sub-dir under `package_root` AND the
/// projection sub-dir under the projections-root mirror.
struct ComposeKind {
    /// Package sub-dir AND projection sub-dir (e.g. `skills`, `agents`, `hooks`,
    /// `agentdocs`).
    dir: &'static str,
    /// Fallback kind label used for the node source id (`<Kind>:<id>`) and node
    /// metadata when a package JSON omits its own top-level `kind`.
    default_kind: &'static str,
    layout: ProjectionLayout,
    /// True for kinds whose projection is a byte-identical passthrough of the
    /// package's `source.body` (hooks, agentdocs, commands — no frontmatter transform). For
    /// these the projection node is addressed with the raw codec and carries a
    /// `sourceDocCid` witness (see [`compose_graph_from_package_tree`]).
    verbatim_passthrough: bool,
}

/// The compose table — the single place the producer learns about artifact
/// kinds. Skills/agents each project into Claude + Codex (2 edges); hooks and
/// agentdocs project a single file (1 edge). The exact number of runtimes is NOT
/// hardcoded here: it's read from each package's declared `projections` map, so a
/// kind that grows a new runtime target needs no code change.
const COMPOSE_KINDS: &[ComposeKind] = &[
    ComposeKind {
        dir: "skills",
        default_kind: "SkillPackage",
        layout: ProjectionLayout::NestedById,
        verbatim_passthrough: false,
    },
    ComposeKind {
        dir: "agents",
        default_kind: "AgentPackage",
        layout: ProjectionLayout::Flat,
        verbatim_passthrough: false,
    },
    ComposeKind {
        dir: "hooks",
        default_kind: "HookPackage",
        layout: ProjectionLayout::Flat,
        verbatim_passthrough: true,
    },
    ComposeKind {
        dir: "agentdocs",
        default_kind: "AgentDocPackage",
        layout: ProjectionLayout::NestedById,
        verbatim_passthrough: true,
    },
    ComposeKind {
        dir: "commands",
        default_kind: "CommandPackage",
        layout: ProjectionLayout::Flat,
        verbatim_passthrough: true,
    },
    ComposeKind {
        dir: "mcp-servers",
        default_kind: "McpServerPackage",
        layout: ProjectionLayout::NestedById,
        verbatim_passthrough: false,
    },
    ComposeKind {
        dir: "mcp-profiles",
        default_kind: "McpProfilePackage",
        layout: ProjectionLayout::NestedById,
        verbatim_passthrough: false,
    },
];

/// Build a content-addressed [`CompositionGraph`] over every elohim capability
/// package under `package_root/{skills,agents,hooks,agentdocs,commands,...}/*.json` — the
/// verifiable "how was every artifact composed" derivation graph. Every CID in
/// the output is [`BlobCid::compute`] over real bytes; this producer never
/// re-implements a hash.
///
/// For each package JSON the graph gets:
/// - a **package node** (cid = the package bytes; source =
///   `elohim-agent`/`Content`/`<Kind>:<id>`; metadata = `master`,
///   `sourceRuntime`, `composedBy`, `id`, `kind`),
/// - a **projection node** per projected artifact that exists on disk, and
/// - a **`Projection` derivation edge** package -> projection, attributed to
///   `composed_by`.
///
/// The set of projected runtimes is read from each package's declared
/// `projections` map; the per-kind fixture LAYOUT (nested-by-id vs flat) comes
/// from [`COMPOSE_KINDS`]. A projection whose fixture is absent is simply
/// omitted — coverage is a fact the graph reports, not a precondition. When
/// `projections_root` is `None`, only package nodes are emitted.
///
/// For **verbatim-passthrough** kinds (hooks, agentdocs, commands — whose projection is a
/// byte-identical copy of the package's `source.body`), the projection node is
/// addressed with the raw codec and carries a `sourceDocCid =
/// BlobCid::compute_raw(source.body)` witness in its metadata. Because the node's
/// own CID is then `compute_raw` over the projection bytes, `sourceDocCid` equals
/// the projection CID exactly — a content-addressed proof that "projection ==
/// source doc" (it would diverge the instant a projection stopped matching its
/// source body).
///
/// The kinds are walked in [`COMPOSE_KINDS`] order, packages in sorted filename
/// order, and runtimes in sorted key order, so the output is deterministic and
/// diffable. The returned graph satisfies [`CompositionGraph::validate`].
pub fn compose_graph_from_package_tree(
    package_root: &Path,
    projections_root: Option<&Path>,
    composed_by: &str,
) -> Result<CompositionGraph> {
    let mut graph = CompositionGraph::new().with_metadata(json!({
        "generatedBy": "eprfs-agent compose-graph",
        "composedBy": composed_by,
    }));

    for spec in COMPOSE_KINDS {
        collect_compose_kind(
            &mut graph,
            spec,
            package_root,
            projections_root,
            composed_by,
        )?;
    }

    // Fail loudly rather than hand back a structurally-invalid graph.
    graph.validate()?;
    Ok(graph)
}

/// Walk `package_root/<spec.dir>/*.json`, appending a package node plus its
/// projection nodes and `Projection` edges to `graph`.
fn collect_compose_kind(
    graph: &mut CompositionGraph,
    spec: &ComposeKind,
    package_root: &Path,
    projections_root: Option<&Path>,
    composed_by: &str,
) -> Result<()> {
    let dir = package_root.join(spec.dir);
    if !dir.exists() {
        return Ok(());
    }

    for child in sorted_children(&dir)? {
        if is_dir(&child)? || child.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let bytes = read_bytes(&child)?;
        let package_cid = BlobCid::compute(&bytes);
        let package: Value = serde_json::from_slice(&bytes)?;

        let kind = package
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or(spec.default_kind)
            .to_string();

        let metadata = package.get("metadata").cloned().unwrap_or(Value::Null);
        let id = meta_str(&metadata, "id").unwrap_or_default();
        let source_runtime = meta_str(&metadata, "sourceRuntime");
        let master = meta_str(&metadata, "master");
        // The package's own author credit, falling back to the caller's --composed-by.
        let node_composed_by = meta_str(&metadata, "composedBy").unwrap_or(composed_by.to_string());

        // Package node: cid over the package bytes, domain-neutral source identity.
        graph.add_node(CompositionNode::new(
            package_cid.clone(),
            ProjectionSource::new(
                NAMESPACE,
                ProjectionSourceKind::Content,
                format!("{kind}:{id}"),
            ),
            json!({
                "role": "package",
                "kind": kind,
                "id": id,
                "master": master,
                "sourceRuntime": source_runtime,
                "composedBy": node_composed_by,
            }),
        ));

        // Verbatim-passthrough kinds (hooks, agentdocs) project `source.body`
        // byte-for-byte. For them we witness "projection == source doc" as a
        // content address: `sourceDocCid = compute_raw(source.body)` — recorded on
        // the projection node — must EQUAL the projection's own raw CID. The
        // equality is proven by the CIDs, never asserted: if a projection ever
        // diverged from its source body, the two raw CIDs would differ.
        let source_doc_cid = if spec.verbatim_passthrough {
            package
                .get("source")
                .and_then(|source| source.get("body"))
                .and_then(Value::as_str)
                .map(|body| BlobCid::compute_raw(body.as_bytes()))
        } else {
            None
        };

        // One projection node + edge per declared runtime whose fixture exists.
        let Some(root) = projections_root else {
            continue;
        };
        for runtime in declared_runtimes(&package) {
            let Some(declared) = declared_projection_path(&package, &runtime) else {
                continue;
            };
            let Some(leaf) = Path::new(&declared).file_name() else {
                continue;
            };
            let mirror = mirror_projection_path(root, &runtime, spec, &id, leaf);
            if !mirror.exists() {
                continue;
            }

            let proj_bytes = read_bytes(&mirror)?;
            // Verbatim-passthrough projections ARE raw document bytes, so they take
            // the raw codec (bafkrei…): the node's own CID is then `compute_raw` over
            // its bytes and — when projection == source doc — equals `source_doc_cid`.
            // Composed kinds (skills/agents) keep the dag-cbor addressing used
            // elsewhere in the graph.
            let proj_cid = if spec.verbatim_passthrough {
                BlobCid::compute_raw(&proj_bytes)
            } else {
                BlobCid::compute(&proj_bytes)
            };

            let mut proj_metadata = json!({
                "role": "projection",
                "kind": kind,
                "id": id,
                "runtime": runtime,
                "path": mirror.to_string_lossy(),
                "declaredPath": declared,
            });
            // The witness: for verbatim kinds, sourceDocCid == this node's cid.
            if let Some(source_doc_cid) = &source_doc_cid {
                proj_metadata["sourceDocCid"] = json!(source_doc_cid.to_string());
            }

            graph.add_node(CompositionNode::new(
                proj_cid.clone(),
                ProjectionSource::new(
                    NAMESPACE,
                    ProjectionSourceKind::Content,
                    format!("{id}@{runtime}"),
                ),
                proj_metadata,
            ));

            graph.add_edge(DerivationEdge::projection(
                package_cid.clone(),
                proj_cid,
                composed_by,
                json!({ "runtime": runtime }),
            ));
        }
    }

    Ok(())
}

/// Read a string field from a package's `metadata` bag.
fn meta_str(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// The projection runtime keys a package declares, sorted for deterministic
/// output. Empty when the package declares no `projections` map.
fn declared_runtimes(package: &Value) -> Vec<String> {
    let mut runtimes: Vec<String> = package
        .get("projections")
        .and_then(Value::as_object)
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    runtimes.sort();
    runtimes
}

/// The repo-relative runtime path a package declares for `runtime` (e.g.
/// `.claude/skills/<id>/SKILL.md`). The mirror fixture's leaf filename is derived
/// from this, so a kind whose leaf name is doc-specific (agentdocs: `CLAUDE.md`)
/// needs no special case.
fn declared_projection_path(package: &Value, runtime: &str) -> Option<String> {
    package
        .get("projections")
        .and_then(|p| p.get(runtime))
        .and_then(|entry| entry.get("path"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Build the projections-root MIRROR path for one projected artifact. The leaf
/// filename comes from the package's declared runtime path; the LAYOUT (whether
/// an `<id>` directory segment is inserted) is the kind's only per-kind
/// projection knowledge.
fn mirror_projection_path(
    projections_root: &Path,
    runtime: &str,
    spec: &ComposeKind,
    id: &str,
    leaf: &std::ffi::OsStr,
) -> PathBuf {
    let base = projections_root.join(runtime).join(spec.dir);
    match spec.layout {
        ProjectionLayout::NestedById => base.join(id).join(leaf),
        ProjectionLayout::Flat => base.join(leaf),
    }
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

    #[test]
    fn compose_graph_is_kind_agnostic_over_the_package_tree() {
        let tree = TempTree::new();
        let pkg_root = tree.root.join("packages");
        let proj_root = tree.root.join("projections");

        // Helper: write a package JSON + (optionally) its projection fixtures.
        let write = |rel: &str, bytes: &[u8]| {
            let p = pkg_root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, bytes).unwrap();
        };
        let write_proj = |rel: &str, bytes: &[u8]| {
            let p = proj_root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, bytes).unwrap();
        };

        // A skill: claude + codex projections (nested-by-id, SKILL.md leaf).
        write(
            "skills/alpha.json",
            br#"{"kind":"SkillPackage","metadata":{"id":"alpha","master":"package","sourceRuntime":"claude","composedBy":"claude-opus-4-8"},"projections":{"claude":{"path":".claude/skills/alpha/SKILL.md"},"codex":{"path":".codex/skills/alpha/SKILL.md"}}}"#,
        );
        write_proj("claude/skills/alpha/SKILL.md", b"# alpha claude\n");
        write_proj("codex/skills/alpha/SKILL.md", b"# alpha codex\n");

        // A hook: single claude projection (flat, .py leaf).
        write(
            "hooks/beta.json",
            br#"{"kind":"HookPackage","metadata":{"id":"beta","master":"package"},"projections":{"claude":{"path":".claude/hooks/beta.py"}}}"#,
        );
        write_proj("claude/hooks/beta.py", b"print('beta')\n");

        // An agentdoc: single claude projection whose leaf is the doc's own name
        // (nested-by-id) — the declared path is an arbitrary repo path.
        write(
            "agentdocs/gamma.json",
            br#"{"kind":"AgentDocPackage","metadata":{"id":"gamma","master":"package"},"projections":{"claude":{"path":"some/where/CLAUDE.md"}}}"#,
        );
        write_proj("claude/agentdocs/gamma/CLAUDE.md", b"# gamma gospel\n");

        // A command: claude + codex projections, both flat and byte-identical.
        write(
            "commands/delta.json",
            br##"{"kind":"CommandPackage","metadata":{"id":"delta","master":"package"},"source":{"body":"# delta\n"},"projections":{"claude":{"path":".claude/commands/delta.md"},"codex":{"path":".codex/commands/delta.md"}}}"##,
        );
        write_proj("claude/commands/delta.md", b"# delta\n");
        write_proj("codex/commands/delta.md", b"# delta\n");

        // An MCP server definition and activation profile both use nested
        // fixtures because their runtime paths are aggregate root config files.
        write(
            "mcp-servers/jenkins.json",
            br#"{"kind":"McpServerPackage","metadata":{"id":"jenkins","master":"package"},"projections":{"claude":{"path":".mcp.json"},"codex":{"path":".codex/config.toml"}}}"#,
        );
        write_proj(
            "claude/mcp-servers/jenkins/.mcp.json",
            b"{\"mcpServers\":{\"jenkins\":{}}}\n",
        );
        write_proj(
            "codex/mcp-servers/jenkins/config.toml",
            b"[mcp_servers.jenkins]\n",
        );
        write(
            "mcp-profiles/project.json",
            br#"{"kind":"McpProfilePackage","metadata":{"id":"project","master":"package"},"projections":{"claude":{"path":".mcp.json"},"codex":{"path":".codex/config.toml"}}}"#,
        );
        write_proj(
            "claude/mcp-profiles/project/.mcp.json",
            b"{\"mcpServers\":{\"jenkins\":{}}}\n",
        );
        write_proj(
            "codex/mcp-profiles/project/config.toml",
            b"[mcp_servers.jenkins]\n",
        );

        let graph = compose_graph_from_package_tree(&pkg_root, Some(&proj_root), "claude-opus-4-8")
            .unwrap();

        assert!(graph.validate().is_ok(), "graph must satisfy invariants");

        // 6 package nodes + 10 projection nodes (2 skill + 1 hook + 1 agentdoc
        // + 2 command + 2 MCP server + 2 MCP profile).
        assert_eq!(graph.nodes.len(), 16, "6 packages + 10 projections");
        assert_eq!(graph.edges.len(), 10, "one edge per existing projection");

        // Every edge's source is a package node cid, derived a projection cid,
        // and the package's cid is the canonical CID of its bytes.
        let alpha_bytes = fs::read(pkg_root.join("skills/alpha.json")).unwrap();
        let alpha_cid = BlobCid::compute(&alpha_bytes);
        let alpha_edges = graph.edges.iter().filter(|e| e.source == alpha_cid).count();
        assert_eq!(alpha_edges, 2, "alpha skill has claude + codex edges");

        // The package node carries master/composedBy in metadata.
        let alpha_node = graph
            .nodes
            .iter()
            .find(|n| n.cid == alpha_cid)
            .expect("alpha package node present");
        assert_eq!(alpha_node.metadata["master"], "package");
        assert_eq!(alpha_node.metadata["composedBy"], "claude-opus-4-8");
        assert_eq!(alpha_node.source.namespace, NAMESPACE);
        assert_eq!(alpha_node.source.id, "SkillPackage:alpha");

        let profile_bytes = fs::read(pkg_root.join("mcp-profiles/project.json")).unwrap();
        let profile_cid = BlobCid::compute(&profile_bytes);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.source == profile_cid)
                .count(),
            2,
            "MCP profile has claude + codex projection edges"
        );
    }

    #[test]
    fn compose_graph_without_projections_root_is_package_only() {
        let tree = TempTree::new();
        let pkg_root = tree.root.join("packages");
        fs::create_dir_all(pkg_root.join("skills")).unwrap();
        fs::write(
            pkg_root.join("skills/alpha.json"),
            br#"{"kind":"SkillPackage","metadata":{"id":"alpha"},"projections":{"claude":{"path":".claude/skills/alpha/SKILL.md"}}}"#,
        )
        .unwrap();

        let graph = compose_graph_from_package_tree(&pkg_root, None, "x").unwrap();
        assert_eq!(graph.nodes.len(), 1, "package node only");
        assert!(graph.edges.is_empty(), "no projections => no edges");
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn verbatim_kind_projection_witnesses_source_doc_cid() {
        // For verbatim-passthrough kinds (AgentDocPackage, HookPackage,
        // CommandPackage) the
        // projection is a byte-identical copy of the package `source.body`. The
        // projection node must carry `sourceDocCid = compute_raw(source.body)` AND
        // its own CID must be `compute_raw` over the projection bytes — so
        // `sourceDocCid == projectionCid`, a content-addressed proof, not an
        // assertion.
        let tree = TempTree::new();
        let pkg_root = tree.root.join("packages");
        let proj_root = tree.root.join("projections");

        let write = |rel: &str, bytes: &[u8]| {
            let p = pkg_root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, bytes).unwrap();
        };
        let write_proj = |rel: &str, bytes: &[u8]| {
            let p = proj_root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, bytes).unwrap();
        };

        // AgentDocPackage: source.body is the whole gospel doc, projected verbatim
        // (nested-by-id layout, leaf = the doc's own name).
        const DOC_BODY: &[u8] = b"# CLAUDE.md\n\nverbatim gospel body\n";
        write(
            "agentdocs/gospel.json",
            format!(
                r#"{{"kind":"AgentDocPackage","metadata":{{"id":"gospel"}},"source":{{"body":{}}},"projections":{{"claude":{{"path":"CLAUDE.md"}}}}}}"#,
                serde_json::to_string(&String::from_utf8(DOC_BODY.to_vec()).unwrap()).unwrap()
            )
            .as_bytes(),
        );
        write_proj("claude/agentdocs/gospel/CLAUDE.md", DOC_BODY);

        // HookPackage: source.body is the hook code, projected verbatim (flat).
        const HOOK_BODY: &[u8] = b"#!/usr/bin/env python3\nprint('hook')\n";
        write(
            "hooks/gate.json",
            format!(
                r#"{{"kind":"HookPackage","metadata":{{"id":"gate"}},"source":{{"body":{}}},"projections":{{"claude":{{"path":".claude/hooks/gate.py"}}}}}}"#,
                serde_json::to_string(&String::from_utf8(HOOK_BODY.to_vec()).unwrap()).unwrap()
            )
            .as_bytes(),
        );
        write_proj("claude/hooks/gate.py", HOOK_BODY);

        // CommandPackage: source.body is one flat markdown command projected
        // byte-identically into both runtime homes.
        const COMMAND_BODY: &[u8] = b"# /do-it\n\nDo the thing.\n";
        write(
            "commands/do-it.json",
            format!(
                r#"{{"kind":"CommandPackage","metadata":{{"id":"do-it"}},"source":{{"body":{}}},"projections":{{"claude":{{"path":".claude/commands/do-it.md"}},"codex":{{"path":".codex/commands/do-it.md"}}}}}}"#,
                serde_json::to_string(&String::from_utf8(COMMAND_BODY.to_vec()).unwrap()).unwrap()
            )
            .as_bytes(),
        );
        write_proj("claude/commands/do-it.md", COMMAND_BODY);
        write_proj("codex/commands/do-it.md", COMMAND_BODY);

        let graph = compose_graph_from_package_tree(&pkg_root, Some(&proj_root), "claude-opus-4-8")
            .unwrap();
        assert!(graph.validate().is_ok());

        // Assert the witness on each verbatim projection node.
        for (id, body, expected_projections) in [
            ("gospel", DOC_BODY, 1),
            ("gate", HOOK_BODY, 1),
            ("do-it", COMMAND_BODY, 2),
        ] {
            let nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.metadata["role"] == "projection" && n.metadata["id"] == id)
                .collect();
            assert_eq!(
                nodes.len(),
                expected_projections,
                "projection count for {id}"
            );

            let expected = BlobCid::compute_raw(body);
            for node in nodes {
                // sourceDocCid witness present and equals compute_raw(source.body)…
                assert_eq!(
                    node.metadata["sourceDocCid"],
                    expected.to_string(),
                    "sourceDocCid must be compute_raw(source.body) for {id}"
                );
                // …and equals the projection node's OWN cid (the content-addressed
                // "projection == source doc" proof).
                assert_eq!(
                    node.cid, expected,
                    "projection cid must equal sourceDocCid for verbatim kind {id}"
                );
                // Verbatim projections use the raw codec (bafkrei…).
                assert!(
                    node.cid.to_string().starts_with("bafkrei"),
                    "verbatim projection cid must be raw-codec for {id}"
                );
            }
        }
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
