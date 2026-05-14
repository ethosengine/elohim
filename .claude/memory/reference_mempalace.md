---
name: MemPalace — wired historian + librarian substrate
description: Python CLI + library + MCP server (30 tools) with ChromaDB vector store and SQLite temporal entity-relationship graph. Wired in 2026-05-14; baked into udi-plus image; ~/.mempalace symlinked to /projects/elohim/.mempalace.
type: reference
originSessionId: b5ef4833-2583-4482-b36e-b595da75dafe
---
**URL**: https://github.com/mempalace/mempalace

**What it is**: A layered memory system — CLI tool (`mempalace`), Python library, MCP server (30 tools), and Claude Code auto-save hooks. Stores verbatim text (no summarization) and retrieves via semantic search + temporal graph.

**Architecture metaphor**:
- **Wings** = people/projects/agents (scoping containers; default = source-dir basename)
- **Rooms** = topics (auto-detected from folder structure at `mempalace init`)
- **Drawers** = individual content pieces (chunks from mined files)

**Storage**: ChromaDB vector store (`sentence-transformers/all-MiniLM-L6-v2`, ~88MB model baked into `/opt/mempalace/hf-cache`) + SQLite temporal entity-relationship graph. Fully offline — `HF_HUB_OFFLINE=1` and `TRANSFORMERS_OFFLINE=1` in devfile env; verified no network calls during mining.

## Where it lives (this workspace)

- **Image**: `harbor.ethosengine.com/devspaces/udi-plus-mem-rust-nix:latest` — mempalace + sentence-transformers + pre-warmed model baked in.
- **Palace data**: `/projects/elohim/.mempalace/palace` (PVC-persisted, 137M after pilot mine).
- **Symlink**: `~/.mempalace -> /projects/elohim/.mempalace` so plain CLI calls work without `--palace`. Mirrors the `~/.claude -> $CLAUDE_CONFIG_DIR` pattern. Applied manually per session; setup-mempalace command in devfile.yaml does it on demand but is deliberately NOT in postStart (see [[feedback_no_brittle_commands_in_poststart]]).
- **MCP server**: `mempalace-mcp --palace /projects/elohim/.mempalace/palace` — stdio, scoped per-subagent (not globally registered, so 30 tool schemas don't pollute parent context).

## Wings mined (2026-05-14 pilot)

| Wing | Source | Files | Drawers |
|---|---|---|---|
| `shifts` | `.claude/shifts/` | 76 | 1,306 |
| `memory` | `.claude/memory/` | 169 | 902 |
| `plans` | `genesis/plans/` | 212 | 7,442 |
| `elohim-protocol` | `genesis/docs/content/elohim-protocol/` | 185 | 3,216 |

Total: 642 files, 12,866 drawers. Mining is operator/agent-driven (no postStart automation). Re-mining: `mempalace init <dir> --no-llm --yes --auto-mine` (idempotent — skips already-filed drawers).

## Wired into

- **`historian.md`** — read-only tool surface (`mempalace_search`, `kg_query`, `traverse`, `find_tunnels`, etc.). Primary substrate; replaces grep+read+judgment with embedding similarity + temporal graph.
- **`librarian.md`** — read + curate surface (adds `sync`, `check_duplicate`, `add/update/delete_drawer`, `kg_add/invalidate`, `create/delete_tunnel`, `hook_settings`). `mempalace_check_duplicate` becomes the real comparator for dedupe-memory-scan (replacing TF-IDF approximation). `mempalace_sync` is the natural counterpart to cleanup-scan (prune drawers whose source files were deleted/moved/gitignored).

## Known constraints

- **`$MEMPALACE_HOME` env var is decorative** — the CLI does NOT consult it. Palace location is controlled by `~/.mempalace/config.json` (`palace_path` field) or `--palace` flag. The env var is set in devfile for documentation/discoverability only.
- **`$MEMPALACE_EMBEDDING_MODEL` env var is decorative** (3.3.5). `embedding.py` hardcodes `ONNXMiniLM_L6_V2` from ChromaDB — model is not swappable. Only `MEMPALACE_EMBEDDING_DEVICE` (cpu/cuda/coreml/dml) actually configures anything. Switching to a different model (e.g. bge-m3 from memsearch) requires forking mempalace and rewriting `_build_ef_class()` plus a new ONNX export — upstream-PR scope, not configuration.
- **File ownership** — palace files written as the calling uid. If session A writes as root and session B reads as user 1234, the latter gets ENOENT on `drwx------ root` dirs. Stay consistent.
- **Per-source-dir pollution** — `mempalace init` writes `mempalace.yaml` + `entities.json` INTO the mined source directory (room mapping + entity hints). Gitignored repo-wide via `**/mempalace.yaml`, `**/entities.json`.
