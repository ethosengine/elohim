---
title: "Two-peer recovery harness — honesty fix, per-peer transport, peer-loss + recovery primitive, one quiesce reader, baseline series"
id: two-peer-recovery-harness-plan
status: Draft
class: substrate
domain: local mesh harness × peer-hoster dataplane (T2) × quiescence measurement
sprint: proposed (Plan 1 of 2 for the transport self-awareness arc; Plan 2 = PathObservation/select_path, written after this baseline exists)
habits: [dataplane-convergence]
cites:
  - "transport-self-awareness-diversity-harness-design | the spec this plan implements the baseline half of (§3.0, §3.3, §3.4, §3.5 witness); §3.1-3.2 and the flag-on measure are Plan 2 | sha256:9713242ed4162305 | path: genesis/docs/superpowers/specs/2026-08-24-transport-self-awareness-diversity-harness-design.md"
  - app/elohim-app/scripts/hc-mesh.sh
  - app/elohim-app/scripts/hc-mesh-quiesce.sh
  - app/elohim-app/scripts/hc-mesh-transport-matrix.sh
  - genesis/scripts/quiesce-timeline.py
  - scripts/ci/fleet-quiesce-gate.sh
  - elohim/elohim-storage/src/sync/mod.rs
  - elohim/elohim-storage/src/sync/doc_store.rs
  - elohim/elohim-storage/src/p2p/mod.rs
---

# Two-Peer Recovery Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the local mesh able to cycle two peers and two doorways through every transport pair, inflict peer loss, and time recovery from the survivor — recorded by the same reader the fleet's quiesce gate uses — so a baseline `time_to_recover` per transport pair exists *before* any transport self-awareness code lands.

**Architecture:** Four cuts in dependency order. (0) `syncDocuments` stops lying (`count_all()`); (1) `hc-mesh.sh` learns per-peer transport (`MESH_PEER_TRANSPORTS`) and prints its own footprint; (2) `hc-mesh-quiesce.sh` tees the gate's lines to a file and `quiesce-timeline.py --local` reads them into the same series as the fleet; (3) a new `hc-mesh-recovery.sh` implements the churn primitive — stop one peer's storage, wipe its recoverable state (warm) or its transport identity too (cold), restart it in the scenario's transport, poll the four saga-derived predicates, emit one record — and `hc-mesh-recovery-matrix.sh` cycles the two slots through a declared scenario library. The configurations are the library; the two slots cycle through it; no scenario ever adds a process.

**Tech Stack:** bash (hc-mesh family, `set -u`, sourced-safe), python3 (`quiesce-timeline.py`, stdlib only), Rust (`elohim-storage`, one method + one test), `just` (the `mesh` verb).

**Spec:** `genesis/docs/superpowers/specs/2026-08-24-transport-self-awareness-diversity-harness-design.md` — §3.0 (Cut 0), §3.3 (harness + recovery primitive), §3.4 (measure). Plan 2 covers §3.1–3.2.

## Global Constraints

- The local mesh for this arc is **exactly two peers, two doorways**: `MESH_PEERS=matthew,jessica`. Never add a third process to prove a scenario — add a scenario to the library.
- Storage binary slot the running mesh uses: `/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev/debug/elohim-storage`. Rust builds for the mesh go INTO that slot with `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev`, `RUSTFLAGS='--cfg getrandom_backend="custom"'`, `RUSTC_WRAPPER=""`. The FINAL build before any restart must be feature-full: `--features "p2p p2p-iroh"` (a p2p-only build makes dual/iroh restarts refuse).
- `cargo nextest` is not installed; use `cargo test`. Never judge a cargo run from piped output — echo `EXIT=$?` on its own line.
- `hc-mesh.sh` is `set -u` and is SOURCED by `just test mesh` and the matrix script: every new variable needs a `${VAR:-default}`; new functions must not execute on source.
- `hc-mesh.sh stop` kills its own process group — never run `stop` inside a backgrounded compound command.
- Recovery wipes touch ONLY `$MESH_DIR/<peer>/` (storage state). Conductor sandboxes under `elohim/holochain/local-dev/<peer>/` are never touched by this plan (agent keys and DHT survive both shapes; cold join = new *transport* identity, declared limit in Task 6).
- The "recovered" predicate is the spec's four saga-derived legs, read over HTTP only (no filesystem diffs — blob dir layouts differ per transport): P1 head parity (`/db/content/<id>` 200 with equal `blobHash` for every survivor content row that has one), P2 blob bytes (`GET /blob/<hash>` 200 on the recovering peer for each of those hashes), P3 `/p2p/status.pull.caughtUp == true && pull.failed == 0`, P4 doorway A (`:8888`) and B (`:8889`) both 200 on `/db/content/elohim-host-landing`. Plus P0 sync-doc parity (`/sync/v1/elohim/docs?limit=1` `.total` equal).
- One reader: `genesis/scripts/quiesce-timeline.py`. No second parser of gate lines anywhere.
- **P2P gate note (answers the design-audit flag on the route mentions below):** this plan declares NO new HTTP route, table, or entity. Every route named (`/sync/v1/elohim/docs`, `/db/content`, `/db/stats`, `/blob/<hash>`, `/p2p/status`) already exists and is only READ by the harness. The one Rust change is a count method. The spec's gate output (§5) covers the arc; nothing here needs a new entry-type answer.
- Commit per task, path-limited (`git add <files>`), never `git add -A`; commit-only — the integrator pushes.
- DoD for a Rust task is `just gate elohim-storage`; for harness tasks it is the task's own test script plus a live run on the mesh.

---

### Task 1: `syncDocuments` counts every namespace (Cut 0)

**Files:**
- Modify: `elohim/elohim-storage/src/sync/mod.rs:281-283` (add `count_all` beside `count_documents`; add test in the existing `mod tests`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:9425` (`refresh_status`)

**Interfaces:**
- Consumes: `DocStore::count_all(&self) -> Result<u64, StorageError>` (exists, `doc_store.rs:284`); test helper `test_sync_manager() -> (SyncManager, TempDir)` (exists, `sync/mod.rs:292`); `SyncManager::apply_changes(ns: &str, doc_id: &str, changes: Vec<Vec<u8>>)` (exists).
- Produces: `SyncManager::count_all(&self) -> Result<u64, StorageError>`; `/p2p/status.syncDocuments` = total docs across all namespaces.

- [ ] **Step 1: Write the failing test** — append inside `mod tests` in `elohim/elohim-storage/src/sync/mod.rs` (after the existing `one_change_by_hash_…` test):

```rust
    /// `/p2p/status.syncDocuments` read 0 on a store holding 5,356 docs
    /// (fleet, 2026-08-24) because `count_documents("_all")` scanned the key
    /// prefix `"_all:"` while every doc lives under `"elohim:"`. The status
    /// projection must count the whole store, and a namespace count must
    /// stay namespace-scoped — both pinned here so neither drifts again.
    #[tokio::test]
    async fn count_all_counts_every_namespace_and_count_documents_stays_scoped() {
        let (sync, _tmp) = test_sync_manager().await;
        for (ns, doc) in [("elohim", "node:a"), ("elohim", "node:b"), ("other", "node:c")] {
            let mut d = Automerge::new();
            d.transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "title", doc)?;
                Ok(())
            })
            .unwrap();
            sync.apply_changes(ns, doc, vec![d.save()]).await.unwrap();
        }
        assert_eq!(sync.count_all().await.unwrap(), 3, "whole store");
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 2, "scoped");
        assert_eq!(sync.count_documents("_all").await.unwrap(), 0, "\"_all\" is a namespace name, not a wildcard — this is the bug's shape");
    }
```

- [ ] **Step 2: Run it and watch it fail to compile** (`count_all` does not exist on `SyncManager`):

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' RUSTC_WRAPPER="" \
cargo test --lib sync::tests::count_all_counts_every_namespace 2>&1 | tail -15; echo "EXIT=${PIPESTATUS[0]}"
```
Expected: `error[E0599]: no method named `count_all` found` and `EXIT=101`.

- [ ] **Step 3: Add `count_all` to `SyncManager`** — in `elohim/elohim-storage/src/sync/mod.rs`, directly after `count_documents` (line 283):

```rust
    /// Every document in the store, across all namespaces. This is what a
    /// "how many sync documents does this peer hold" status field means;
    /// `count_documents` answers the per-app question.
    pub async fn count_all(&self) -> Result<u64, StorageError> {
        self.doc_store.count_all().await
    }
```

- [ ] **Step 4: Point the status projection at it** — `elohim/elohim-storage/src/p2p/mod.rs:9425`, replace

```rust
        let sync_documents = self.sync_manager.count_documents("_all").await.unwrap_or(0) as usize;
```
with
```rust
        // Whole-store count. `count_documents("_all")` scanned the literal
        // "_all:" prefix and read 0 forever (2026-08-24, 5,356 docs on the fleet).
        let sync_documents = self.sync_manager.count_all().await.unwrap_or(0) as usize;
```

- [ ] **Step 5: Run the test, expect PASS**:

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' RUSTC_WRAPPER="" \
cargo test --lib sync::tests::count_all_counts_every_namespace 2>&1 | tail -5; echo "EXIT=${PIPESTATUS[0]}"
```
Expected: `test sync::tests::count_all_counts_every_namespace_and_count_documents_stays_scoped ... ok`, `EXIT=0`.

- [ ] **Step 6: Prove it live on the running mesh** — feature-full build into the slot, restart one peer, read the field:

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' RUSTC_WRAPPER="" \
cargo build --bin elohim-storage --features "p2p p2p-iroh" 2>&1 | tail -3; echo "EXIT=${PIPESTATUS[0]}"
cd /projects/elohim
MESH_RESTART_APPLY_PROFILE=1 STORAGE_BIN=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev/debug/elohim-storage \
  app/elohim-app/scripts/hc-mesh.sh storage-restart matthew
sleep 5; curl -s localhost:8090/p2p/status | python3 -c 'import sys,json;d=json.load(sys.stdin);print("syncDocuments=",d["syncDocuments"])'
curl -s "localhost:8090/sync/v1/elohim/docs?limit=1" | python3 -c 'import sys,json;print("route total=",json.load(sys.stdin)["total"])'
```
Expected: `EXIT=0`; `syncDocuments=` equals `route total=` (≈504 on the current mesh). If they differ, the fix did not reach the slot — re-read the Global Constraints on the binary slot before touching code.

- [ ] **Step 7: Gate and commit**:

```bash
cd /projects/elohim && just gate elohim-storage 2>&1 | tail -8; echo "EXIT=${PIPESTATUS[0]}"
git add elohim/elohim-storage/src/sync/mod.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "storage(sync): /p2p/status.syncDocuments counts the whole store — count_documents(\"_all\") scanned a literal prefix and read 0 forever

Fleet held 5,356 docs (GET /sync/v1/elohim/docs) while the status field read 0;
local mesh 504 vs 0. Adds SyncManager::count_all over DocStore::count_all and
pins both the whole-store and the namespace-scoped counts.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: Per-peer transport in `hc-mesh.sh` (`MESH_PEER_TRANSPORTS`)

**Files:**
- Modify: `app/elohim-app/scripts/hc-mesh.sh` — knob block (after line 136 `export MESH_TRANSPORT_BACKEND`), `assert_storage_transport_capability` call sites (two, inside `storage_restart` ~lines 725 and 750), `restart_env_overlay` (line 833; signature + its call at line 789), storage launch (line 1505), `status_all` (line 592 loop), `mesh_transport_backend_from_status` (line 231)
- Create: `app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh`

**Interfaces:**
- Consumes: `PEERS` array, `http_port <idx>`, `storage_transport_for <name> <port>` (all exist).
- Produces: `peer_transport <name>` → `libp2p|dual|iroh` (per-peer declaration, falls back to `MESH_TRANSPORT_BACKEND`); `restart_env_overlay <captured-environ> <peer-name>`; `mesh_transport_backend_from_status` prints a homogeneous mode OR a `name=mode,name=mode` set (never `unknown` for a readable mixed mesh).

- [ ] **Step 1: Write the failing test** — `app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh`:

```bash
#!/usr/bin/env bash
# Sourced-mode unit test for hc-mesh.sh's per-peer transport knob. Sourcing
# never starts anything (dispatch guard at the bottom of hc-mesh.sh).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

# 1. unset knob → every peer inherits MESH_TRANSPORT_BACKEND
( set +e; MESH_PEERS=matthew,jessica MESH_TRANSPORT_BACKEND=dual MESH_PEER_TRANSPORTS= \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "inherit: matthew=dual"  '[ "$(peer_transport matthew)" = dual ]'
  t "inherit: jessica=dual"  '[ "$(peer_transport jessica)" = dual ]'
  exit $fail ) || fail=1

# 2. partial map → named peer gets its own, the other inherits
( set +e; MESH_PEERS=matthew,jessica MESH_TRANSPORT_BACKEND=dual MESH_PEER_TRANSPORTS="jessica=iroh" \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "map: matthew inherits dual" '[ "$(peer_transport matthew)" = dual ]'
  t "map: jessica=iroh"          '[ "$(peer_transport jessica)" = iroh ]'
  t "overlay carries the PEER mode, not the global" \
    'restart_env_overlay /dev/null jessica | grep -qx "ELOHIM_TRANSPORT_BACKEND=iroh"'
  exit $fail ) || fail=1

# 3. invalid mode is refused at source time
( set +e; MESH_PEERS=matthew,jessica MESH_PEER_TRANSPORTS="jessica=quic" \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1; rc=$?
  t "invalid per-peer mode refused (rc=$rc)" '[ "$rc" -ne 0 ]'
  exit $fail ) || fail=1

exit $fail
```

- [ ] **Step 2: Run it, expect failures** (`peer_transport: command not found`):

```bash
bash /projects/elohim/app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh; echo "EXIT=$?"
```
Expected: `FAIL inherit: matthew=dual` … `EXIT=1`.

- [ ] **Step 3: Add the knob and `peer_transport`** — in `hc-mesh.sh` immediately after `export MESH_TRANSPORT_BACKEND` (line 136):

```bash
# Per-peer transport, the diversity axis of the two-peer recovery harness:
#   MESH_PEER_TRANSPORTS="matthew=libp2p,jessica=iroh"
# Any peer not named inherits MESH_TRANSPORT_BACKEND. The configurations are
# the library; the two slots cycle through it — a scenario never adds a peer.
MESH_PEER_TRANSPORTS="${MESH_PEER_TRANSPORTS:-}"
peer_transport() { # <peer-name> -> libp2p|dual|iroh
  local kv
  IFS=',' read -ra _pt <<< "$MESH_PEER_TRANSPORTS"
  for kv in "${_pt[@]}"; do
    [ "${kv%%=*}" = "$1" ] && { echo "${kv#*=}"; return 0; }
  done
  echo "$MESH_TRANSPORT_BACKEND"
}
_validate_peer_transports() {
  local kv
  IFS=',' read -ra _pt <<< "$MESH_PEER_TRANSPORTS"
  for kv in "${_pt[@]}"; do
    [ -z "$kv" ] && continue
    case "${kv#*=}" in libp2p|dual|iroh) ;; *)
      echo "invalid MESH_PEER_TRANSPORTS entry '$kv' (expected <peer>=libp2p|dual|iroh)" >&2
      return 2 ;;
    esac
  done
}
_validate_peer_transports || { if [ "${BASH_SOURCE[0]}" != "$0" ]; then return 2; else exit 2; fi; }
export MESH_PEER_TRANSPORTS
```

- [ ] **Step 4: Thread the peer name through the restart overlay** — change the signature at line 833 and its first line:

```bash
restart_env_overlay() { # <captured-environ> <peer-name>
  # The caller's PER-PEER transport selection deliberately beats the captured
  # daemon environment. This is how one slot is cycled into a new transport.
  printf '%s\n' "ELOHIM_TRANSPORT_BACKEND=$(peer_transport "$2")"
```
and the call at line 789: `"$(restart_env_overlay "$envfile" "$name")"`.

- [ ] **Step 5: Per-peer capability assert, launch, status, classification** — four edits:
  - both `assert_storage_transport_capability "$bin" "$MESH_TRANSPORT_BACKEND"` inside `storage_restart` → `assert_storage_transport_capability "$bin" "$(peer_transport "$name")"`
  - launch (line 1505): `ELOHIM_TRANSPORT_BACKEND="$MESH_TRANSPORT_BACKEND" \` → `ELOHIM_TRANSPORT_BACKEND="$(peer_transport "$name")" \`, and the echo two lines below: `transport=$(peer_transport "$name")`
  - `mesh_transport_backend_from_status`: replace the final `else echo unknown` branch (the mixed case) with a per-peer listing so a deliberately mixed mesh is named, not shrugged at:

```bash
  else
    # Mixed by design (MESH_PEER_TRANSPORTS): name each peer's live plane.
    local j=0 out="" st
    for _name in "${PEERS[@]}"; do
      st="$(curl -fsS -m 3 "http://localhost:$(http_port "$j")/p2p/status" 2>/dev/null)"
      if jq -e '.irohNodeId | type == "string" and length > 0' >/dev/null 2>&1 <<<"$st"; then out+="${out:+,}$_name=dual"
      elif jq -e '.peerId | type == "string" and test("^[0-9a-fA-F]{64}$")' >/dev/null 2>&1 <<<"$st"; then out+="${out:+,}$_name=iroh"
      else out+="${out:+,}$_name=libp2p"; fi
      j=$((j + 1))
    done
    echo "$out"
  fi
```
  - `status_all` peer line already prints `transport=$transport` from the live environ — no change needed; the launch-side declaration is what moved.

- [ ] **Step 6: Run the test, expect PASS**:

```bash
bash /projects/elohim/app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh; echo "EXIT=$?"
```
Expected: six `ok` lines, `EXIT=0`.

- [ ] **Step 7: Live proof — cycle one slot into iroh and back** (mesh is running in dual):

```bash
cd /projects/elohim
MESH_PEERS=matthew,jessica,james MESH_PEER_TRANSPORTS="jessica=iroh" MESH_RESTART_APPLY_PROFILE=1 \
  app/elohim-app/scripts/hc-mesh.sh storage-restart jessica
sleep 8; MESH_PEERS=matthew,jessica,james bash -c 'source app/elohim-app/scripts/hc-mesh.sh >/dev/null 2>&1; mesh_transport_backend_from_status'
```
Expected: `matthew=dual,jessica=iroh,james=dual`. (Three peers today because the current mesh has three; the harness tasks below run on two.) Then restore: same command with `MESH_PEER_TRANSPORTS=` and confirm `dual`.

- [ ] **Step 8: Commit**:

```bash
git add app/elohim-app/scripts/hc-mesh.sh app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh
git commit -m "mesh: per-peer transport (MESH_PEER_TRANSPORTS) — one slot cycles into a new plane; mixed meshes are named, not 'unknown'

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Footprint line in `hc-mesh.sh status`

**Files:**
- Modify: `app/elohim-app/scripts/hc-mesh.sh` — `status_all`, after the `mongod` line (line ~611) and before the `echo` that precedes the running-binary block
- Modify: `app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh` (one more assertion block)

**Interfaces:**
- Produces: `mesh_footprint` → lines `footprint <role> <name> rss=<MB> cpu=<pct>` and a final `footprint total rss=<MB>`; called from `status_all`.

- [ ] **Step 1: Add the failing assertion** — append to the test before the final `exit $fail`:

```bash
# 4. footprint is a function of what is RUNNING, formatted for grep
( set +e; MESH_PEERS=matthew,jessica source "$here/../hc-mesh.sh" >/dev/null 2>&1
  out="$(mesh_footprint 2>/dev/null)"
  t "footprint prints a total line" 'grep -q "^footprint total rss=[0-9]*MB" <<<"$out"'
  exit $fail ) || fail=1
```
Run: `bash app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh; echo "EXIT=$?"` → expect `FAIL footprint prints a total line`.

- [ ] **Step 2: Implement `mesh_footprint`** — add above `status_all`:

```bash
# What the mesh COSTS, read from ps — so "can we afford this locally" is a
# number in the same command that reports health. Conductors are ~2 GB RSS
# each (measured 2026-08-24: 3 conductors = 6.0 of a 6.9 GB mesh); that is why
# the recovery harness runs two peers, not three.
mesh_footprint() {
  local total=0
  while IFS= read -r line; do
    set -- $line; local pid="$1" rss="$2" cpu="$3"; shift 3
    local role="other" name="-"
    case "$*" in
      *holochain*--config-path*) role=conductor; name="$(sed -n 's#.*/local-dev/\([^/]*\)/.*#\1#p' <<<"$*")" ;;
      *elohim-storage*--http-port*) role=storage; name="$(sed -n 's/.*--http-port \([0-9]*\).*/\1/p' <<<"$*")" ;;
      *"/doorway "*--listen*) role=doorway; name="$(sed -n 's/.*--listen [^:]*:\([0-9]*\).*/\1/p' <<<"$*")" ;;
      *mongod*"$MESH_DIR"*) role=mongod ;;
      *) continue ;;
    esac
    printf 'footprint %-9s %-8s rss=%dMB cpu=%s%%\n' "$role" "$name" $((rss / 1024)) "$cpu"
    total=$((total + rss / 1024))
  done < <(ps -eo pid=,rss=,pcpu=,args= | grep -E "holochain --piped|elohim-storage.*--http-port|/doorway .*--listen|mongod --dbpath" | grep -v grep)
  echo "footprint total rss=${total}MB"
}
```
and in `status_all`, after the mongod line: `echo; mesh_footprint`.

- [ ] **Step 3: Test + live check**:

```bash
bash app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh; echo "EXIT=$?"
app/elohim-app/scripts/hc-mesh.sh status | grep '^footprint'
```
Expected: `EXIT=0`; one `conductor` line per running conductor at ~2000MB, storage ~200MB, doorways ~100MB, a `total`.

- [ ] **Step 4: Commit**:

```bash
git add app/elohim-app/scripts/hc-mesh.sh app/elohim-app/scripts/__tests__/hc-mesh-peer-transport.test.sh
git commit -m "mesh: status prints the mesh footprint (rss/cpu per conductor/storage/doorway + total)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: One reader — `hc-mesh-quiesce.sh` tees gate lines; `quiesce-timeline.py --local`

**Files:**
- Modify: `app/elohim-app/scripts/hc-mesh-quiesce.sh` (the `{ time bash "$GATE_SCRIPT" …; }` line)
- Modify: `genesis/scripts/quiesce-timeline.py` (`parse_quiesce` gains `source`/`labels`; `main()` gains `--local`, `--label`)
- Create: `.claude/scripts/_lib/__tests__/quiesce_timeline_local_test.py`

**Interfaces:**
- Consumes: `parse_quiesce(build: int | None, text: str, build_result: str | None = None) -> dict | None` (exists).
- Produces: records gain `"source": "fleet" | "local"` and `"labels": {…}`; CLI `quiesce-timeline.py --local <gate-log> [--label k=v …] [--record]`; the wrapper writes `$MESH_DIR/quiesce-gate/<start_iso>.log` (full gate stdout) and prints `gate-log: <path>`.

- [ ] **Step 1: Write the failing test** — `.claude/scripts/_lib/__tests__/quiesce_timeline_local_test.py`:

```python
#!/usr/bin/env python3
"""quiesce-timeline.py --local: the local mesh gate's lines parse into the SAME
record shape as a fleet build, tagged source=local, with optional labels."""
import importlib.util, io, os, sys, unittest
from contextlib import redirect_stdout

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
spec = importlib.util.spec_from_file_location("qt", os.path.join(ROOT, "genesis", "scripts", "quiesce-timeline.py"))
qt = importlib.util.module_from_spec(spec); spec.loader.exec_module(qt)

LOG = """fleet-quiesce[2026-08-24T18:00:00Z]: starting — deadline=1200s poll=10s sustain=33s content=elohim-host-landing
fleet-quiesce[2026-08-24T18:00:00Z]: FAIL A-caughtUp=False B-caughtUp=True A-quiesced=False(actionable=None) — (A-not-caughtUp A-not-quiesced)
fleet-quiesce[2026-08-24T18:00:10Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 0s, need 33s
fleet-quiesce[2026-08-24T18:00:40Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 30s, need 33s
fleet-quiesce[2026-08-24T18:00:50Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True — sustained 40s, need 33s
FLEET QUIESCENT (A-QUIESCED; B excluded from predicate)
"""

class LocalParse(unittest.TestCase):
    def test_local_log_parses_as_measured_with_source_and_labels(self):
        rec = qt.parse_quiesce(None, LOG, None, source="local", labels={"scenario": "homo-dual", "shape": "warm"})
        self.assertEqual(rec["outcome"], "measured")
        self.assertEqual(rec["source"], "local")
        self.assertEqual(rec["labels"], {"scenario": "homo-dual", "shape": "warm"})
        self.assertEqual(rec["best_window_s"], 40)
        self.assertEqual(rec["time_to_verdict_s"], 50)
        self.assertEqual(rec["blocking_legs"], {"A-not-caughtUp": 1, "A-not-quiesced": 1})
        self.assertIsNone(rec["build"])

    def test_fleet_default_source(self):
        rec = qt.parse_quiesce(1379, LOG, "SUCCESS")
        self.assertEqual(rec["source"], "fleet")
        self.assertEqual(rec["labels"], {})

    def test_cli_local_prints_a_record(self):
        import tempfile
        with tempfile.NamedTemporaryFile("w", suffix=".log", delete=False) as f:
            f.write(LOG); path = f.name
        out = io.StringIO()
        with redirect_stdout(out):
            rc = qt.main(["--local", path, "--label", "scenario=homo-dual"])
        self.assertEqual(rc, 0)
        self.assertIn("MEASURED", out.getvalue())
        self.assertIn("homo-dual", out.getvalue())

if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run, expect failure** (`parse_quiesce() got an unexpected keyword argument 'source'`):

```bash
python3 /projects/elohim/.claude/scripts/_lib/__tests__/quiesce_timeline_local_test.py 2>&1 | tail -6; echo "EXIT=${PIPESTATUS[0]}"
```

- [ ] **Step 3: Extend the parser** — in `genesis/scripts/quiesce-timeline.py`:
  - signature: `def parse_quiesce(build: int | None, text: str, build_result: str | None = None, *, source: str = "fleet", labels: dict | None = None) -> dict | None:`
  - the returned dict gains two keys after `"build": build,`: `"source": source, "labels": dict(labels or {}),`
  - `main(argv=None)`: change `def main() -> int:` to `def main(argv: list[str] | None = None) -> int:` and `args = ap.parse_args(argv)`; add
    ```python
    ap.add_argument("--local", help="parse a local mesh gate log (hc-mesh-quiesce.sh writes $MESH_DIR/quiesce-gate/<start>.log)")
    ap.add_argument("--label", action="append", default=[], help="k=v label attached to a --local record (scenario=, shape=, run=)")
    ```
    and, before the existing `--build/--builds` handling:
    ```python
    if args.local:
        labels = dict(kv.split("=", 1) for kv in args.label if "=" in kv)
        with open(args.local, encoding="utf-8", errors="replace") as fh:
            rec = parse_quiesce(None, fh.read(), None, source="local", labels=labels)
        if rec is None:
            print(f"{args.local}: no fleet-quiesce lines found", file=sys.stderr); return 2
        render([rec])
        if args.record:
            SERIES.parent.mkdir(parents=True, exist_ok=True)
            with SERIES.open("a") as out: out.write(json.dumps(rec) + "\n")
        return 0
    ```
  - `render`: where it prints the build column, print `rec.get("build") or rec.get("source")` and, if `rec.get("labels")`, append `" " + " ".join(f"{k}={v}" for k, v in rec["labels"].items())` to the line. Read `render` before editing so the fleet column layout is unchanged for fleet records.

- [ ] **Step 4: Tee the gate output in the wrapper** — `app/elohim-app/scripts/hc-mesh-quiesce.sh`, replace

```bash
{ time bash "$GATE_SCRIPT" "$DOORWAY_A" "$DOORWAY_B" "$CONTENT_ID" "$STORAGE_A" "$STORAGE_B"; } 2> "$TIME_FILE"
gate_exit=$?
```
with
```bash
mkdir -p "$MESH_DIR/quiesce-gate"
GATE_LOG="$MESH_DIR/quiesce-gate/${start_iso//:/-}.log"
{ time bash "$GATE_SCRIPT" "$DOORWAY_A" "$DOORWAY_B" "$CONTENT_ID" "$STORAGE_A" "$STORAGE_B" | tee "$GATE_LOG"; } 2> "$TIME_FILE"
gate_exit=${PIPESTATUS[0]}
echo "gate-log: $GATE_LOG"
```
(`PIPESTATUS[0]` because `tee` would otherwise mask the gate's exit 3.)

- [ ] **Step 5: Tests, then a live parse of a real local gate log**:

```bash
python3 /projects/elohim/.claude/scripts/_lib/__tests__/quiesce_timeline_local_test.py 2>&1 | tail -4; echo "EXIT=${PIPESTATUS[0]}"
cd /projects/elohim && QUIESCE_DEADLINE_SECS=300 app/elohim-app/scripts/hc-mesh-quiesce.sh | tail -3
python3 genesis/scripts/quiesce-timeline.py --local "$(ls -t /tmp/elohim-local-mesh/quiesce-gate/*.log | head -1)" --label scenario=smoke
```
Expected: `EXIT=0`; the wrapper prints `gate-log: …`; the reader renders one `local` record (`MEASURED` if the mesh quiesced within 300s, else `no-measure` with `best_window_s` — both are valid outputs; a crash is not).

- [ ] **Step 6: Commit**:

```bash
git add app/elohim-app/scripts/hc-mesh-quiesce.sh genesis/scripts/quiesce-timeline.py .claude/scripts/_lib/__tests__/quiesce_timeline_local_test.py
git commit -m "quiesce: one reader for two emitters — local gate log tee'd to a file, quiesce-timeline.py --local parses it into the fleet series shape (source=local, labels)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: The recovery primitive — `hc-mesh-recovery.sh <warm|cold> <peer>`

**Files:**
- Create: `app/elohim-app/scripts/hc-mesh-recovery.sh`
- Create: `app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh`
- Modify: `justfile` (`mesh` recipe, line 203–210: add `recovery`)

**Interfaces:**
- Consumes (sourced from `hc-mesh.sh`): `PEERS`, `http_port <idx>`, `storage_pid_for_port <port>`, `storage_restart <peer>` (the function behind `storage-restart`), `peer_transport <name>`, `MESH_DIR`, `DOORWAY_PORT`, `DOORWAY_B_PORT`.
- Produces: `recovery_snapshot <survivor-port> > <file>` (JSON: `{"docs":N,"content":N,"rows":[{"id":..,"blobHash":..},…]}`); `recovery_predicate <snapshot-file> <recovering-port>` → prints `P0=<0|1> P1=<0|1> P2=<0|1> P3=<0|1> P4=<0|1>` and returns 0 iff all 1; `receipt_max <peer-log> <since-epoch>` → max `recv_validation_receipt_received elapsed_s` since the epoch; one line per poll `recovery[<ts>]: PASS|FAIL <legs> — elapsed Ns`; final `RECOVERED in Ns` or `NOT-RECOVERED after Ns (<failing legs>)`; one JSON record appended to `$MESH_DIR/recovery-timeline.jsonl` with keys `ts, shape, peer, survivor, transport_survivor, transport_recovering, recovered (bool), time_to_recover_s, polls, failing_legs, labels, conductor_receipt_max_s {recovering, survivor}`.

- [ ] **Step 1: Write the failing test** — `app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh` (tests the pure functions against a fake HTTP server; no mesh needed):

```bash
#!/usr/bin/env bash
# Unit test for hc-mesh-recovery.sh's snapshot + predicate against a stub
# storage peer served by python's http.server on a free port.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0; t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"; kill $srv 2>/dev/null' EXIT
mkdir -p "$tmp/db/content" "$tmp/sync/v1/elohim" "$tmp/blob" "$tmp/p2p"
echo '{"items":[{"id":"a","blobHash":"sha256-aa"},{"id":"b"}]}' > "$tmp/db/content/index.html"
echo '{"id":"a","blobHash":"sha256-aa"}' > "$tmp/db/content/a"
echo '{"contentCount":2}' > "$tmp/db/stats"
echo '{"total":2}' > "$tmp/sync/v1/elohim/docs"
echo 'bytes' > "$tmp/blob/sha256-aa"
echo '{"pull":{"caughtUp":true,"failed":0}}' > "$tmp/p2p/status"
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("",0));print(s.getsockname()[1])')
( cd "$tmp" && python3 -m http.server "$port" >/dev/null 2>&1 ) & srv=$!
sleep 1
set +e; RECOVERY_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery.sh"; set -e 2>/dev/null; set +e
recovery_snapshot "$port" > "$tmp/snap.json"
t "snapshot captured 1 blob-bearing row" '[ "$(python3 -c "import json;print(len(json.load(open(\"$tmp/snap.json\"))[\"rows\"]))")" = 1 ]'
RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
  out="$(recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
t "all legs pass on a matching peer (rc=$rc): $out" '[ "$rc" -eq 0 ] && [ "$out" = "P0=1 P1=1 P2=1 P3=1 P4=1" ]'
rm "$tmp/blob/sha256-aa"
RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
  out="$(recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
t "missing blob bytes fails ONLY P2 (rc=$rc): $out" '[ "$rc" -ne 0 ] && [ "$out" = "P0=1 P1=1 P2=0 P3=1 P4=1" ]'
exit $fail
```

- [ ] **Step 2: Run, expect failure** (script missing):

```bash
bash /projects/elohim/app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh; echo "EXIT=$?"
```

- [ ] **Step 3: Write `hc-mesh-recovery.sh`**:

```bash
#!/usr/bin/env bash
# hc-mesh-recovery.sh — the churn primitive of the two-peer harness.
#
#   hc-mesh-recovery.sh <warm|cold> <recovering-peer> [--label k=v ...]
#
# One peer holds the other's full recoverable state. This script INFLICTS the
# loss and TIMES the recovery from the survivor, then writes one record. It is
# the resiliency saga's own demand (ch.1-2 awaken/form, 5-7 co-steward/converge/
# custody, 11 pull queue finishes), read from the recovering peer's HTTP
# surface only — never a filesystem diff (blob layouts differ per transport).
#
#   warm   stop the peer's storage; wipe DocStore + content db + blobs + caches;
#          keep identity.key/iroh.key (same transport identity, same agent).
#   cold   warm + identity.key + iroh.key: a NEW libp2p PeerId and iroh NodeId.
#          Declared limit: the conductor agent key survives (sandboxes are not
#          regenerated here) — cold join is a new TRANSPORT identity, not yet a
#          new agent. Regenerating one sandbox is a separate row.
#
# Backpressure witness (spec §3.5): the SURVIVOR's conductor is the one being
# hammered while the loser re-acquires. Both peers' max
# `recv_validation_receipt_received elapsed_s` during the window is recorded —
# a sync that destabilises a conductor shows up here as seconds, per transport.
#
# "Recovered" = five legs, all true (spec §3.3):
#   P0 /sync/v1/elohim/docs total == survivor's
#   P1 every survivor content row with a blobHash: /db/content/<id> 200 + equal blobHash
#   P2 GET /blob/<hash> 200 on the recovering peer for each of those hashes
#   P3 /p2p/status pull.caughtUp == true && pull.failed == 0
#   P4 doorway A and B both 200 on /db/content/elohim-host-landing
#
# Sourceable with RECOVERY_SOURCE_ONLY=1 (unit tests use the functions).
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECOVERY_DEADLINE_SECS="${RECOVERY_DEADLINE_SECS:-900}"
RECOVERY_POLL_SECS="${RECOVERY_POLL_SECS:-5}"
RECOVERY_DOORWAY_A="${RECOVERY_DOORWAY_A:-http://localhost:${DOORWAY_PORT:-8888}}"
RECOVERY_DOORWAY_B="${RECOVERY_DOORWAY_B:-http://localhost:${DOORWAY_B_PORT:-8889}}"
RECOVERY_LANDING_PATH="${RECOVERY_LANDING_PATH:-/db/content/elohim-host-landing}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"

rlog() { printf 'recovery[%s]: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1"; }

recovery_snapshot() { # <survivor-http-port> -> JSON on stdout
  local port="$1"
  python3 - "$port" <<'PY'
import json, sys, urllib.request
port = sys.argv[1]
def get(p):
    with urllib.request.urlopen(f"http://localhost:{port}{p}", timeout=20) as r: return json.load(r)
items = get("/db/content?limit=500").get("items", [])
print(json.dumps({
  "docs": get("/sync/v1/elohim/docs?limit=1").get("total", 0),
  "content": get("/db/stats").get("contentCount", 0),
  "rows": [{"id": i["id"], "blobHash": i["blobHash"]} for i in items if i.get("blobHash")],
}))
PY
}

recovery_predicate() { # <snapshot-file> <recovering-http-port> -> "P0=.. P1=.. P2=.. P3=.. P4=.."; rc 0 iff all 1
  local snap="$1" port="$2"
  python3 - "$snap" "$port" "$RECOVERY_DOORWAY_A" "$RECOVERY_DOORWAY_B" "$RECOVERY_LANDING_PATH" <<'PY'
import json, sys, urllib.request, urllib.error
snap, port, dwa, dwb, landing = sys.argv[1:6]
s = json.load(open(snap))
def code(url):
    try:
        with urllib.request.urlopen(url, timeout=15) as r: return r.status, r
    except urllib.error.HTTPError as e: return e.code, None
    except Exception: return 0, None
def getj(p):
    c, r = code(f"http://localhost:{port}{p}")
    return json.load(r) if c == 200 and r else None
docs = getj("/sync/v1/elohim/docs?limit=1")
p0 = int(bool(docs) and docs.get("total") == s["docs"])
p1 = 1; p2 = 1
for row in s["rows"]:
    j = getj(f"/db/content/{row['id']}")
    if not j or j.get("blobHash") != row["blobHash"]: p1 = 0
    if code(f"http://localhost:{port}/blob/{row['blobHash']}")[0] != 200: p2 = 0
st = getj("/p2p/status") or {}
pull = st.get("pull") or {}
p3 = int(pull.get("caughtUp") is True and pull.get("failed", 1) == 0)
p4 = int(code(dwa + landing)[0] == 200 and code(dwb + landing)[0] == 200)
print(f"P0={p0} P1={p1} P2={p2} P3={p3} P4={p4}")
sys.exit(0 if p0 and p1 and p2 and p3 and p4 else 1)
PY
}

if [ "${RECOVERY_SOURCE_ONLY:-0}" = "1" ]; then return 0 2>/dev/null || exit 0; fi

# ---- main -------------------------------------------------------------------
shape="${1:-}"; peer="${2:-}"; shift 2 2>/dev/null || true
case "$shape" in warm|cold) ;; *) echo "usage: $0 <warm|cold> <recovering-peer> [--label k=v ...]" >&2; exit 2 ;; esac
labels="{}"
while [ $# -gt 0 ]; do
  case "$1" in --label) labels="$(python3 -c 'import json,sys;d=json.loads(sys.argv[1]);k,v=sys.argv[2].split("=",1);d[k]=v;print(json.dumps(d))' "$labels" "$2")"; shift 2 ;;
  *) echo "unknown arg $1" >&2; exit 2 ;; esac
done

set +e
# shellcheck source=hc-mesh.sh
source "$SCRIPT_DIR/hc-mesh.sh" >/dev/null 2>&1
set -u
idx=-1; i=0; for n in "${PEERS[@]}"; do [ "$n" = "$peer" ] && idx=$i; i=$((i+1)); done
[ "$idx" -ge 0 ] || { echo "$peer is not in MESH_PEERS=$MESH_PEERS" >&2; exit 2; }
[ "${#PEERS[@]}" -eq 2 ] || { echo "recovery harness runs on exactly two peers (MESH_PEERS=$MESH_PEERS)" >&2; exit 2; }
sidx=$((1 - idx)); survivor="${PEERS[$sidx]}"
rport="$(http_port "$idx")"; sport="$(http_port "$sidx")"
t_surv="$(storage_transport_for "$survivor" "$sport")"; t_rec="$(peer_transport "$peer")"

snap="$(mktemp)"; recovery_snapshot "$sport" > "$snap" || { echo "survivor $survivor:$sport unreadable" >&2; exit 3; }
rlog "shape=$shape peer=$peer survivor=$survivor rows=$(python3 -c 'import json,sys;print(len(json.load(open(sys.argv[1]))["rows"]))' "$snap") transports survivor=$t_surv recovering=$t_rec"

pid="$(storage_pid_for_port "$rport")"
[ -n "$pid" ] && { kill "$pid"; for _ in $(seq 1 15); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done; kill -0 "$pid" 2>/dev/null && kill -9 "$pid"; }
wipe=(sync.sled content.db content.db-shm content.db-wal graph.db blobs blobs_iroh cache contest-backoff.json)
[ "$shape" = cold ] && wipe+=(identity.key iroh.key)
for f in "${wipe[@]}"; do rm -rf "${MESH_DIR:?}/$peer/$f"; done
rlog "loss inflicted: $shape wipe of ${#wipe[@]} entries under $MESH_DIR/$peer"

MESH_RESTART_APPLY_PROFILE=1 storage_restart "$peer" >/dev/null 2>&1 || rlog "storage_restart reported non-zero; polling anyway"
t0=$(date +%s); until curl -sf -m 2 "http://localhost:$rport/health" >/dev/null; do sleep 1; [ $(( $(date +%s) - t0 )) -gt 120 ] && { echo "$peer never served /health" >&2; exit 4; }; done
t0=$(date +%s); polls=0; legs=""; recovered=0
while :; do
  legs="$(recovery_predicate "$snap" "$rport")"; rc=$?; polls=$((polls+1)); el=$(( $(date +%s) - t0 ))
  if [ "$rc" -eq 0 ]; then rlog "PASS $legs — elapsed ${el}s"; recovered=1; break; else rlog "FAIL $legs — elapsed ${el}s"; fi
  [ "$el" -ge "$RECOVERY_DEADLINE_SECS" ] && break
  sleep "$RECOVERY_POLL_SECS"
done
failing="$(tr ' ' '\n' <<<"$legs" | grep '=0$' | cut -d= -f1 | paste -sd, -)"
if [ "$recovered" -eq 1 ]; then echo "RECOVERED in ${el}s"; else echo "NOT-RECOVERED after ${el}s ($failing)"; fi
# Backpressure witness: max conductor receipt latency logged by each peer since t0.
receipt_max() { # <peer-log> <since-epoch> -> max elapsed_s (0 if none)
  python3 - "$1" "$2" <<'PY2'
import re, sys, datetime
log, since = sys.argv[1], int(sys.argv[2]); mx = 0.0
pat = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)[^\n]*elapsed_s[^0-9]*([0-9.]+)[^\n]*recv_validation_receipt_received')
for line in open(log, errors="replace"):
    m = pat.search(line)
    if not m: continue
    ts = datetime.datetime.strptime(m.group(1), "%Y-%m-%dT%H:%M:%S").replace(tzinfo=datetime.timezone.utc).timestamp()
    if ts >= since: mx = max(mx, float(m.group(2)))
print(f"{mx:.1f}")
PY2
}
rcpt_rec="$(receipt_max "$LOGDIR/$peer.log" "$t0")"; rcpt_surv="$(receipt_max "$LOGDIR/$survivor.log" "$t0")"
rlog "conductor receipt latency max during recovery: recovering=${rcpt_rec}s survivor=${rcpt_surv}s"
python3 - "$MESH_DIR/recovery-timeline.jsonl" "$shape" "$peer" "$survivor" "$t_surv" "$t_rec" "$recovered" "$el" "$polls" "$failing" "$labels" "$rcpt_rec" "$rcpt_surv" <<'PY'
import json, sys, datetime
p, shape, peer, surv, ts, tr, rec, el, polls, failing, labels, rr, rs = sys.argv[1:14]
json.dump({"ts": datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"), "shape": shape, "peer": peer, "survivor": surv,
           "transport_survivor": ts, "transport_recovering": tr, "recovered": rec == "1", "time_to_recover_s": int(el),
           "polls": int(polls), "failing_legs": [x for x in failing.split(",") if x], "labels": json.loads(labels),
           "conductor_receipt_max_s": {"recovering": float(rr), "survivor": float(rs)}}, open(p, "a"))
open(p, "a").write("\n")
PY
rm -f "$snap"
[ "$recovered" -eq 1 ]
```

- [ ] **Step 4: `just mesh recovery`** — in `justfile` line 207 case, add before `*)`:

```
      recovery) shift; exec "{{ app_dir }}/scripts/hc-mesh-recovery.sh" "$@" ;;
```
(check how the recipe passes extra args — if `{{ action }}` is the only parameter, add a second recipe parameter `+args=""` and pass `{{ args }}`; match the file's existing style for `test target scope`).

- [ ] **Step 5: Unit test, then a live warm recovery on the two-peer mesh** — the current mesh is three peers; bring it to two first (this is the ONE full restart in the plan — do it foreground, tolerate exit 144 from `stop`):

```bash
bash /projects/elohim/app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh; echo "EXIT=$?"
cd /projects/elohim && app/elohim-app/scripts/hc-mesh.sh stop; echo "stop rc=$? (144 = own process group, fine)"
MESH_PEERS=matthew,jessica app/elohim-app/scripts/hc-mesh.sh start && MESH_PEERS=matthew,jessica app/elohim-app/scripts/hc-mesh.sh prologue
MESH_PEERS=matthew,jessica app/elohim-app/scripts/hc-mesh.sh status | grep -E 'storage=|^footprint total'
MESH_PEERS=matthew,jessica app/elohim-app/scripts/hc-mesh-recovery.sh warm jessica --label scenario=homo-dual --label run=smoke; echo "EXIT=$?"
tail -1 /tmp/elohim-local-mesh/recovery-timeline.jsonl
```
Expected: unit `EXIT=0`; status shows two `UP` storages and `footprint total` ≈ 4.5 GB; recovery prints `FAIL` polls then `RECOVERED in Ns` (`EXIT=0`) and one JSONL record with `"recovered": true`. If it ends `NOT-RECOVERED (P2)`, the survivor is not serving blob bytes to the recoverer — that is a real finding for the spec's §3.2 verdict surface; record it, do not loosen P2.

- [ ] **Step 6: Commit**:

```bash
git add app/elohim-app/scripts/hc-mesh-recovery.sh app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh justfile
git commit -m "mesh: recovery primitive — inflict peer loss (warm/cold), time recovery from the survivor against the saga's own predicates, one JSONL record

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: The scenario library and the cycling matrix — `hc-mesh-recovery-matrix.sh`

**Files:**
- Create: `app/elohim-app/scripts/mesh-recovery-scenarios.tsv`
- Create: `app/elohim-app/scripts/hc-mesh-recovery-matrix.sh`
- Modify: `justfile` (`mesh` recipe: add `recovery-matrix`)

**Interfaces:**
- Consumes: `hc-mesh-recovery.sh <shape> <peer> --label …` (Task 5); `hc-mesh.sh storage-restart <peer>` with `MESH_PEER_TRANSPORTS` (Task 2); `quiesce-timeline.py --local` (Task 4, optional per run via `MESH_RECOVERY_QUIESCE=1`).
- Produces: `$MESH_DIR/recovery-timeline.jsonl` records labelled `scenario=<name> shape=<warm|cold> run=<n> expect=<recover|no-shared-transport>`; a rendered table on stdout; exit 0 iff every run matched its scenario's expectation.

- [ ] **Step 1: The library** — `app/elohim-app/scripts/mesh-recovery-scenarios.tsv` (tab-separated; `#` comments):

```
# scenario	survivor	recovering	expect
# The configurations are the library; the two slots cycle through it.
homo-libp2p	libp2p	libp2p	recover
homo-iroh	iroh	iroh	recover
homo-dual	dual	dual	recover
mixed-dual-libp2p	dual	libp2p	recover
mixed-dual-iroh	dual	iroh	recover
split-libp2p-iroh	libp2p	iroh	no-shared-transport
```

- [ ] **Step 2: Write the failing test** — append to `app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh` before `exit $fail`:

```bash
# matrix: library parses and role alternation is deterministic
( set +e; RECOVERY_MATRIX_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery-matrix.sh"
  t "library has 6 scenarios" '[ "$(matrix_scenarios | wc -l)" -eq 6 ]'
  t "run 1 recovers peer[1], run 2 recovers peer[0]" '[ "$(matrix_recovering_index 1)" = 1 ] && [ "$(matrix_recovering_index 2)" = 0 ]'
  exit $fail ) || fail=1
```
Run it → expect `FAIL library has 6 scenarios`.

- [ ] **Step 3: Write the matrix**:

```bash
#!/usr/bin/env bash
# hc-mesh-recovery-matrix.sh — cycle the two slots through the scenario
# library × recovery shapes × N runs, alternating survivor/recovering roles.
#
#   MESH_RECOVERY_RUNS=3 MESH_RECOVERY_SHAPES=warm,cold MESH_RECOVERY_SCENARIOS=homo-dual,split-libp2p-iroh \
#   hc-mesh-recovery-matrix.sh
#
# A scenario whose `expect` is no-shared-transport PASSES when recovery FAILS
# with P0..P2 all 0 (nothing crossed) — the honest red the spec demands.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="${MESH_RECOVERY_LIBRARY:-$SCRIPT_DIR/mesh-recovery-scenarios.tsv}"
RUNS="${MESH_RECOVERY_RUNS:-3}"
IFS=',' read -ra SHAPES <<< "${MESH_RECOVERY_SHAPES:-warm,cold}"
ONLY="${MESH_RECOVERY_SCENARIOS:-}"

matrix_scenarios() { grep -v '^#' "$LIB" | grep -v '^\s*$' | { if [ -n "$ONLY" ]; then grep -E "^(${ONLY//,/|})	"; else cat; fi; }; }
matrix_recovering_index() { echo $(( $1 % 2 )); }   # run 1 → slot 1, run 2 → slot 0, …

if [ "${RECOVERY_MATRIX_SOURCE_ONLY:-0}" = "1" ]; then return 0 2>/dev/null || exit 0; fi

set +e; source "$SCRIPT_DIR/hc-mesh.sh" >/dev/null 2>&1; set -u
[ "${#PEERS[@]}" -eq 2 ] || { echo "recovery matrix runs on exactly two peers (MESH_PEERS=$MESH_PEERS)" >&2; exit 2; }
rc=0; rows=()
while IFS=$'\t' read -r name t_surv t_rec expect; do
  for shape in "${SHAPES[@]}"; do
    for run in $(seq 1 "$RUNS"); do
      ri="$(matrix_recovering_index "$run")"; si=$((1 - ri))
      rec="${PEERS[$ri]}"; surv="${PEERS[$si]}"
      export MESH_PEER_TRANSPORTS="$surv=$t_surv,$rec=$t_rec"
      echo; echo "=== $name · $shape · run $run · survivor=$surv($t_surv) recovering=$rec($t_rec) ==="
      # put the SURVIVOR into its declared plane first (it must be serving when the loser returns)
      MESH_RESTART_APPLY_PROFILE=1 "$SCRIPT_DIR/hc-mesh.sh" storage-restart "$surv" >/dev/null 2>&1 || echo "  survivor restart non-zero; continuing"
      sleep 10
      "$SCRIPT_DIR/hc-mesh-recovery.sh" "$shape" "$rec" --label "scenario=$name" --label "shape=$shape" --label "run=$run" --label "expect=$expect"
      got=$?
      last="$(tail -1 "$MESH_DIR/recovery-timeline.jsonl")"
      legs="$(python3 -c 'import json,sys;print(",".join(json.loads(sys.argv[1])["failing_legs"]))' "$last")"
      secs="$(python3 -c 'import json,sys;print(json.loads(sys.argv[1])["time_to_recover_s"])' "$last")"
      case "$expect" in
        recover)             verdict=$([ "$got" -eq 0 ] && echo PASS || echo FAIL) ;;
        no-shared-transport) verdict=$([ "$got" -ne 0 ] && [[ "$legs" == *P0* && "$legs" == *P1* && "$legs" == *P2* ]] && echo "PASS(expected-red)" || echo FAIL) ;;
      esac
      [[ "$verdict" == PASS* ]] || rc=1
      rows+=("$name	$shape	$run	$surv=$t_surv	$rec=$t_rec	${secs}s	$legs	$verdict")
      if [ "${MESH_RECOVERY_QUIESCE:-0}" = "1" ]; then
        "$SCRIPT_DIR/hc-mesh-quiesce.sh" >/dev/null 2>&1
        python3 "$REPO_ROOT/genesis/scripts/quiesce-timeline.py" --local "$(ls -t "$MESH_DIR"/quiesce-gate/*.log | head -1)" \
          --label "scenario=$name" --label "shape=$shape" --label "run=$run" --record >/dev/null
      fi
    done
  done
done < <(matrix_scenarios)
echo; printf 'scenario\tshape\trun\tsurvivor\trecovering\tt_recover\tfailing\tverdict\n'; printf '%s\n' "${rows[@]}"
exit $rc
```
and `justfile`: `recovery-matrix) exec "{{ app_dir }}/scripts/hc-mesh-recovery-matrix.sh" ;;`.

- [ ] **Step 4: Unit test, then one cheap live cycle** (one scenario, one shape, one run — the honest red):

```bash
bash /projects/elohim/app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh; echo "EXIT=$?"
cd /projects/elohim && MESH_PEERS=matthew,jessica MESH_RECOVERY_RUNS=1 MESH_RECOVERY_SHAPES=warm MESH_RECOVERY_SCENARIOS=split-libp2p-iroh RECOVERY_DEADLINE_SECS=180 \
  app/elohim-app/scripts/hc-mesh-recovery-matrix.sh | tail -4; echo "EXIT=${PIPESTATUS[0]}"
```
Expected: unit `EXIT=0`; the table's last row ends `PASS(expected-red)` with `failing` containing `P0,P1,P2`; `EXIT=0`. If the split pair *recovers*, that is a finding (some plane bridged them — most likely the doorway manifest board bootstrapping iroh) and the scenario's `expect` is wrong, not the harness: record it in the spec's §7 and stop.

- [ ] **Step 5: Commit**:

```bash
git add app/elohim-app/scripts/mesh-recovery-scenarios.tsv app/elohim-app/scripts/hc-mesh-recovery-matrix.sh app/elohim-app/scripts/__tests__/hc-mesh-recovery.test.sh justfile
git commit -m "mesh: recovery matrix — cycle two slots through the scenario library × shapes × runs, roles alternating, expected-red for the split pair

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: The baseline series and the habit delta

**Files:**
- Create: `genesis/a2o/reports/recovery-baseline/2026-08-24-baseline.md` (rendered table — the deliverable)
- Modify: `genesis/manifests/habits.yaml` (`dataplane-convergence` delta line)

**Interfaces:**
- Consumes: `$MESH_DIR/recovery-timeline.jsonl` (Task 5/6); `.claude/data/quiesce-timeline.jsonl` local records (Task 4).
- Produces: the baseline table Plan 2's before/after compares against.

- [ ] **Step 1: Run the full baseline** (6 scenarios × 2 shapes × 3 runs = 36 recoveries; budget ~2–4 h at the dev-tier pacing profile — run in the background, foreground-check with `tail`):

```bash
cd /projects/elohim && MESH_PEERS=matthew,jessica MESH_RECOVERY_QUIESCE=1 \
  nohup app/elohim-app/scripts/hc-mesh-recovery-matrix.sh > /tmp/elohim-local-mesh/recovery-baseline.log 2>&1 &
# later:
tail -3 /tmp/elohim-local-mesh/recovery-baseline.log; grep -c '"recovered"' /tmp/elohim-local-mesh/recovery-timeline.jsonl
```

- [ ] **Step 2: Render the table**:

```bash
python3 - <<'PY' > genesis/a2o/reports/recovery-baseline/2026-08-24-baseline.md
import json, statistics as st, collections
rs=[json.loads(l) for l in open("/tmp/elohim-local-mesh/recovery-timeline.jsonl") if l.strip()]
g=collections.defaultdict(list)
for r in rs: g[(r["labels"].get("scenario"), r["shape"])].append(r)
print("# Two-peer recovery baseline — 2026-08-24 (transport selection OFF; this is what Plan 2 must beat)\n")
print("| scenario | shape | runs | recovered | t_recover median s | min | max | survivor conductor receipt max s | failing legs seen |\n|---|---|---|---|---|---|---|---|---|")
for (sc,sh),v in sorted(g.items()):
    ok=[r for r in v if r["recovered"]]; t=[r["time_to_recover_s"] for r in ok]
    rc=[r.get("conductor_receipt_max_s",{}).get("survivor",0) for r in v]
    legs=sorted({l for r in v for l in r["failing_legs"]})
    print(f"| {sc} | {sh} | {len(v)} | {len(ok)} | {int(st.median(t)) if t else '—'} | {min(t) if t else '—'} | {max(t) if t else '—'} | {max(rc) if rc else '—'} | {','.join(legs) or '—'} |")
PY
cat genesis/a2o/reports/recovery-baseline/2026-08-24-baseline.md
```

- [ ] **Step 3: One-line delta in `habits.yaml`** under `dataplane-convergence` (match the file's existing `deltas:` list style — read the habit's block first):

```yaml
      - "2026-08-24b: two-peer recovery harness landed (warm/cold × 6 transport pairs, saga-predicate 'recovered'); baseline t_recover recorded genesis/a2o/reports/recovery-baseline/2026-08-24-baseline.md; syncDocuments honesty fix live; status stays red (fleet inventory-convergence:42 still red)"
```

- [ ] **Step 4: Commit**:

```bash
git add genesis/a2o/reports/recovery-baseline/2026-08-24-baseline.md genesis/manifests/habits.yaml
git commit -m "habits: dataplane-convergence delta — two-peer recovery baseline recorded (selection OFF), the number Plan 2 has to beat

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Self-review

**Spec coverage.** §3.0 → Task 1. §3.3 topology/per-peer transport/footprint → Tasks 2–3. §3.3 recovery primitive + six scenarios + role swap → Tasks 5–6. §3.4 one reader, two emitters → Task 4; proof shape (baseline half) → Task 7. §3.1–3.2 (`PathObservation`, `select_path`, `syncVerdicts`) and the flag-on half of §3.4 plus fleet confirmation (§7 rows 1–5, 9-after, 10) are **Plan 2**, deliberately after this baseline. §2 non-goals honoured: no new process, no notarized metric, no sandbox regeneration (declared limit in Task 5's header — cold join is a new transport identity, not a new agent; a "regenerate one sandbox" row belongs to Plan 2 or a later plan if the saga's ch.1 proof needs a new agent key).

**Placeholders.** None: every step has its code or exact command; expected outputs are stated; the two "if this happens it is a finding" branches (Task 5 P2, Task 6 split pair) say what to record and to stop, not "handle it".

**Type/name consistency.** `peer_transport` (Task 2) is what Tasks 5–6 call; `restart_env_overlay <environ> <name>` matches its single call site; `recovery_snapshot`/`recovery_predicate` names and the `P0..P4` line format are identical in Task 5's script and test; `matrix_scenarios`/`matrix_recovering_index` match test and script; the JSONL keys written in Task 5 are the keys read in Tasks 6–7; `parse_quiesce(..., source=, labels=)` matches the test and the CLI; `count_all` on `SyncManager` matches the test and the `refresh_status` call.

**Known soft spots (named, not hidden).** Task 5's snapshot reads `/db/content?limit=500` — the local corpus is 456 rows; if the corpus grows past 500 the snapshot must page (add `offset` — same shape as the sync driver's `next_doc_list_offset`). The matrix assumes `hc-mesh.sh storage-restart` returns after the peer serves `/health` (it does — 180 s wait loop); it does not wait for re-mesh, which is why Task 6 sleeps 10 s after the survivor restart and why the recovering peer's poll loop, not the restart, is the timer.
