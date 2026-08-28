---
name: feedback_sccache_failure_classes
description: "Three sccache failure classes: cache-corruption null-byte/unclosed-delimiter, intermittent spawn ENOENT, and AccessDenied on a dead Garage key (server won't start → DNA red in 85s)."
metadata: 
  node_type: memory
  title: sccache failure classes (umbrella)
  type: feedback
  originSessionId: fd355744-897d-4e84-ba71-b1962e798f4c
  modified: 2026-08-27T22:34:35.508Z
---

# sccache failure classes (umbrella)

Folds the distinct sccache failure-mode entries. Members:

- **Storage-wipe class (2026-08-27, elohim-holochain #1403; root-caused 2026-08-28):** the AccessDenied was NOT a rotated key — Garage's LMDB metadata emptied 2026-08-23 (1Gi memcg, mmap-backed LMDB, termination reason Unknown) and the resync worker reaped every block against the empty view; tell = `garage status` HEALTHY + layout intact + every `garage stats` table 0. Buckets + keys re-provisioned 2026-08-28 (sccache-elohim, tempo, pyroscope — the bootstrap Job covers only sccache). Symptom description follows: `sccache: error: Server startup failed: cache storage failed to read: PermissionDenied … S3Error { code: "AccessDenied", message: "Forbidden: No such key: GK…" }` — the Garage access key in the `jenkins`-ns `sccache-credentials` Secret no longer exists server-side. Surfaces as `sccache rustc -vV` exit 2 → `cargo metadata … exit status: 101` → `DNA BUILD FAILED` ~85 s in, before any compile. Classifier grep = `Server startup failed.*AccessDenied`. Operator-owned (Secret + Garage admin); repo mitigation = a flake `shellHook` probe that leaves `RUSTC_WRAPPER` unset when the server can't start (cold compile, not a red) — see backlog `sccache-garage-harden`. Edge/app/epr have no wrapper and are unaffected. The DNA job is `longRunning` (fire-and-forget), so its red is INVISIBLE to the orchestrator's level guard — edge and genesis still dispatch (orchestrator #1733: holochain #1403 FAILURE, edge #1386 dispatched anyway). Do not assume a level-0 red blocks the run; check the downstream job itself.

- [[feedback_sccache_cache_corruption_recovery]] — 'unclosed delimiter'/null-byte = .sccache_check 404 leaked into rustc probe; an EMPTY bucket (a full wipe!) triggers it; fix RUSTC_WRAPPER='' or heal the sentinel.
- [[feedback_sccache_spawn_enoent_rca]] — cargo intermittently fails to spawn the sccache binary itself (~1.7%, matches sccache #2023/#2687); classifier grep = `could not execute process .sccache rustc`, NOT ENOENT.*build-script.
