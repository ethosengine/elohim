---
id: "backlog-anti-entropy-egress-baseline-2026-07-27"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Anti-entropy egress baseline: two of three concurrent sync loops have zero counters — honest gap, not a percentage"
slug: "2026-07-27-anti-entropy-egress-baseline"
written: "2026-07-27"
author: "claude (freenet NOW slice Task 4 — sonnet measurement pass)"
status: "open"
priority: "medium"
tags: [anti-entropy, sync, gossip, automerge, kitsune2, inventory-gossip, metrics, prometheus, egress, observability-gap]
cites:
  - genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
  - genesis/docs/superpowers/plans/2026-07-27-freenet-now-slice-plan.md
---

# Anti-entropy egress baseline: two of three concurrent sync loops have zero counters

## Why this exists

We run three concurrent, unbudgeted anti-entropy loops — kitsune2 gossip
(120–300s cadence), inventory gossip (60s), Automerge doc-sync (60s) — with
no aggregate bandwidth accounting across them. Freenet's own single 300s loop
measures 53.7% of their total egress; we have no equivalent measurement, and
the absence is not benign: the 2026-07-20 adam melt
(`genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md`)
was a gossip storm transmitting link-quality data into `holochain_sqlite`
write-lock contention — exactly the failure mode aggregate loop accounting
would have surfaced earlier. This doc is the honest baseline that prevents
recurrence: what is measured, what is not, and what must not be inferred from
the gap.

**Method:** static grep of the elohim-storage metrics surface
(`src/http.rs`, `src/metrics.rs`, and call-site greps across `src/p2p/`,
`src/p2p_iroh/`, `src/sync/`) to enumerate which loops have counters at all,
then live Prometheus queries (`mcp__observability__query_prometheus`,
`list_prometheus_metric_names`) against the `elohim-alpha` namespace to pull
24h per-node rates for whatever exists. Queried 2026-07-27 against datasource
`prometheus`, nodes: `elohim-adam-alpha-0`, `elohim-eve-alpha-0`,
`elohim-gertrude-alpha-0`, `elohim-james-alpha-0`, `elohim-jessica-alpha-0`,
`elohim-matthew-alpha-0`, `elohim-susan-alpha-0` (7 pods — all alpha-cluster
elohim-storage StatefulSet pods with a `/metrics` scrape target under
`namespace="elohim-alpha"`).

## Instrumentation status per loop

| Loop | Cadence | Counter(s) exist? | Evidence |
|---|---|---|---|
| **Automerge doc-sync** | 60s | **Yes** | `elohim_sync_rounds_total`, `elohim_sync_requests_total{kind}`, `elohim_sync_request_outcomes_total{result}`, `elohim_sync_docs_enumerated_total` — all registered in `src/metrics.rs` (`register_all()`), called from `src/p2p/mod.rs` (`inc_sync_round`, `inc_sync_request("announce_change"\|"list_documents"\|"sync_changes")`, `inc_sync_request_outcome`) |
| **Inventory gossip** (`elohim/inventory/blob` gossipsub topic) | 60s | **No** | `grep -n "metrics::" src/p2p/inventory_gossip.rs src/p2p/inventory_broadcaster.rs` — zero matches. No reader anywhere in the module. |
| **kitsune2 gossip** (Holochain conductor-internal DHT gossip) | 120–300s | **No** | kitsune2 runs inside the embedded conductor process, not elohim-storage; no `elohim_*` metric name references kitsune/gossip. The one storage-side substrate-gossip module (`src/p2p/conductor_agent_info_gossip.rs`, propagating `AgentInfoSigned` at a 60s heartbeat — a *different* gossip than kitsune2's own DHT gossip) also has zero `metrics::` calls. `GET /db/p2p/conductor-diagnostics` (`src/http.rs`) exposes conductor peer-store/transport state as ad hoc JSON, not a Prometheus series — not queryable via `query_prometheus`. |

**Live Prometheus confirmation** — `list_prometheus_metric_names` with regex
`elohim_.*(sync\|gossip\|inventory\|automerge\|kitsune)` returned exactly:
`elohim_sync_docs_enumerated_total`, `elohim_sync_request_outcomes_total`,
`elohim_sync_requests_total`, `elohim_sync_rounds_total`. A second query with
regex `.*(gossip\|inventory\|kitsune\|automerge).*` (no `elohim_` prefix
restriction, in case a differently-named series existed) returned only
Grafana/Alertmanager's own internal gossip metrics
(`alertmanager_nflog_gossip_messages_propagated_total`, etc.) — nothing
elohim-owned. **Inventory gossip and kitsune2 gossip are unmeasured — no
counter exists for either, confirmed both statically and live.**

## Automerge doc-sync: the measured loop (24h, 2026-07-27, per pod)

`sum by (pod) (increase(elohim_sync_rounds_total{namespace="elohim-alpha"}[24h]))`:

| Pod | Sync rounds / 24h | Implied cadence |
|---|---|---|
| elohim-james-alpha-0 | 1432.7 | ~60.3s |
| elohim-matthew-alpha-0 | 1430.5 | ~60.4s |
| elohim-jessica-alpha-0 | 1429.6 | ~60.4s |
| elohim-susan-alpha-0 | 1418.8 | ~60.9s |
| elohim-adam-alpha-0 | 1429.7 | ~60.4s |
| elohim-gertrude-alpha-0 | 1425.4 | ~60.5s |
| elohim-eve-alpha-0 | 1427.5 | ~60.4s |

Cadence is consistent with the documented 60s Automerge doc-sync interval
(this cross-check is the corroboration that these counters attach to the doc-sync
loop and not one of the other two).

`elohim_sync_requests_total{kind}` (24h increase, by pod):

- `list_documents`: 29.6k–38.8k per pod — the dominant request kind by far.
- `sync_changes`: 86–154 per pod.
- `announce_change`: only nonzero on `elohim-matthew-alpha-0` (240.5/24h);
  zero (no series) on the other six pods in this window. This is consistent
  with Task 5's framing (`ListDocumentsSince`/`AnnounceChange` historically
  under-constructed on the send side) — worth a follow-up read on whether
  `announce_change` is only invoked from one node's code path or whether the
  other six simply had nothing to announce in this window; not resolved here.

`elohim_sync_request_outcomes_total{result}` (24h increase, by pod): `ok`
dominates (25.8k–38.0k per pod), `timeout` is a real minority
(1063–3824 per pod — `elohim-susan-alpha-0` is the outlier high at 3824),
`io` is small (25–98 per pod), `connection_closed` is 0 on every pod that
reported it.

`elohim_sync_docs_enumerated_total` (24h increase, by pod): 24.5M–35.6M per
pod. This is a **count of documents enumerated**, not a byte count — it
confirms the enumeration volume the digest-shortcut (`inc_sync_in_sync`,
landing today) is meant to collapse toward O(1), but it cannot be converted
to egress bytes without a per-document wire-size figure, which is not
measured anywhere in this codebase.

**`elohim_sync_in_sync_total` — instrumented today, baseline pending first
deploy.** This is the new digest-equal-shortcut counter added in this same
slice's Task 1 (`src/metrics.rs::inc_sync_in_sync`, wired at the
`SyncResponse::InSync` arm in `src/p2p/mod.rs`). `list_prometheus_metric_names`
with regex `elohim_sync_in_sync_total` returned **zero series** — the code is
registered in `register_all()` but has not been scraped yet because it has not
reached a running conductor. A flat-zero value once it does appear (per the
doc comment on `inc_sync_in_sync`) would mean the shortcut never fires and the
optimization is inert; that check is future work, not this baseline.

## What is explicitly NOT claimed here

- **No aggregate bandwidth figure.** None of the four measured counters are
  byte counts — they are round counts, request counts by kind, outcome
  counts by result, and a document-enumeration count. There is no
  request-size or document-size multiplier recorded anywhere in
  elohim-storage, so no byte total can be derived from them without
  inventing a number.
- **No percentage-of-total-egress claim**, per the explicit instruction this
  baseline exists to honor. A generic per-pod network egress counter *does*
  exist at the cAdvisor/cgroup layer
  (`container_network_transmit_bytes_total{namespace="elohim-alpha"}`,
  measured 2026-07-27 at roughly 7–24 GB/24h per `elohim-*-alpha-0` pod), but
  it is undifferentiated — it sums HTTP API traffic, blob transfer, all P2P
  protocols (libp2p + iroh), and both unmeasured gossip loops into one
  number. There is no way to subtract out kitsune2's or inventory gossip's
  share from that total, so computing "doc-sync is X% of egress" — or worse,
  extrapolating a figure for the *unmeasured* loops — would be exactly the
  retry-masked/partial-denominator metric trap the Freenet survey names.
  This total is recorded here only as context for scale, not as a
  denominator for any loop-specific percentage.
- **Inventory gossip and kitsune2 gossip egress: unmeasured — no counter
  exists.** This is the finding, not a placeholder for one. Any number
  offered for either loop's share of traffic would be invented.

## Follow-up (not this task)

- Adding counters to `src/p2p/inventory_gossip.rs` / `inventory_broadcaster.rs`
  (message count + payload byte count per send/receive) would close the
  inventory-gossip gap with the same idiom already used for doc-sync.
  kitsune2 gossip is conductor-internal and would need either a conductor-side
  metrics tap or admin-RPC-derived gauges surfaced through
  `handle_conductor_diagnostics` and re-exported as a Prometheus series —
  a materially larger lift than the inventory-gossip gap.
- Once `elohim_sync_in_sync_total` has scrape history post-deploy, re-query
  it against `elohim_sync_rounds_total` to compute the digest-shortcut hit
  rate — this is the number Phase 3 (capability-relative budget/eviction, per
  the parent slice plan) is blocked on.
