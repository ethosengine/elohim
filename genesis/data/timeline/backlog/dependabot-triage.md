---
id: "backlog-dependabot-triage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dependabot alert surface — 302 open on the default branch (135 high) as of 2026-10-03; index of the concern entries that own them"
slug: "dependabot-triage"
written: "2026-06-02"
author: "cartographer"
status: "wip"
priority: "high"
area: "cargo"
recurrence: 2
source_shifts:
  - "2026-05-17"
domain: "operator"
deprecation_status: in-progress
severity: security
fingerprints: [c4bc9714e080, c81302c029ff]
relatedNodeIds:
  - "memory:feedback_cargo_resolution_vs_compilation"
  - "memory:feedback_subagent_dep_conflict_supervision"
tags: [cargo, security, dependabot, vulnerabilities, operator-domain]
cites:
  - https://github.com/ethosengine/elohim/security/dependabot
shift_objective: |
  Dependabot/GitHub reports 191 vulnerabilities (1 critical, 113 high, 62 moderate, 15 low) on
  the default branch, untriaged (first surfaced 2026-05-17 at 170/109-high; re-surfaced
  2026-06-06 as a push-time banner at 191/113-high — count is growing, not draining). With no
  triage pass, the alert count is noise — there is no signal about which advisories are
  reachable in our code vs transitive-only, exploitable vs theoretical, or fixable-by-bump vs
  blocked-on-an-upstream.
  Resolve it with a triage pass that produces a reviewed disposition per advisory cluster:
  bump-now, blocked-on-upstream (with the blocking crate named), not-reachable (transitive
  dep we don't exercise), or accepted-risk (with rationale). Mind the cargo gotchas: a
  pre-release crate resolving is NOT the same as compiling (feedback_cargo_resolution_vs_compilation),
  and version bumps need crate-wide caller review (feedback_subagent_dep_conflict_supervision).
  This is operator+maintainer domain (the triage decisions and any risk-acceptance are
  operator calls). Done when the 191 alerts have a reviewed disposition and the
  critical+high-severity reachable subset has a remediation plan.
---

# Triage the Dependabot vulnerability backlog

## What is flagged (quote the banner)

> remote: GitHub found 191 vulnerabilities on ethosengine/elohim's default branch
> (1 critical, 113 high, 62 moderate, 15 low).

Captured as security ledger fingerprint `c4bc9714e080` (2026-06-06, push-time banner). The
earlier cartographer capture (2026-05-17) read 170/109-high; the count has grown, confirming
the surface is unmonitored, not draining.

## Why this matters

191 untriaged alerts (1 critical, 113 high) is alarm fatigue, not signal — the security
surface is effectively unmonitored because no one can tell which alerts matter. A triage pass
converts the count into a small actionable set.

## The failure shape

- Dependabot/GitHub raises 191 advisories on the default branch; none are dispositioned.
- No distinction between reachable-and-exploitable vs transitive-and-theoretical.
- No remediation plan for the critical/high-severity subset.

## Usage inventory (blast radius — bounded scope pass, 2026-06-06)

The advisories span the full polyglot dependency surface; this is why the count is large and
why a per-advisory disposition is operator-owned, not a background bump:

- **Cargo**: 33 `Cargo.lock` files across the workspace + vendored subrepos (`elohim/rust-ipfs`,
  `elohim/brit`, `elohim/rakia/...`), ~1,398 unique crates total. Three subrepos carry their
  own `.github/dependabot.yml` (`rust-ipfs`, `brit`, `rakia/elohim/brit`) — vendored advisory
  noise the top-level repo also inherits.
- **pnpm/npm**: root `pnpm-lock.yaml` (~28.7k lines), `sophia/pnpm-lock.yaml` (~19k lines),
  `che-devworkspaces/package-lock.json`.
- **No top-level `.github/dependabot.yml`** — only the three vendored subrepo configs exist, so
  there is no central update cadence or grouping policy at the repo root.

Per-advisory enumeration is **not reachable from the dev environment**: no `gh` CLI, no
`cargo-audit` binary, and `GET /repos/ethosengine/elohim/dependabot/alerts` returns HTTP 401
(requires a token the dev env doesn't hold). The authoritative list lives at the cited
GitHub security tab and is an operator-token read.

## Shape of the fix (operator/maintainer-owned dispositions)

Per advisory cluster, assign: **bump-now** / **blocked-on-upstream** (name the crate) /
**not-reachable** (transitive, unexercised) / **accepted-risk** (with rationale). Guard the
cargo traps:

- Pre-release crate resolves ≠ compiles (`feedback_cargo_resolution_vs_compilation`) — build
  before pinning a bump.
- Version bumps need crate-wide caller review (`feedback_subagent_dep_conflict_supervision`).

### Suggested first-pass sequencing (plan sketch for the operator sprint)

1. **The 1 critical first** — pull its GHSA from the security tab, identify the crate/package
   and whether it's reachable (direct dep vs transitive). A single critical is the one item
   worth a same-day decision.
2. **Cluster the 113 high by root crate/upgrade-unit**, not by alert — most of 191 will
   collapse into a handful of transitive trees (one outdated crate fans out into many CVEs).
   Canonicalize each *upgrade unit* as its own concern if it earns a distinct trajectory.
3. **Add a top-level `.github/dependabot.yml`** with grouped updates so the count stops growing
   silently between triage passes (this is the one bounded sub-deliverable that does NOT
   require per-advisory decisions and could land independently).
4. **Vendored subrepos** (`rust-ipfs`/`brit`/`rakia`) — decide whether their advisories are
   in-scope (we build them) or should be excluded from the top-level surface.

## Current decision (2026-10-03 — every open alert has an owning entry; nothing landed this pass)

> remote: GitHub found 302 vulnerabilities on ethosengine/elohim's default branch
> (135 high, 140 moderate, 27 low).

Ledger fingerprint `c81302c029ff` (push to `dev`, 2026-10-03). The default branch is `dev`, so
the banner describes the current tree. The count is 302 against 191 in June and 304 at the
July decomposition: the two campaigns closed alerts by fix and wrote the rest as repo-side
dispositions (NOT-REACHABLE, MIRROR-RETRY, NEEDS-OPERATOR) that were never recorded as
dismissals on GitHub, and 124 new alerts (#786 to #915: 117 npm, 7 cargo) have been raised
since.

**The alert list is now readable from the dev environment.**
`gh api --paginate '/repos/ethosengine/elohim/dependabot/alerts?state=open&per_page=100'`
returns all 302 (the `gh` CLI is installed and authenticated as EthosengineBot). Every
statement below that the list is an operator-token read is superseded.

**The mirror constraint is retired on both sides.** `.npmrc` and `.cargo/config.toml` now
resolve public packages straight from npmjs and crates.io (registry split, 2026-07-30). Every
`MIRROR-RETRY` disposition in the campaign record is therefore unblocked and should be re-run,
not re-probed.

Ownership of the 302, by upgrade unit:

| Alerts | Unit | Owning entry | State |
|---|---|---|---|
| 226 npm | same-major transitive overrides (50 entries) | `security-npm-transitive-override-refresh` | open, resolution proven, ready to run |
| 6 npm | Angular 22.1.0 to 22.2.x, whole family | `security-angular-22-patch-line-coordinated-bump` | open, queued |
| 5 npm | vitest 3 to 4 in three SDK packages | `security-sdk-vitest-3-to-4-bump` | blocked on dependency major, plan written |
| 4 npm | sharp 0.34 to 0.35 in `steward/device` | `security-steward-device-sharp-0-35-bump` | blocked on breaking bump, plan written |
| 16 npm | dev-tooling transitives with no same-major fix | `security-dev-tooling-transitives-without-same-major-fix` | blocked on parent majors or dismissal |
| 45 cargo | four service locks plus bitswap | `security-cargo-vulnerability-campaign-retry-queue` | operator decision sheet; 2026-10-03 addendum there |

By manifest: `pnpm-lock.yaml` 252, `elohim/elohim-storage/Cargo.lock` 16, `steward/node/Cargo.lock`
14, `doorway/doorway-service/Cargo.lock` 10, `elohim/holochain/tests/sweettest/Cargo.lock` 4,
`steward/device/package.json` 2, one each on `elohim/sdk/storage-client-ts/package.json`,
`elohim/sdk/epr-ts/package.json`, `elohim/elohim-agent/elohim-agent-sdk/package.json` and
`elohim/elohim-bitswap/Cargo.lock`.

Nothing was fixed in this pass. Every npm closure needs `pnpm install` in the shared working
tree plus the JS gates, and every cargo closure needs `cargo test`; the dispatching session
had an alpha fleet roll in flight in that tree and ruled out worktrees and heavy cargo. The
first move when the tree is quiet is the override refresh: one `package.json` edit, 226 alerts.

Still undone from the June plan sketch: a top-level grouped `.github/dependabot.yml` (item 3).

## Prior decision (2026-07-29 — campaign underway; superseded where the section above says so)

**The operator-initiated sprint this entry called for has started.** The alert surface is now
decomposed into eleven write-disjoint cluster files at repo root,
`VULNERABILITY_CLUSTER_{01..10,12}_*.md` (there is no cluster 11) — clusters 01–06 npm,
07–10 + 12 cargo. Cluster 06 is the sole permitted `pnpm-lock.yaml` writer; 01–05 hand it
target ranges. Clusters 09 and 10 have landed real remediations with locked-check evidence.

Three inherited "hard blockers" were re-probed on 2026-07-29 and **two were environment
artifacts, not upstream constraints**:

1. **"Missing Nexus package versions" — partly stale.** The `elohim-mirror` Nexus cargo group
   is reachable and serving (it fetched ~30 crates during verification). Transient outages had
   been recorded as permanent blockers in clusters 07/08. Nexus is a fetch layer only, so it
   was never the right lever for a *resolution* conflict either way.
2. **"pnpm blocked" — false in this environment.** Cluster 06's `ENOENT ... mkdir
   '/nix/xdg/cache/pnpm/store/v11'` was one agent's sandbox. Here `pnpm config get registry`
   → `https://nexus.ethosengine.com/repository/npm/` and `pnpm store path` →
   `/projects/.pnpm-store/v11`. **All ~195 npm alerts are actionable.**
3. **"Holochain's serde pin blocks `time`" — real conflict, wrong verdict.** The mechanism is
   a `serde_derive` exact-pin collision (`holochain_serialized_bytes 0.0.56` → `serde
   =1.0.219` → `serde_derive =1.0.219`), not `serde` vs `serde_core`. But **hsb `0.0.57`
   declares `serde = "=1.0.228"`**, and the Holochain 0.7 family moved `-dev.N` → `-rc.N` with
   every rc requiring hsb `=0.0.57`. serde 1.0.228 satisfies `jsonwebtoken 10.3.0`,
   `time 0.3.47` and `serde_with 3.19` — so a dev→rc bump closes that advisory class. **This is
   our decision, not upstream's.** Recorded in `memory:project_serde_wall_escapable_via_hsb_057`.

**The remaining operator ceiling is the Tier-2 half of that bump.** 16 lockfiles sit on hsb
0.0.56. Tier 1 (native services: doorway, elohim-storage, steward/node) is agent-executable —
separate workspaces, no DNA hash movement — though a client-library dev→rc bump cannot
self-certify wire compatibility against a dev-line conductor. Tier 2 (the 5 DNAs,
`holochain/rna/rust`, `tests/sweettest`, the 6 `sdk/domains/*/types` crates) moves the DNA hash
→ `ALLOW_DNA_REINSTALL`, new agent keys, prod migration/lineage. **That is the operator call
this entry still guards.**

### Prior decision (2026-06-06 — superseded by the above)

This is a security-class concern whose remediation crosses a dependency-major /
>20-file surface (33 Cargo.lock files, ~1,398 crates, two large pnpm trees) and whose core
work — per-advisory disposition and any risk-acceptance — is explicitly an operator/maintainer
decision. Per the deprecation-triage hard rule ("if the fix would touch >20 files or change a
dependency major version, STOP at blocked with a written plan sketch — that scale needs an
operator-initiated sprint, not a background agent"), the terminal automation state is
**blocked-and-canonicalized**. Ledger fingerprint `c4bc9714e080` is marked `triaged` so the
sentinel cites this decision and does not re-dispatch on the recurring push banner; the
deprecation-stasis sweep owns the re-check.

The one independently-landable, non-operator-gated sub-deliverable is item 3 above (a
top-level grouped `.github/dependabot.yml`). It is left here as a trajectory rather than
landed this run because it was not in the captured fingerprint's bounded scope and a config
that changes the org-wide update cadence is itself an operator policy choice.

## Acceptance

Every alert has a reviewed disposition; the critical+high-severity reachable subset has a
remediation plan. Re-check trigger: the next push banner whose count differs (the sentinel will
re-capture as a new fingerprint), or operator pickup of the shift_objective.

**Campaign surface as decomposed (2026-07-29): 304 unique alert IDs, span #482–#785** — larger
than the 191 in this entry's title, which is the historical 2026-06-06 push banner. Per-cluster:
01/15 · 02/34 · 03/88 · 04/57 · 05/1 · 06/15 · 07/36 · 08/31 · 09/21 · 10/11 · 12/10. The
authoritative live list remains the cited GitHub security tab (an operator-token read — `gh` CLI
and the dependabot alerts API are both unavailable from the dev environment). Per-cluster
disposition and evidence live in the cluster files, which are the working surface; this entry
tracks the campaign's decision state and the operator ceiling only.
