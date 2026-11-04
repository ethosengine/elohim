---
id: "backlog-brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit build fingerprint cannot resolve any commit in the monorepo, so every gate's evidence is 'unavailable' and nothing reuses"
slug: "brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo"
written: "2026-11-04"
author: "claude (row-5 precursor session a340d2e2)"
status: "backlog"
priority: "high"
tags: [gate-evidence, reuse, brit, rakia-brit, gix, fingerprint, push-gate-pays-once]
cites:
  - genesis/orchestrator/gate-evidence.mjs
  - elohim/brit/brit-cli/Cargo.toml
---

# `brit build fingerprint` resolves nothing in the monorepo

## Observed (2026-11-04, this device)

- The push at 15:31Z printed `evidence: unavailable (fingerprint of <step> failed: error: invalid
  arguments: could not resolve commit '<sha>': couldn't parse revision …)` for 9 gates. No gate
  could look up or record evidence, so every gate ran in full.
- Reproduced with both brit 0.1.6 (`e93a0206eb`) and brit 0.1.7 (`5269295760`). In the monorepo,
  `brit build fingerprint <manifest> --commit HEAD` fails with "The rev-spec is malformed and misses
  a ref name", and a full 40-hex SHA fails with "couldn't parse revision". This holds for every
  monorepo manifest tried (eprfs, orchestrator, jenkins-bridge).
- The same command **works** in the `elohim/rakia` submodule's repository, and `brit revision resolve
  HEAD` works in the monorepo. The defect is therefore in the `build fingerprint` path, which runs on
  `rakia-brit` and its older gix 0.81, against this repository's state. Neither brit nor the manifests
  are at fault.
- It worked on 2026-11-03 evening: the eprfs proof run in a worktree of this repository recorded
  evidence `bafyreihg73…` at `bb4772d67`. Today that same commit fails to resolve.
- Present before the regression and not by themselves the cause: `.git/shallow` (2026-11-03 09:44),
  `refs/notes/brit/build/*%3A*` notes. Newer since the working run: `refs/notes/brit/covenant/<session>/<cid>`
  refs, `refs/notes/brit/build/gate%3Aeprfs`, the merged integrator history, and `.git/brit-mutation.guard`.

## Next probe

Bisect the repository state, not the binary. Clone this repository locally (no checkout), then add
the newer refs back one class at a time until `build fingerprint` fails. The likely suspects are the
covenant notes refs, which have a deeper path and long CID segments, and the shallow boundary
interacting with gix 0.81's revision walk. The cure belongs in rakia-brit's repository opening (move
it to brit's gix, or tolerate the ref shape), and the gate's `evidence: unavailable` line should name
the cause.

**Probe for the cure:** `brit build fingerprint elohim/eprfs/build-manifest.json --commit HEAD` prints
a fingerprint in the monorepo, and a push shows `evidence: hit` lines again.
