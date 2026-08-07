---
id: "backlog-deprecation-devfile-start-doorway-dead-command-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire the dead devfile `start-doorway` command (superseded by hc-start.sh)"
slug: "deprecation-devfile-start-doorway-dead-command-retire"
written: "2026-06-10"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: []
relatedNodeIds: []
tags: [deprecation, devfile, che, doorway, hc-start, dev-stack]
cites:
  - devfile.yaml
  - app/elohim-app/scripts/hc-start.sh
  - genesis/docs/superpowers/plans/2026-06-10-che-network-agency-arc-plan.md
  - genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
---

## What is deprecated

The Eclipse Che `devfile.yaml` command `start-doorway` (id `start-doorway`,
~lines 276–281). The deprecation is **intentional and self-authored** — a
comment-only note added in commit `1162438ae` (arc plan Task 2.3) that the
PostToolUse deprecation sentinel captured as a NEW fingerprint:

```
# DEPRECATED: use app/elohim-app/scripts/hc-start.sh (pnpm run hc:start) — the
# canonical stack starter. This command's hardcoded ws://localhost:4444 conductor
# URL is dead (hc-start discovers the dynamic admin port via the
# elohim/holochain/local-dev/.hc_ports file it writes), and the workingDir
# predates the move to doorway/doorway-service.
```

The command is **non-executable in the current tree** — broken on three axes:

1. **Dead `workingDir`**: `/projects/elohim/holochain/doorway` does not exist.
   The entire `/projects/elohim/holochain/` directory is gone — doorway moved to
   `doorway/doorway-service/`. The exec can't even enter its working directory,
   so `cargo build --release` fails before anything runs.
2. **Dead conductor URL**: the hardcoded `--conductor-url ws://localhost:4444`
   predates dynamic-port discovery. `hc-start.sh` writes the live admin port to
   `elohim/holochain/local-dev/.hc_ports`; the static 4444 is no longer where the
   dev conductor binds.
3. **Stale storage assumption**: `STORAGE_URL=http://localhost:8090` and
   `./target/release/doorway` assume a build layout that the workingDir move
   invalidated.

It is superseded by `app/elohim-app/scripts/hc-start.sh` (`pnpm run hc:start`),
the canonical local-stack starter per the arc plan.

## Usage inventory

The `start-doorway` command id is invoked from exactly **one** place — the Che
command palette via the devfile itself. It is referenced nowhere as a runnable
dependency:

- `devfile.yaml:276` — the command definition (the only invocation surface; Che
  surfaces it in the UI command palette).

The two other textual occurrences are **documentation of this deprecation
decision**, not invocations:

- `genesis/docs/superpowers/plans/2026-06-10-che-network-agency-arc-plan.md:59,195`
  — arc plan Task 2.3 row + the "leave + add a deprecation comment" instruction.
- `genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md:160`
  — design doc listing the three duplicated stack-start seams.

No CI job, shell script, package.json script, or doc *runs* `start-doorway`.
Removing it cannot break any automation. (Sibling, separately tracked: a stale
`doorway:start` package-script snippet embedded in
`elohim/holochain/docs/claude.md:837` points at the same dead
`../holochain/doorway/target/release/doorway` path — a docs-snippet concern, not
this command, left out of this fingerprint's scope.)

## Migration path

There is no API migration — the replacement already exists and is documented:

- **Use** `pnpm run hc:start` (which runs `app/elohim-app/scripts/hc-start.sh`)
  for the full local stack (conductor + storage + doorway), with dynamic ports
  discovered via `elohim/holochain/local-dev/.hc_ports`.
- **Terminal state for the command itself**: delete the `start-doorway` exec
  command block from `devfile.yaml` (the id entry and its 5-line body, plus the
  3-line DEPRECATED comment above it). No replacement command is needed in the
  devfile — `hc:start` is invoked through the app's pnpm scripts, not a Che
  command-palette entry.

## Current decision

**Blocked — operator-gated devfile removal.** The evidence says the command is
**safe to remove** (non-executable, single non-automation invocation site,
documented live replacement). But the removal is deliberately *not* a
background-agent bounded fix, for two reasons:

1. **Fresh, deliberate operator decision.** The arc plan Task 2.3 chose, in the
   *same commit* that triggered this fingerprint (`1162438ae`), to "leave; add a
   deprecation comment pointing at hc-start" rather than remove — explicitly
   because "devfile edits propagate on workspace rebuild and are operator-visible
   startup surface." Applying the removal blind would override a same-day,
   intentional decision on a workspace-rebuild surface. Comment-now,
   remove-later is the author's chosen sequence.
2. **Workspace-rebuild blast radius.** A `devfile.yaml` edit re-materializes on
   the next Che workspace rebuild and is operator-visible at startup. That is an
   operator-owned surface (the same posture the repo takes toward cluster
   manifests), not a silent background landing.

So the right terminal state for this run is: canonicalize the safe-to-remove
finding and hand the operator a ready, evidence-backed one-edit removal — gated
behind their next devfile-cleanup pass (a natural companion to the arc plan's
Phase-2 stack-seam de-duplication and the sibling `hc-start-*`/`hc-seed-*`
dev-stack drift entries). The sentinel will suppress further dispatch on this
fingerprint (ledger `status: blocked`); the stasis sweep / the operator's
devfile pass owns the removal.

The deprecation comment stays in place meanwhile — it correctly steers anyone
who opens the command palette to `hc:start`.

**Fingerprint list emptied 2026-08-07 — the concern is unchanged, its capture
surface is gone.** This entry is the cleanest specimen of the class Guard O now
closes: it was *born* from a self-authored comment. The `# DEPRECATED: use
app/elohim-app/scripts/hc-start.sh …` note that commit `1162438ae` added on
purpose was fingerprinted as a finding (`a2464d792194`), and a later scope grep
of the same three lines minted a second one (`8cb5f41fe4ea`, still `open` when
Guard O landed) — two background Opus dispatches spent on one comment whose
entire content is a pointer to the replacement it names.

Both rows are deleted: a first-party comment narrating a deprecation is the
documentation surface, not a live toolchain warning, and the sentinel no longer
fingerprints it (`.claude/hooks/deprecation-sentinel.py` Guard O; see
`deprecation-sentinel-redundant-capture-surfaces.md` Class 7). Nothing about the
decision above changes — the removal is still operator-gated and still owed. The
only difference is that the *comment* no longer bills for it.

## Verification

N/A — not yet removed (blocked on operator-gated devfile edit). When the operator
removes the `start-doorway` block on a devfile-cleanup pass, verification =
`devfile.yaml` parses (the workspace rebuilds cleanly) and `grep -n start-doorway
devfile.yaml` returns nothing. Then delete this entry (the ledger rows are
already gone — see Current decision).
