---
id: "backlog-beacon-repeatable-shared-record-lanes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "relay-addr-beacon: make the shared-record lane repeatable (Vec, not last-value-wins) so one beacon leg can contribute to BOTH doorways.elohim.host and the apex"
slug: "beacon-repeatable-shared-record-lanes"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (operator-requested Codex queue, doorway-federated continuity roadmap)"
status: "done"
priority: "high"
area: "doorway/utility-plane"
domain: "protocol"
jobs: [relay-addr-beacon]
relatedNodeIds:
  - "habit:doorway-failover"
  - "memory:project_two_premises_dns_beacon_owned"
cites:
  - genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md
  - genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md
tags: [doorway, dns, logical-anycast, beacon, bounded-code-fix, codex-claimable, agent-agnostic]
---

# Repeatable shared-record lanes in relay-addr-beacon

**Why this exists.** Apex multi-A (sprint-plan Task 3.1, Operator-menu item 2) is
blocked by an encoded beacon constraint: `--shared-record-name` / `--record-owner`
are `Option<String>` (`relay-addr-beacon/src/config.rs:100-106`), so a second
flag silently last-value-wins and one beacon leg cannot contribute to both
`doorways.elohim.host` (already shipped, both legs) and `elohim.host`. The
2026-08-18 attempt was reverted on exactly this evidence. The operator still
owns the apex flip; this item only removes the mechanism ceiling.

## Scope (bounded — the beacon crate only)

1. `config.rs`: `shared_record_name: Vec<String>` (clap `Action::Append`, env
   comma-separated) paired positionally with `record_owner` — OR one
   `--shared-record <name>=<owner>` repeatable flag (prefer the latter; it
   cannot mispair). Keep the existing single-flag spelling accepted for
   backward compatibility so the deployed manifests
   (`genesis/orchestrator/manifests/infra/alpha-coturn-{shem,operations}.yaml`)
   keep parsing unchanged.
2. `main.rs:79-89`: build one `cloudflare::SharedRecordConfig` per lane; the
   Cloudflare sink holds a `Vec` of shared lanes and PATCHes each with the same
   owner+freshness stamp (`sinks/cloudflare.rs` shared-lane logic unchanged per
   lane).
3. `Config::validate` (`config.rs:158`): every lane needs an owner; duplicate
   lane names are an error.
4. Update the crate README / flag docs; note in the two coturn manifests'
   header comment that the lane is now repeatable (do NOT add the apex lane —
   that is the operator's flip).

## DoD / verification (self-contained)

- Unit tests: (a) two `--shared-record` flags parse into two lanes, order
  preserved; (b) the legacy pair still parses into one lane; (c) a lane without
  an owner fails validation; (d) duplicate lane names fail validation.
- `cd relay-addr-beacon && RUSTFLAGS="" cargo test` → `EXIT=0` echoed on its
  own line; `cargo clippy -- -D warnings` clean; `cargo fmt --check`.
- Render-check: `check-ingress-conflicts.sh` untouched (no manifest host change).

## Disjointness

Own crate, own lockfile; no overlap with doorway-service or elohim-storage
write-sets. Sibling item: `beacon-health-aware-shared-lane-membership` (can be
done in either order; together they are the first health-aware doorway-set DNS).

## Completion (2026-08-23)

The beacon now accepts repeatable atomic `--shared-record <name>=<owner>` lanes
and comma-separated `BEACON_SHARED_RECORDS`, while the deployed legacy
`--shared-record-name` / `--record-owner` pair remains accepted unchanged.
Validation rejects missing owners and DNS-equivalent duplicate names. The
Cloudflare sink holds an ordered vector and applies the existing ownership,
freshness, and stale-sibling rules independently to every lane with one zone
lookup per pass.

Focused evidence: 43 crate tests pass with `EXIT=0`, including the four config
contract cases and an ordered two-lane PATCH test; `cargo clippy --tests -- -D
warnings` and `cargo fmt --check` are green. The crate README passed the required
fresh-context blind-reader loop. Both coturn manifests only gained an explicit
comment that the mechanism is repeatable: no apex hostname or live DNS lane was
added, so the production apex flip remains operator-owned.
