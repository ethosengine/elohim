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

## 2026-11-04 lead: the repository is shallow

`brit rebase origin/dev` refused this checkout outright: "shallow repositories require Git".
`.git/shallow` exists (written 2026-11-03 09:44), so some fetch truncated this repository's history.
The gix paths behind `build fingerprint`, brit's rebase and rakia-brit's commit resolution do not
support a shallow repository. `brit revision resolve` still works, which explains the split seen
above. First probe: `git rev-parse --is-shallow-repository`, then `git fetch --unshallow origin`
on a copy of this repository and re-run `brit build fingerprint … --commit HEAD`. Also find which
fetch created the shallow boundary (an integrator `--depth` fetch, or a Che bootstrap clone), so it
does not recur.

## 2026-11-04 probe: shallowness is not the cause

Still failing on brit 0.1.8 (`a8187cb3c`). A probe repository whose `objects/info/alternates` points
at this repository's object store, carrying copies of its `config`, `packed-refs`, every loose ref,
`HEAD`, `shallow`, `brit-mutation.guard`, `commit-graph`, `index`, `modules/`, `worktrees/`, `logs/`
and `.gitmodules`, fingerprints `elohim/eprfs/build-manifest.json --commit HEAD` cleanly, with the
shallow marker present and with it removed. The boundary commit's parents are present locally, so the
marker does not truncate anything the walk needs. **Do not unshallow as a cure.** The cause, found by bisecting the probe, follows.

## 2026-11-04 cause: gix 0.81 opens this repository with 32 pack-index slots, and it has 49

Diagnosed by a Codex (gpt-5.6-sol) bisect and verified on this device. `brit build fingerprint` opens
the repository through `gix::discover()` (`elohim/brit/brit-cli/src/commands/fingerprint.rs:36`) on
`gix 0.81` (`brit-cli/Cargo.toml:49`, `brit-graph/Cargo.toml:19`; `rakia-brit/Cargo.toml:18` carries
the same pin). `.git` is owned by an unmapped uid while brit runs as root, so discovery first treats
the repository as reduced-trust and selects `Slots::Given(32)` (vendored gix `open/options.rs:157`).
`safe.directory` later promotes trust to Full, but the 32-slot ODB stays. This repository has 49 pack
indexes and no multi-pack index, so revision parsing cannot load every index, and the capacity error
surfaces as "rev-spec is malformed" / "couldn't parse revision". A harness on the embedded gix shows
the flip: the probe fails at 32 and 48 slots and resolves at 49 or disk-sized slots; the live
repository resolves HEAD with disk-sized slots. Linked worktrees' admin directories are root-owned,
so they get disk-sized slots, which is why the 2026-11-03 worktree run worked. Pack count crossing
32 is what turned it red. Shallowness, covenant refs, `info/refs` and worktree contents are cleared.

**Cure (brit repository):** move `brit-cli` and `brit-graph` to brit's current gix, whose
`open/options.rs:194` and `open/repository.rs:550` defer slot selection until trust is final, and do
the same for `rakia-brit`. A smaller backport to the 0.81 fork is acceptable. Not cures: repacking,
GC, a multi-pack index, chown, or unshallowing; each only moves the threshold. The gate's
`evidence: unavailable` line should also print the underlying ODB error instead of the rev-spec one.
**Probe for the cure:** unchanged (`brit build fingerprint elohim/eprfs/build-manifest.json --commit
HEAD` in the main checkout prints a fingerprint).

## 2026-11-04 cure in flight, and the defect is wider

- brit branch `fix/gix-slots-after-trust` (off brit `6a919e096`, worktree `.worktrees/brit-gix-slots`):
  one brit-cli open helper for `build fingerprint` and `build plan` that, when final trust is Full,
  re-opens the repository with disk-sized slots; Reduced trust keeps the 32-slot cap; the gix 0.81
  pin is kept (it is deliberate: brit's 2026-07-01 Nexus publish design ~99). A debug build of it
  fingerprints the main checkout at `6c9260b6d` where brit 0.1.8 (`a8187cb3c`) fails. A gpt-6-astra
  review returned DO-NOT-LAND (the second open re-ran discovery, so a swapped repository could
  inherit the first one's trust; the regression test did not exercise the production path); the
  next round opens the exact resolved repository with `bail_if_untrusted(true)` and binds identity.
- **Wider than fingerprint:** plain `brit status --no-write` and `brit diff` (in-tree gix) fail in
  linked worktrees of the monorepo with "The object database has too few index slots, current=32,
  needed=17". brit's core open path needs the same correction; that part is offered to the
  brit-stability work on shem (berth notes 2026-11-04 22:41Z and after).

## 2026-11-04 adjacent CLI defect: `brit fetch -r` is shadowed by the global `-r`

In the monorepo's `elohim/brit` submodule, `brit fetch -r origin <refspec>` fails with "Could not
verify that \"origin\" is a valid git directory before attempting to use it", and without `-r` it
fails with "No configured remote could be found, or too many were available". `brit fetch --help`
documents `-r, --remote <REMOTE>`, but the global `-r, --repository` option appears to consume the
value first, so `origin` is parsed as a repository path (diagnosis shared with shem's brit-replay
session 01a1a894). Not in either current slice; a Git fallback fetched
`run-ci/rebase-reliability-20261104`. Probe for the cure: `brit fetch -r origin <refspec>` inside a
submodule fetches.
