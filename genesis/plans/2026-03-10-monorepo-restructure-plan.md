# Monorepo Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize the monorepo so directories reflect system boundaries (core, gateway, steward shells) instead of technology labels.

**Architecture:** Move directories in dependency order — leaf nodes first, then things that depend on them. Update all path references (Cargo.toml, pnpm-workspace, Jenkinsfiles, Dockerfiles, scripts) after each logical group of moves. Commit frequently so each commit is a coherent, describable change.

**Tech Stack:** git mv, Cargo workspace path dependencies, pnpm workspace, Jenkinsfile Groovy, Docker COPY contexts, shell scripts, justfile

**Key constraint:** This is a work-in-progress project. Tests may not all pass. Focus on getting moves right and updating path references. Don't try to fix pre-existing issues.

---

### Task 1: Move shared Rust crates to root

These have no internal path deps to each other (except elohim-sdk → elohim-storage-client and doorway-client, but those are siblings that move together).

**Files:**
- Move: `holochain/crates/doorway-client/` → `crates/doorway-client/`
- Move: `holochain/crates/elohim-sdk/` → `crates/elohim-sdk/`
- Move: `holochain/crates/elohim-storage-client/` → `crates/elohim-storage-client/`
- Modify: `crates/elohim-sdk/Cargo.toml` (internal path deps stay relative, just verify)

**Step 1: Create directory and move**
```bash
mkdir -p crates
git mv holochain/crates/doorway-client crates/doorway-client
git mv holochain/crates/elohim-sdk crates/elohim-sdk
git mv holochain/crates/elohim-storage-client crates/elohim-storage-client
```

**Step 2: Verify internal path deps in crates/elohim-sdk/Cargo.toml**
The relative paths `../elohim-storage-client` and `../doorway-client` should still work since siblings moved together.

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: move shared Rust crates to root crates/"
```

---

### Task 2: Move Holochain-specific dirs into elohim/holochain/

**Files:**
- Move: `holochain/dna/` → `elohim/holochain/dna/`
- Move: `holochain/holochain-cache-core/` → `elohim/holochain/holochain-cache-core/`
- Move: `holochain/rna/` → `elohim/holochain/rna/`
- Move: `holochain/edgenode/` → `elohim/holochain/edgenode/`
- Move: `holochain/elohim-wasm/` → `elohim/holochain/elohim-wasm/`

**Step 1: Create directory and move**
```bash
mkdir -p elohim/holochain
git mv holochain/dna elohim/holochain/dna
git mv holochain/holochain-cache-core elohim/holochain/holochain-cache-core
git mv holochain/rna elohim/holochain/rna
git mv holochain/edgenode elohim/holochain/edgenode
git mv holochain/elohim-wasm elohim/holochain/elohim-wasm
```

**Step 2: Update internal path deps in DNA workspace**

`elohim/holochain/dna/elohim/Cargo.toml` has:
```toml
doorway-client = { path = "../../crates/doorway-client" }
hc-rna = { path = "../../rna/rust" }
```
Update to:
```toml
doorway-client = { path = "../../../../crates/doorway-client" }
hc-rna = { path = "../../rna/rust" }
```
Note: `hc-rna` stays the same — `rna/` moved with `dna/` so relative path is preserved.

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: move Holochain-specific dirs to elohim/holochain/"
```

---

### Task 3: Move elohim-storage and elohim-bitswap into elohim/

**Files:**
- Move: `holochain/elohim-storage/` → `elohim/elohim-storage/`
- Move: `holochain/elohim-bitswap/` → `elohim/elohim-bitswap/`

**Step 1: Move**
```bash
git mv holochain/elohim-storage elohim/elohim-storage
git mv holochain/elohim-bitswap elohim/elohim-bitswap
```

**Step 2: Commit**
```bash
git add -A
git commit -m "refactor: move elohim-storage and elohim-bitswap to elohim/"
```

---

### Task 4: Move SDK and rust-ipfs into elohim/

**Files:**
- Move: `holochain/sdk/` → `elohim/sdk/`
- Move: `rust-ipfs/` → `elohim/rust-ipfs/` (git submodule — needs special handling)

**Step 1: Move SDK**
```bash
git mv holochain/sdk elohim/sdk
```

**Step 2: Move rust-ipfs submodule**
Git submodules require editing `.gitmodules` and re-adding:
```bash
# Update .gitmodules path
sed -i 's|path = rust-ipfs|path = elohim/rust-ipfs|' .gitmodules
# Move the directory
git mv rust-ipfs elohim/rust-ipfs
git add .gitmodules
```

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: move sdk and rust-ipfs into elohim/"
```

---

### Task 5: Move manifests to genesis/

**Files:**
- Move: `holochain/manifests/` → `genesis/manifests/`

**Step 1: Move**
```bash
git mv holochain/manifests genesis/manifests
```

**Step 2: Commit**
```bash
git add -A
git commit -m "refactor: move k8s manifests to genesis/"
```

---

### Task 6: Clean up remaining holochain/ directory

After tasks 1-5, `holochain/` should only contain `local-dev/`, `target/`, and the `Jenkinsfile`. Check what's left and either move or note.

**Step 1: Check remaining contents**
```bash
ls holochain/
```
Expected: `local-dev/`, `target/`, `Jenkinsfile`, possibly `CLAUDE.md`, `README`

**Step 2: Move Jenkinsfile**
The edge Jenkinsfile currently lives at `holochain/Jenkinsfile`. It builds doorway + storage + edgenode. It should move — but its `jenkinsPath` in orchestrator references `holochain/Jenkinsfile`. Move it and update orchestrator reference together in Task 13.

**Step 3: Move local-dev (TBD)**
For now, move to `scripts/local-dev/` or leave and decide later:
```bash
git mv holochain/local-dev scripts/local-dev  # or leave for now
```

**Step 4: Remove empty holochain/ directory**
```bash
# Git will auto-remove empty dirs. If target/ remains, it's in .gitignore anyway.
```

**Step 5: Commit**
```bash
git add -A
git commit -m "refactor: clean up holochain/ directory remnants"
```

---

### Task 7: Restructure steward/ (device + node)

**Files:**
- Move: `steward/*` → `steward/device/` (Tauri desktop)
- Move: `elohim-node/` → `steward/node/` (headless P2P)
- Modify: `steward/node/Cargo.toml` (path deps to elohim-storage, elohim-bitswap, elohim-agent, constitution)

**Step 1: Move current steward contents to device/**
```bash
# Create temp, move contents, then restructure
mkdir -p steward-tmp
git mv steward/* steward-tmp/ 2>/dev/null || true
# Handle dotfiles and special files
git mv steward/.* steward-tmp/ 2>/dev/null || true
mkdir -p steward/device
git mv steward-tmp/* steward/device/ 2>/dev/null || true
git mv steward-tmp/.* steward/device/ 2>/dev/null || true
rmdir steward-tmp
```
Alternative simpler approach:
```bash
cd /projects/elohim
mkdir steward-device-tmp
git mv steward steward-device-tmp/device
git mv steward-device-tmp steward
```

**Step 2: Move elohim-node to steward/node**
```bash
git mv elohim-node steward/node
```

**Step 3: Update steward/node/Cargo.toml path dependencies**
Current:
```toml
elohim-storage = { path = "../holochain/elohim-storage", ... }
elohim-bitswap = { path = "../holochain/elohim-bitswap" }
elohim-agent = { path = "../elohim/elohim-agent", ... }
constitution = { path = "../elohim/constitution" }
```
New:
```toml
elohim-storage = { path = "../../elohim/elohim-storage", ... }
elohim-bitswap = { path = "../../elohim/elohim-bitswap" }
elohim-agent = { path = "../../elohim/elohim-agent/elohim-agent-service", ... }
constitution = { path = "../../elohim/constitution" }
```
Note: elohim-agent path changes because of Task 8 restructure. If doing tasks sequentially, use `../../elohim/elohim-agent` here and update again in Task 8.

**Step 4: Commit**
```bash
git add -A
git commit -m "refactor: restructure steward/ into device/ and node/ shells"
```

---

### Task 8: Restructure elohim/elohim-agent/

**Files:**
- Move: `elohim/elohim-agent/` → `elohim/elohim-agent-tmp/elohim-agent-service/` (rename)
- Move: `elohim/elohim-agent-sdk/` → `elohim/elohim-agent/elohim-agent-sdk/`
- Move: `mcp-servers/` → `elohim/elohim-agent/mcp-servers/`

**Step 1: Restructure**
This is tricky because we're nesting `elohim-agent/` inside itself. Use a temp dir:
```bash
cd /projects/elohim
# Move current elohim-agent to temp name
git mv elohim/elohim-agent elohim/elohim-agent-service-tmp
# Create new elohim-agent parent
mkdir -p elohim/elohim-agent
# Move service into new parent
git mv elohim/elohim-agent-service-tmp elohim/elohim-agent/elohim-agent-service
# Move SDK into new parent
git mv elohim/elohim-agent-sdk elohim/elohim-agent/elohim-agent-sdk
# Move MCP servers into new parent
git mv mcp-servers/elohim-content elohim/elohim-agent/mcp-servers/elohim-content
rmdir mcp-servers 2>/dev/null || true
```

**Step 2: Update elohim/ Cargo.toml workspace members**
In `elohim/Cargo.toml`, update:
```toml
# Old
members = ["constitution", "elohim-agent", "eae"]
# New
members = ["constitution", "elohim-agent/elohim-agent-service", "eae"]
```
Also update workspace dependency:
```toml
# Old
elohim-agent = { path = "elohim-agent" }
# New
elohim-agent = { path = "elohim-agent/elohim-agent-service" }
```

**Step 3: Update steward/node/Cargo.toml if not already done**
```toml
elohim-agent = { path = "../../elohim/elohim-agent/elohim-agent-service", ... }
```

**Step 4: Commit**
```bash
git add -A
git commit -m "refactor: restructure elohim-agent into agent boundary with sdk, service, and mcp-servers"
```

---

### Task 9: Distribute research/ directories

**Files:**
- Move: `research/economic/` → `genesis/research/economic/`
- Move: `research/bootstrap/` → `elohim/research/bootstrap/`
- Move: `research/p2p-shipyard/` → `steward/research/p2p-shipyard/`
- Move: `research/sbd/` → `steward/research/sbd/`
- Move: `research/tx5/` → `steward/research/tx5/`
- Move: `research/tauri-plugin-holochain/` → `steward/device/research/tauri-plugin-holochain/`
- Move: `research/holo-envoy/` → `elohim/holochain/research/holo-envoy/`
- Move: `research/matrix/` → `elohim/research/matrix/`
- Move: `research/web-sdk/` → `elohim/sdk/research/web-sdk/`

**Step 1: Create directories and move**
```bash
mkdir -p genesis/research steward/research steward/device/research elohim/research elohim/holochain/research elohim/sdk/research

git mv research/economic genesis/research/economic
git mv research/bootstrap elohim/research/bootstrap
git mv research/p2p-shipyard steward/research/p2p-shipyard
git mv research/sbd steward/research/sbd
git mv research/tx5 steward/research/tx5
git mv research/tauri-plugin-holochain steward/device/research/tauri-plugin-holochain
git mv research/holo-envoy elohim/holochain/research/holo-envoy
git mv research/matrix elohim/research/matrix
git mv research/web-sdk elohim/sdk/research/web-sdk
```

**Step 2: Remove empty research/ if empty**
```bash
rmdir research 2>/dev/null || true
```

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: distribute research/ to nearest domain analogues"
```

---

### Task 10: Update doorway Cargo.toml path dependencies

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `doorway/doorway-service/Dockerfile`

**Step 1: Update Cargo.toml**
Current:
```toml
holochain-cache-core = { path = "../../holochain/holochain-cache-core" }
doorway-client = { path = "../../holochain/crates/doorway-client" }
```
New:
```toml
holochain-cache-core = { path = "../../elohim/holochain/holochain-cache-core" }
doorway-client = { path = "../../crates/doorway-client" }
```

**Step 2: Update Dockerfile COPY paths**
Current:
```dockerfile
COPY holochain/crates ./holochain/crates
COPY holochain/holochain-cache-core ./holochain/holochain-cache-core
```
And sed commands. Update to new paths:
```dockerfile
COPY crates ./crates
COPY elohim/holochain/holochain-cache-core ./elohim/holochain/holochain-cache-core
```
Update corresponding sed commands to match new Cargo.toml paths.

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: update doorway path deps for new directory structure"
```

---

### Task 11: Update pnpm-workspace.yaml and package.json references

**Files:**
- Modify: `pnpm-workspace.yaml`
- Modify: `elohim-app/package.json` (file: reference, npm scripts)

**Step 1: Update pnpm-workspace.yaml**
Current:
```yaml
packages:
  - elohim-app
  - doorway/doorway-app
  - elohim-library
  - elohim-library/projects/elohim-service
  - steward
  - genesis/seeder
  - genesis/a2o
  - genesis/orchestrator
  - holochain/sdk/storage-client-ts
  - mcp-servers/elohim-content
  - elohim/elohim-agent-sdk
```
New:
```yaml
packages:
  - elohim-app
  - doorway/doorway-app
  - elohim-library
  - elohim-library/projects/elohim-service
  - steward/device
  - genesis/seeder
  - genesis/a2o
  - genesis/orchestrator
  - elohim/sdk/storage-client-ts
  - elohim/elohim-agent/mcp-servers/elohim-content
  - elohim/elohim-agent/elohim-agent-sdk
```

**Step 2: Update elohim-app/package.json**
- `holochain-cache-core` file ref: `file:../holochain/holochain-cache-core/pkg` → `file:../elohim/holochain/holochain-cache-core/pkg`
- npm scripts referencing `../holochain/local-dev/` → update to wherever local-dev lands
- npm scripts referencing `../holochain/elohim-storage` → `../elohim/elohim-storage`

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: update pnpm workspace and package.json path references"
```

---

### Task 12: Update justfile

**Files:**
- Modify: `justfile`

**Step 1: Update directory variables**
Current:
```just
hc_dir      := root / "holochain"
steward_dir := root / "steward"
doorway_dir := root / "doorway"
node_dir    := root / "elohim-node"
storage_bin := hc_dir / "target/release/elohim-storage"
happ_path   := hc_dir / "dna/elohim/workdir/elohim.happ"
```
New:
```just
elohim_dir  := root / "elohim"
steward_dir := root / "steward" / "device"
doorway_dir := root / "doorway"
node_dir    := root / "steward" / "node"
storage_bin := elohim_dir / "elohim-storage/target/release/elohim-storage"
happ_path   := elohim_dir / "holochain/dna/elohim/workdir/elohim.happ"
```
Also update TAURI_CONFIG line referencing `../holochain/target/release/elohim-storage`.

**Step 2: Commit**
```bash
git add -A
git commit -m "refactor: update justfile paths for new structure"
```

---

### Task 13: Update Jenkinsfiles

This is the most critical task. Each Jenkinsfile has hardcoded paths for build stages, change detection, and artifact handling.

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` (PIPELINES map — change patterns, jenkinsPath)
- Modify: `Jenkinsfile` (app pipeline — holochain/sdk, holochain-cache-core refs)
- Modify: `holochain/Jenkinsfile` → move to appropriate location (edge pipeline)
- Modify: `elohim/holochain/dna/Jenkinsfile` (internal refs)
- Modify: `steward/device/Jenkinsfile` (holochain/dna, holochain/sdk refs)

**Step 1: Update orchestrator PIPELINES map**

`genesis/orchestrator/Jenkinsfile` — update `changePatterns` and `jenkinsPath`:
```groovy
'elohim-holochain': [
    jenkinsPath: 'elohim/holochain/dna/Jenkinsfile',
    changePatterns: ['elohim/holochain/dna/', 'elohim/holochain/holochain-cache-core/', 'elohim/holochain/rna/', 'VERSION'],
    // rest stays
],
'elohim-edge': [
    jenkinsPath: 'elohim/holochain/Jenkinsfile',  // or wherever edge Jenkinsfile moves
    changePatterns: ['doorway/doorway-service/', 'doorway/doorway-app/', 'elohim/elohim-agent/elohim-agent-sdk/', 'elohim/holochain/edgenode/', 'elohim/elohim-storage/', 'crates/', 'VERSION'],
    // rest stays
],
'elohim': [
    jenkinsPath: 'Jenkinsfile',
    changePatterns: ['elohim-app/', 'elohim-library/', 'elohim/sdk/', 'VERSION'],
    // rest stays
],
'elohim-steward': [
    jenkinsPath: 'steward/device/Jenkinsfile',
    changePatterns: ['steward/'],
    // rest stays
],
```

**Step 2: Move edge Jenkinsfile**
The edge Jenkinsfile at `holochain/Jenkinsfile` needs a new home. It builds doorway + storage + edgenode. Options:
- `elohim/holochain/Jenkinsfile` (since edgenode is there)
- A dedicated location
For now, move it with the holochain content:
```bash
git mv holochain/Jenkinsfile elohim/holochain/Jenkinsfile
```

**Step 3: Update app Jenkinsfile (root)**
Update lines referencing `holochain/`:
- `holochain/holochain-cache-core/pkg` → `elohim/holochain/holochain-cache-core/pkg`
- `holochain/sdk/storage-client-ts` → `elohim/sdk/storage-client-ts`

**Step 4: Update edge Jenkinsfile (now elohim/holochain/Jenkinsfile)**
Update all `dir()` and `cd` and `changeset` references:
- `holochain/elohim-storage` → `elohim/elohim-storage`
- `holochain/edgenode` → `elohim/holochain/edgenode`
- `elohim-node/simulation` → `steward/node/simulation`

**Step 5: Update DNA Jenkinsfile (elohim/holochain/dna/Jenkinsfile)**
Update references — most internal paths should still work since `dna/` moved with `holochain-cache-core/` and `rna/`.
- `../../holochain-cache-core` → still `../../holochain-cache-core` (relative preserved)
- Any absolute-from-root paths need updating

**Step 6: Update steward Jenkinsfile (steward/device/Jenkinsfile)**
- `../holochain/dna/elohim` → `../../elohim/holochain/dna/elohim`
- `holochain/sdk/storage-client-ts` → `elohim/sdk/storage-client-ts`

**Step 7: Commit**
```bash
git add -A
git commit -m "refactor: update all Jenkinsfile paths for new monorepo structure"
```

---

### Task 14: Update Docker and simulation configs

**Files:**
- Modify: `steward/node/simulation/docker-compose.yml` (was `elohim-node/simulation/docker-compose.yml`)
- Modify: various shell scripts in local-dev, generate-types.sh

**Step 1: Update docker-compose build contexts**
`steward/node/simulation/docker-compose.yml`:
- `../../holochain/elohim-storage` → `../../../elohim/elohim-storage`

**Step 2: Update shell scripts**
- `elohim/elohim-storage/scripts/generate-types.sh` — update comments/paths
- Any local-dev scripts that moved

**Step 3: Commit**
```bash
git add -A
git commit -m "refactor: update Docker and script paths for new structure"
```

---

### Task 15: Update CLAUDE.md, .claude/ config, and memory files

**Files:**
- Modify: `CLAUDE.md` (build commands, architecture docs)
- Modify: `.claude/settings.local.json` (allowed commands with paths)
- Modify: `.claude/file-relationships.json` (sync patterns)
- Modify: `/projects/.claude-config/projects/-projects-elohim/memory/MEMORY.md`

**Step 1: Update CLAUDE.md**
Update all build command paths, architecture references, file locations table, pipeline table.

**Step 2: Update .claude/settings.local.json**
Update allowed command patterns referencing `holochain/elohim-storage` → `elohim/elohim-storage`.

**Step 3: Update .claude/file-relationships.json**
Update trigger patterns and messages referencing old paths.

**Step 4: Update memory files**
Update any hardcoded paths in MEMORY.md.

**Step 5: Commit**
```bash
git add -A
git commit -m "docs: update CLAUDE.md and config files for new monorepo structure"
```

---

### Task 16: Run pnpm install and verify

**Step 1: Run pnpm install from root**
```bash
pnpm install
```
Verify workspace resolution works with updated paths.

**Step 2: Spot-check a Cargo build**
```bash
cd crates/elohim-sdk && cargo check
cd ../../doorway/doorway-service && RUSTFLAGS="" cargo check
cd ../../elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 3: Commit lockfile changes if any**
```bash
git add pnpm-lock.yaml Cargo.lock
git commit -m "chore: update lockfiles after restructure"
```

---

## Execution Order & Dependencies

```
Task 1 (crates)
Task 2 (holochain → elohim/holochain)  ─┐
Task 3 (storage + bitswap → elohim/)    ─┤
Task 4 (sdk + rust-ipfs → elohim/)      ─┤── can be done in sequence, no cross-deps
Task 5 (manifests → genesis/)           ─┤
Task 6 (cleanup holochain/)             ─┘
Task 7 (steward restructure)
Task 8 (elohim-agent restructure)
Task 9 (research distribution)
Task 10 (doorway Cargo.toml + Dockerfile) ── depends on Tasks 1-3
Task 11 (pnpm-workspace + package.json)  ── depends on Tasks 4, 7, 8
Task 12 (justfile)                       ── depends on Tasks 2-7
Task 13 (Jenkinsfiles)                   ── depends on all moves (Tasks 1-9)
Task 14 (Docker + scripts)              ── depends on Tasks 3, 7
Task 15 (docs + config)                 ── depends on all moves
Task 16 (verify)                        ── last
```
