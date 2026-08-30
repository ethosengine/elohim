---
id: "backlog-ci-cid-artifact-twin-source-side-freshness-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis red — manifesto.json CID twin drifted from manifesto.md; the freshness gate was keyed to the generator's directory, not the source's"
slug: "ci-cid-artifact-twin-source-side-freshness-gap"
written: "2026-08-30"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [116c98ba145a, 844752df1596]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, seeder, cid-artifact, generated-twin-freshness, prepush-gate, host-green-not-ci-green, validate-constants]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1522/
  - genesis/seeder/src/__tests__/cid-artifact-integrity.spec.ts
  - genesis/seeder/src/sync-cid-artifacts.ts
  - genesis/seeder/src/cid-artifact.ts
  - genesis/data/lamad/content/manifesto.json
  - genesis/docs/content/elohim-protocol/manifesto.md
  - app/elohim-library/build-manifest.json
  - .husky/pre-push.bash
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/data/timeline/backlog/ci-genesis-seeder-spec-typecheck-gate-gap.md
---

# The generated CID twin went stale because the gate that watches it is keyed to the wrong directory

## The failure

`elohim-genesis/dev` **#1522** (`FAILURE`), stage **Validate Constants**,
`src/__tests__/cid-artifact-integrity.spec.ts` — three of five assertions red
(quoted from the #1522 console, lines 1879–1883):

```
→ manifesto.json.content drifted from manifesto.md. Re-sync: pnpm --filter genesis-seeder run sync:cid-artifacts
→ manifesto.json.blobHash drifted from manifesto.md — /blob/<hash> would 404. Re-sync: …
  expected 'sha256-5ec96d4ee336e3f39badc5dcd8764e…' to be 'sha256-2454b80685dd0950b7cd55f64a7c83…'
→ manifesto.json.blobCid drifted from manifesto.md — CID address dangles. Re-sync: …
  expected 'bafkreic6zfwu5yzw4pzzxlof3tmhmtwdltbo…' to be 'bafkreibeks4anbo5bfilptkv6zfhza5hh6e3…'
```

Occurrence evidence (ledger): `seen: 1`, `first_build: 1522`, `last_build: 1522`
for both captured fingerprints. Two of the three failing assertions were
fingerprinted — the `content drifted` line was not captured, a granularity
observation for `backlog/ci-harvest-fingerprint-granularity-banner-collision.md`,
not a separate concern (all three are one root symptom).

The stage aborts **before Seed Database**, so nothing seeded: elohim.host kept
serving the prior manifesto head while seven `manifesto.md` commits had already
landed on `dev`.

## Verdict — **real**, not flake

Deterministic content-hash comparison; no timing, no network, no cross-build
correlation needed. `seen: 1` is exactly right for a real drift: it appears the
first time CI observes the mismatched pair and stays red until re-synced.

## Root cause — two layers

**Layer 1 (the drift itself).** `manifesto.json` carries three source-tracked
fields (`content`, `blobHash`, `blobCid`) derived from
`genesis/docs/content/elohim-protocol/manifesto.md`. The source moved; the twin
did not. Fixed by regeneration — `a36fb36e1`.

**Layer 2 (why no rail caught it, the concern that outlives the fix).** The
guard *exists* and is good — but it is **keyed to the generator's directory,
not the source's**. `cid-artifact-integrity.spec.ts` lives in `genesis/seeder`,
so it runs when the local gate selects the `genesis` project. The edit that
breaks it is a docs `.md`. Verified on disk:

```
$ printf 'genesis/docs/content/elohim-protocol/manifesto.md\n' \
    | node genesis/orchestrator/gate-runner.mjs --changed-file-list --names
elohim-storybook
```

`manifesto.md` is claimed by `app/elohim-library/build-manifest.json` (Storybook
consumes the protocol docs as story sources) and by nothing in `genesis/`. So
the deterministic-floor guard could not fire pre-push for the only edit that
breaks it, and CI became the first observer — the textbook *host-green ≠
CI-green* shape (museum trap 4/13 family), with a new mechanism: **trigger
misrouting of a generated-twin freshness check.**

The repo already hand-patched this exact class four times without naming it —
`.husky/pre-push.bash` carries source-side legs for humans/presences,
device archetypes, deployments, and account packages. CID artifacts were the
one generated twin nobody wired a source-side trigger for.

**Layer 2b (the remediation hint fails silently green).** Both the assertion
message and the sync tool named `pnpm --filter genesis-seeder`. The package is
`holochain-seeder`, and `pnpm --filter <unknown>` prints
`No projects matched the filters` and **exits 0** — a copy-pasted fix reads as
success and changes nothing. Nine occurrences across the seeder sources, plus
one in the pre-push hook's own `build:data` hint.

## Current decision

Fix and rail both landed locally-verified; `ci_status: in-progress` until the
`elohim-genesis` green streak confirms disappearance (harvester-owned). No
blocker, no infra surface, no cluster touch.

## Fix trail

- `a36fb36e1` — regenerate `genesis/data/lamad/content/manifesto.json` from the
  shipped `manifesto.md` (dispatcher-landed; verified here against the #1522 log:
  the build's *expected* `sha256-2454b806…` / `bafkreibeks4a…` are byte-identical
  to what the file now carries).
- **This commit** — the source-side rail:
  - `genesis/seeder/package.json`: new `sync:cid-artifacts:check` script
    (the tool already had `--check`; it had no script name to call it by).
  - `.husky/pre-push.bash`: new `cid-artifact-freshness` leg, keyed to
    `^genesis/docs/content/elohim-protocol/` and `^genesis/data/lamad/content/`
    — the **source** side. Pure node+tsx, **0.6 s**, so PVC-exempt by omission
    (never in `HEAVY_GATES`, never deferrable). Both trigger paths verified to
    survive the `.ci-ignore` filter, and the leg sits after it.
  - Nine `pnpm --filter genesis-seeder` → `holochain-seeder` corrections across
    `genesis/seeder/src/**` (assertion message, sync tool header,
    `seed-humans`, `seed-agent-bindings`, `seed-conductor-identities`,
    `build-data`), plus the pre-push `build:data` hint, plus the regenerated
    `humans.json` / `presences.json` description strings (description-line-only
    diff, confirmed).

Local verification:

- `npx vitest run src/__tests__/cid-artifact-integrity.spec.ts` → **5/5 pass**.
- Negative test of the new rail: appended a byte to `manifesto.md`, ran
  `pnpm run sync:cid-artifacts:check` → `✗ manifesto.json: drifted [content,
  blobHash, blobCid]`, exit 1; source restored, tree clean.
- `pnpm --filter holochain-seeder run sync:cid-artifacts:check` → in sync ✓
  (proves the corrected filter resolves, where the old one no-op'd at exit 0).

## Residual (not taken this run)

`genesis/docs/superpowers/specs/2026-06-15-node-resource-tunables-and-exhaustion-shape-design.md`
carries two more `pnpm --filter genesis-seeder` hints. That path is a **managed
surface** (`.claude/scripts/_lib/managed_surfaces.py` matches
`genesis/docs/superpowers/specs/*.md`), so correcting it belongs in a cite-tooling
pass, not in a CI-triage commit.
