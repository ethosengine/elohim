---
name: feedback_sccache_failure_classes
description: "Three sccache failure classes: cache-corruption null-byte/unclosed-delimiter, intermittent spawn ENOENT, and AccessDenied on a dead Garage key (server won't start → DNA red in 85s)."
metadata: 
  node_type: memory
  title: sccache failure classes (umbrella)
  type: feedback
  originSessionId: fd355744-897d-4e84-ba71-b1962e798f4c
  modified: 2026-08-27T21:25:44.466Z
---

# sccache failure classes (umbrella)

Folds the distinct sccache failure-mode entries. Members:

- **Credential class (2026-08-27, elohim-holochain #1403):** `sccache: error: Server startup failed: cache storage failed to read: PermissionDenied … S3Error { code: "AccessDenied", message: "Forbidden: No such key: GK…" }` — the Garage access key in the `jenkins`-ns `sccache-credentials` Secret no longer exists server-side. Surfaces as `sccache rustc -vV` exit 2 → `cargo metadata … exit status: 101` → `DNA BUILD FAILED` ~85 s in, before any compile. Classifier grep = `Server startup failed.*AccessDenied`. Operator-owned (Secret + Garage admin); repo mitigation = a flake `shellHook` probe that leaves `RUSTC_WRAPPER` unset when the server can't start (cold compile, not a red) — see backlog `sccache-garage-harden`. Edge/app/epr have no wrapper and are unaffected, but the DNA job sits at orchestrator level 0, so `levelFailed` withholds edge + genesis for ANY push touching `elohim/holochain/**` — `.epr-meta/*.habit.md` atoms included.

- [[feedback_sccache_cache_corruption_recovery]] — 'unclosed delimiter'/null-byte = .sccache_check 404 leaked into rustc probe; an EMPTY bucket (a full wipe!) triggers it; fix RUSTC_WRAPPER='' or heal the sentinel.
- [[feedback_sccache_spawn_enoent_rca]] — cargo intermittently fails to spawn the sccache binary itself (~1.7%, matches sccache #2023/#2687); classifier grep = `could not execute process .sccache rustc`, NOT ENOENT.*build-script.
