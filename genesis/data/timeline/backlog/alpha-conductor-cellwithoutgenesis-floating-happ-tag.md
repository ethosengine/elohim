---
id: "backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alpha conductors crashloop CellWithoutGenesis — floating happ:dev-latest DNA drift installs a genesis-less cell, and the conductor panics BEFORE the storage self-heal can reinstall it"
slug: "alpha-conductor-cellwithoutgenesis-floating-happ-tag"
written: "2026-06-22"
author: "incident — feat-integration dev push + edge redeploy #1102; RCA settled by live kubectl (recovery session) + code-context (this session)"
status: "open"
priority: "critical"
tags: [incident, alpha, conductor, holochain, genesis, CellWithoutGenesis, happ-version, floating-tag, reproducibility, self-heal, edge, leak-fix]
relatedNodeIds:
  - backlog-doorway-substrate-stats-unmeasured-not-zero
  - backlog-agent-peer-binding-cross-signed-proof
cites:
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/main.rs
  - genesis/manifests/humans/terrance-tutor.yaml
  - elohim/elohim-storage/Dockerfile
  - genesis/data/timeline/backlog/ci-genesis-lamad-shell-routing-regression.md
---

# Alpha CellWithoutGenesis — floating-tag DNA drift + heal-unreachable-behind-panic

## What happened (RCA, settled by live kubectl + code)
The ~14 alpha conductor StatefulSets (`elohim-<name>-alpha`, container `elohim-node` =
`elohim-storage:1.0.0-dev-024da9a4`) crashloop with a FATAL `CellWithoutGenesis` panic: cells
registered with a **valid** DnaHash but **zero genesis records**. Chain:

1. **Floating-tag DNA drift.** The hApp is fetched every boot by an init container:
   `oras pull …/elohim-happ:${HAPP_VERSION}` with `HAPP_VERSION=dev-latest` (configmap `happ-version`).
   `dev-latest` floats — the DNA pipeline republished it (likely around the #1294 DNA build), so the
   conductor installed/registered a cell for a **new** DNA whose genesis never completed → a
   genesis-less cell on the persistent `holochain-data` PVC. (The exact trap named in
   `happ_manager.rs:70-73`.)
2. **The self-heal is architecturally UNREACHABLE.** `ensure_happ_installed` (`main.rs:641` →
   `happ_manager.rs:48`) DOES self-heal: `if is_stale(app_info) || drifted → uninstall + install_fresh`
   (re-genesis), and `is_stale` covers "missing roles or **empty cells**." BUT it runs in the storage
   service **after** the conductor's admin-WS is up — and the conductor **panics on the genesis-less
   cell during its own `startup_shutdown_impls`**, tearing down the WS first. So the storage service
   retries to attempt 60 → "Conductor failed to become ready" → exits. **Zero heal markers in the log.**
   The heal is downstream of a fatal panic; it can never fire.
3. **`RESET_STORAGE` cannot recover it.** `reset-storage-pod.sh` only `rm`s `/data/content.db`
   (the `storage-data` PVC) and restarts — it **never touches `/var/local/lib/holochain`**
   (`holochain-data`, where the genesis-less cell + keystore live; `happ_manager.rs:67-69`). So the bad
   cell survives every reset cycle untouched → reset-loops toward a 600s-per-pod timeout.
4. **Env levers are dead-on-arrival.** `ALLOW_DNA_REINSTALL=true` and `GENESIS_SELF_HEAL_IDENTITY=1`
   are already set and impotent (heal is behind the panic); setting `FORCE_DNA_REINSTALL=true` would be
   a no-op for the same reason.

**Not caused by the feat integration** (it touched no conductor/DNA/manifest source); the edge
redeploy `#1102` was merely the restart that re-pulled the floating `dev-latest` DNA. The keystore is
persistent (no re-key on plain restart) — earlier ephemeral-keystore / re-key reads were BOTH wrong.

## Recovery (operator/kubectl — panics-first branch)
Per conductor (each is its own 1-replica STS), **PIN the happ/conductor to a known-good immutable
digest FIRST** (else `install_fresh` re-genesises against whatever `dev-latest` currently is →
re-drift), then clear the bad conductor state so it boots clean:
```
kubectl scale sts elohim-<name>-alpha -n elohim-alpha --replicas=0
kubectl delete pvc holochain-data-elohim-<name>-alpha-0 -n elohim-alpha   # NOT /data/content.db
kubectl scale sts elohim-<name>-alpha -n elohim-alpha --replicas=1
```
Fresh empty `holochain-data` → clean boot → `ensure_happ_installed` sees no app → `install_fresh` →
genesis (re-keys; acceptable on alpha per `GENESIS_SELF_HEAL_IDENTITY=1`). `/data/content.db` stays
(content re-syncs P2P). Canary ONE node (caleb) before the other 13. STOP running `RESET_STORAGE`.

## Durable fixes (repo surface — this is the "never again")
1. **Pin the conductor + hApp to immutable digests; bring the edgenode build into the reproducible
   graph.** `HAPP_VERSION=dev-latest` and `elohim-storage/Dockerfile` `CONDUCTOR_SOURCE_IMAGE=
   elohim-edgenode:latest` are both floating tags built by the **manual che-devworkspaces edgenode job**
   (no build-manifest, not orchestrator-triggered) — so the conductor leak-fix is NOT reproducible from
   the general pipeline, and any boot/redeploy can silently re-bake a different DNA. Pin both to
   `@sha256:` digests (or dated tags), pass `CONDUCTOR_SOURCE_IMAGE` explicitly as a single declared
   build-arg, and give the edgenode build a `build-manifest.json` so graph-walker chains it. (= "Fix B"
   from the leak-fix-reproducibility RCA; now also a recovery PREREQUISITE.)
   - **Why the hApp pin is partition-safety, not just drift-prevention:** the hApp (DNA) is a SEPARATE
     artifact from the conductor — `elohim-edgenode` bakes only the conductor binary; the DNA is fetched
     at boot by the `happ-fetcher` init via `HAPP_VERSION` (so the conductor pin C does NOT cover it).
     And peers on different DNA hashes **cannot communicate** (no live cross-version bridge): a floating
     `HAPP_VERSION` lets restarting peers land on different DNAs → not only `CellWithoutGenesis` drift but
     a **P2P partition**. So the DNA MUST be consistent across ALL peers — pinned + rolled atomically —
     until Holochain ships native cross-version bridges OR we build the internal DNA
     upgrade/update/rollback path (the runtime self-migrating with lineage). Full rationale + the
     bake-vs-pin trade-off: `genesis/docs/superpowers/specs/2026-06-23-runtime-orchestration-developer-mode-bridge-design.md`
     §"Invariant: the DNA must be consistent across ALL peers".
2. **Make the genesis-less self-heal reachable.** The conductor panicking fatally on
   `CellWithoutGenesis` at startup makes the storage service's reinstall-on-stale heal unreachable. Add a
   **pre-conductor-start cell-health sweep** (detect + clear a genesis-less cell on `holochain-data`
   before the conductor loads it) OR make `CellWithoutGenesis` non-fatal (degrade so the WS comes up and
   `ensure_happ_installed`'s `install_fresh` runs). Then this class auto-heals on restart instead of
   crashlooping + needing a manual per-node PVC wipe. Home: `elohim/elohim-storage/src/main.rs`
   (pre-conductor sweep, the reachable surface) ± the conductor submodule.
