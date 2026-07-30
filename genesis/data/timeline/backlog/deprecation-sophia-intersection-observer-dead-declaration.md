---
id: "backlog-deprecation-sophia-intersection-observer-dead-declaration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Deprecated intersection-observer polyfill declared in packages/sophia with zero references — 2-line deletion, queued behind the submodule freeze"
slug: "deprecation-sophia-intersection-observer-dead-declaration"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: open
severity: low
fingerprints: ["50aa3734f6b0", "010ff5a7bfb5"]
relatedNodeIds: []
tags: [deprecation, intersection-observer, polyfill, sophia, dead-declaration, submodule]
cites:
  - https://www.npmjs.com/package/intersection-observer
  - https://developer.mozilla.org/en-US/docs/Web/API/IntersectionObserver
  - sophia/packages/sophia/package.json
---

## What is deprecated

`pnpm install` in the sophia submodule warns:

```
packages/sophia                          |  WARN  deprecated intersection-observer@0.12.2
```

`intersection-observer` is the W3C polyfill for the `IntersectionObserver` DOM
API. It is deprecated upstream for the ordinary reason a polyfill retires:
`IntersectionObserver` is now baseline-supported in every browser the app
targets, so the shim is obsolete rather than broken.

**The distinguishing fact here: the package is not used at all.** It is a dead
manifest declaration, not a live dependency. A full-tree scan of the submodule
(excluding `node_modules`, `.git`, and `pnpm-lock.yaml`) for *both*
`intersection-observer` and `IntersectionObserver` returns exactly **two** hits —
and both are the declaration itself:

```
sophia/packages/sophia/package.json:87   "intersection-observer": "^0.12.0",   (devDependencies)
sophia/packages/sophia/package.json:126  "intersection-observer": "^0.12.0",   (peerDependencies)
```

Zero imports. Zero `require`. Zero references to the `IntersectionObserver`
global anywhere in sophia's sources or tests. Nothing loads the polyfill, and no
code calls the API it shims.

## Usage inventory

**Declarations — 1 file, 2 lines** (`sophia/packages/sophia/package.json`):

| Line | Section | Value |
|---|---|---|
| 87 | `devDependencies` (starts line 59) | `"intersection-observer": "^0.12.0"` |
| 126 | `peerDependencies` (starts line 98) | `"intersection-observer": "^0.12.0"` |

Note it is **not** in `dependencies` (lines 42–58), so it was never a runtime
dependency of the published package — only a dev install plus a peer request on
consumers.

**Consumers of the declaration:** none. No other manifest in either the submodule
or the parent repo declares `intersection-observer`; the parent repo's
`perseus-plugin` (the other Perseus bundling path) does not reference it either.

**Source references:** zero — established by full-tree grep for both the package
name and the `IntersectionObserver` identifier across all file types.

**Blast radius of removal:** nil. Because there is no import, removing the two
lines cannot change the UMD bundle's contents (rollup only inlines what the
import graph reaches) and cannot change runtime behaviour. The only observable
effect is that `pnpm install` stops printing the warning, and consumers stop
being asked for a peer they never needed.

## Migration path

No migration — this is a **deletion**, not a replacement. Nothing needs to adopt
the native API, because nothing was using the polyfilled one.

The exact ready-to-apply patch, in `sophia/packages/sophia/package.json`:

1. Delete line 87 (the `devDependencies` entry).
2. Delete line 126 (the `peerDependencies` entry).
3. Re-run `pnpm install` in `sophia/` to refresh `pnpm-lock.yaml`
   (`intersection-observer@0.12.2` should drop out of the lockfile entirely).

If a future sophia surface ever does need viewport observation, use the native
`IntersectionObserver` directly — no polyfill, no dependency.

## Current decision

**Fix is bounded, exact, and ready — queued behind a transient submodule freeze.
Deliberately not landed by this pass.**

This is the one concern of the three from this triage batch that *is* within the
bounded-fix envelope: a 2-line deletion in a single manifest, with a provably nil
blast radius. It was **not** applied for one reason only, and it is not a
scale reason:

> Every file needing edit lives inside the `sophia` git submodule, which at
> triage time had **uncommitted changes on branch `feat/node24`** — including
> `packages/sophia/package.json`, *the exact file this patch edits* (that branch's
> in-flight diff bumps `uuid` `^10.0.0` → `^11.1.1` in the same
> `dependencies`/`devDependencies` region). A concurrent agent is mid-flight on a
> Node 24 + dependency-security upgrade there. Editing it now risks clobbering
> unlanded work, and would also conflict on lockfile regeneration.

So this entry holds `deprecation_status: open` (fix specified and queued, not
blocked on any technical unknown) rather than `blocked`. The ledger fingerprint
`50aa3734f6b0` is set to `status: triaged` for the same reason — the concern is
canonicalized with a decision the sentinel can cite, and the work is queued
rather than terminal. **No sophia file was modified by this triage pass.**

**Live trajectory — one small step, unblocking on a known event.** When
`feat/node24` lands (or its agent finishes and the submodule worktree is clean):

1. Re-check whether that branch already removed the two lines as part of its own
   dependency sweep — its diff is already touching this file, so this may
   self-resolve. If so, close out: delete the ledger line, delete this entry.
2. Otherwise apply the 3-step patch above, run sophia's `pnpm install` +
   `pnpm test` + `pnpm build`, and on green **close out with full decomposition**:
   delete ledger line `50aa3734f6b0` and delete this backlog entry, with the
   verification quoted in the commit message. There is no lesson here worth
   graduating to a chronicle — a dead declaration removed is exactly the case
   that should decompose to nothing but a commit.

A note for whoever picks this up: the *reason* it is safe to simply delete —
rather than migrate to the native API — is the zero-reference finding under
Usage inventory. Re-confirm that grep before deleting (it is cheap), because if
`feat/node24` or any other in-flight work introduces a first real
`IntersectionObserver` usage, the correct action changes from "delete the
declaration" to "use the native API and still delete the declaration."

## Verification

What this pass proved (no fix landed — scoping evidence, not fix verification):

- **Zero-usage, full-tree:** grep across the whole `sophia` submodule for
  `intersection-observer` **and** `IntersectionObserver`, excluding
  `node_modules`, `.git`, and `pnpm-lock.yaml`, over all file types → exactly
  **2** hits, both being the declaration lines themselves. A narrower
  `--include=*.ts --include=*.tsx --include=*.js --include=*.jsx` scan over
  `packages/` returned **0** hits.
- **Declaration placement:** section-header line numbers in
  `packages/sophia/package.json` (`dependencies` 42, `devDependencies` 59,
  `peerDependencies` 98) put both hits (87, 126) in dev + peer, **not** runtime
  `dependencies`.
- **No other declarer:** no `intersection-observer` in any other manifest in the
  submodule or the parent repo.
- **Contention:** `git -C sophia status --porcelain` showed
  `packages/sophia/package.json` modified on `feat/node24` at triage time; its
  diff touches the same dependency block.

Closure requires: both lines deleted, `intersection-observer` absent from
`sophia/pnpm-lock.yaml`, and sophia `pnpm install` + `pnpm test` + `pnpm build`
green — then this entry and its ledger line are deleted, not parked.
