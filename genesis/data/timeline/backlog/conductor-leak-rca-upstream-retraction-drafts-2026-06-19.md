---
id: backlog-conductor-leak-rca-upstream-retraction-drafts
kind: backlog
title: "Conductor-leak RCA correction — upstream retraction drafts (posted comments referenced a now-refuted RCA)"
status: open
priority: medium
tags: [conductor-leak, upstream, tx5, rca-correction, retraction, self-heal]
occurred_at: 2026-06-19
---

# Conductor-leak RCA correction — action artifact (2026-06-19)

The alpha conductor OOM was diagnosed as a tx5/go-pion off-heap transport leak; that RCA is
**refuted**. Truth: a native **glibc-malloc arena retention** in the holochain child, **cured by
swapping the global allocator glibc→jemalloc** (flat ~2.1–2.9 GB past the old ~5 h OOM cadence; DNA
hash unchanged; go-pion exonerated — Go heap flat ~52 MB). Shipped: fork `b477ca7` + che-dw
`ca69302` → build #13 `elohim-edgenode:latest` jemalloc-prod; Part C `ed111a5cc`. Truth docs:
`genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md`, `genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-prod-changeset.md`,
`genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-diverse-eyes-synthesis.md`, `genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md`.

Produced by the `wrong-rca-correction-sweep` workflow (one sub-agent, `sweep:backlog`, timed out →
the 3 timeline/backlog verdict records are an ADDENDUM below, corrected by hand).

---

## A. INTERNAL CORRECTIONS — banners to prepend (status: PENDING APPLY)

1. `genesis/docs/superpowers/plans/2026-06-17-conductor-leak-fork-patch-debug-plan.md` → SUPERSEDED (whole-plan premise wrong; fork infra became the jemalloc-build vehicle, but Stages 2–4 target the wrong mechanism).
2. `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md` → CORRECTED ("WAIT on upstream tx5 fix" moot; cured locally by allocator swap; arc-factor falsification still holds).
3. `HANDOFF-2026-06-17-fbootstrap-deploy-gate.md` (§"REAL leak fix", lines 118/125–131) → leak-RCA corrected; **F-BOOTSTRAP fix itself STANDS**.
4. `.claude/handoffs/archive/HANDOFF-2026-06-17-conductor-leak-hunt.md` (§3.1/H4) → H4 (glibc allocator) was the closest hypothesis; trail down-ranked it wrongly.
5. `genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md` (line 314/286) → leak-axis corrected; **control-plane design STANDS**.
6. `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md` (line 26) → leak-claim corrected; **arc-policy design + {0,1} infeasibility STAND**.
7. `genesis/docs/superpowers/plans/2026-06-14-dataplane-arc-plan.md` (line 123) → leak-gate resolved; **{0,1} actuation + corpus-scaling STAND**.
8. `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md` (~18/31–38/64) → alpha-OOM evidence void; corpus-scaling rationale stands on first principles.
9. `genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md` (~line 193) → "cure commit `2af2607e7`" corrected (that's the tx5 #194/#199 lineage that did NOT cure); precondition now met by jemalloc; Tasks 1–2 STAND.

### ADDENDUM — backlog verdict records (sweep:backlog timed out — correct by hand)
- `genesis/data/timeline/backlog/conductor-anon-leak-mechanism-smaps-verdict.md`
- `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md`
- `genesis/data/timeline/backlog/arc-shrink-ineffective-memory-soak.md`

Banner spine for all (date 2026-06-19): real cause = glibc-malloc arena retention in the conductor
child; tx5/go-pion (#194/#199, deployed fleet-wide → leak persisted) and arc-factor refuted; cure =
glibc→jemalloc allocator swap (flat past OOM cadence; DNA hash unchanged); cite the 4 truth docs.
Preserve any still-valid methodology (smaps/cgroup discriminator) or still-valid sub-findings.

---

## B. UPSTREAM COMMENT CORRECTIONS — DRAFTS (status: AWAITING OPERATOR SIGN-OFF; do NOT post unprompted)

Post identity question: `EthosengineBot` (GH_TOKEN account) vs personal. Each retracts a *distinct*
original claim and preserves #194/#199 + PR #207 as valid connection-hygiene work.

### Thread 1 — holochain/holochain #5664
Target: https://github.com/holochain/holochain/issues/5664#issuecomment-4732914987

> **Correction to my earlier comment above** ([#issuecomment-4732914987](https://github.com/holochain/holochain/issues/5664#issuecomment-4732914987)). I attributed our 0.6.x OOM to the **tx5/go-pion** transport (zombie PeerConnections), reasoning that off-heap anon growth alongside a flat Rust `[heap]` implied the Go/CGo runtime. **That inference was wrong, and I want to correct the record.**
>
> We built and deployed the tx5 zombie-teardown fix (#194 + #199) fleet-wide, binary-verified — and **the conductor OOM persisted unchanged.** So we read per-VMA `/proc/<pid>/smaps`, which we should have done first. The growing anon is in **glibc malloc secondary arenas** (the `0x77xx` mmap region), serving the conductor's native **Rust/C** allocations. The **Go heap (`0xc0…`) was flat ~52 MB the entire time** — never the leak. My error: glibc's secondary arenas are anon `mmap` classified `other`, never the brk `[heap]`, and Rust-on-Linux defaults to glibc malloc — so "off-heap anon + flat `[heap]`" is the *expected* signature of native Rust/C allocation, not evidence of Go. I misread a textbook native-malloc shape as a Go shape.
>
> The cure: swapping the conductor's global allocator to **jemalloc** (with unprefixed-malloc interposition so C-side allocs route through it too) flattened it — stable for many hours past the prior ~5 h OOM cadence. That points to glibc retaining freed memory in chained arenas (an allocator-fit issue), not a never-freed tx5 leak. I leave the iroh `magicsock` VecDeque caveat untouched — that may well be a separate mechanism.

### Thread 2 — holochain/tx5 #196
Target: https://github.com/holochain/tx5/issues/196#issuecomment-4732919972

> **Correction to my earlier comment** ([#issuecomment-4732919972](https://github.com/holochain/tx5/issues/196#issuecomment-4732919972)). I confirmed Behavior 1 as a real production **off-heap memory leak** and asked whether a tx5 patch release of #194/#199 was warranted to resolve it. The memory-OOM part of that was mistaken.
>
> We deployed #194 + #199 fleet-wide (binary-verified) and the conductor's off-heap OOM **persisted unchanged** — so the zombie-PeerConnection mechanism was not the cause of *our* OOM. Reading per-VMA smaps, the growing anon is in **glibc malloc secondary arenas** (the `0x77xx` mmap region) serving the conductor's native Rust/C allocations; the **Go heap was flat ~52 MB throughout.** (My earlier "flat Rust heap ⇒ Go runtime" read was wrong — glibc arenas are anon `mmap` classified `other`, and Rust defaults to glibc malloc, so off-heap anon is expected for Rust/C.) Switching the conductor's global allocator to **jemalloc** (unprefixed-malloc interposition for the C side) flattened it for many hours past the prior cadence — consistent with glibc retaining freed memory, not a never-freed tx5 leak.
>
> To be clear, this **doesn't make Behavior 1 a non-issue** — connections lingering after a disconnected/failed event still looks like a genuine liveliness problem, and #194/#199 are a worthwhile dead-peer-reaping fix on their own merits. I just no longer have a memory-OOM argument for prioritizing a release; I defer entirely to your judgment there. We'll keep carrying #194/#199 downstream for connection hygiene.

### Thread 3 — holochain/tx5 #207
Target: https://github.com/holochain/tx5/pull/207#issuecomment-4732920114

> **Correction to my earlier comment** ([#issuecomment-4732920114](https://github.com/holochain/tx5/pull/207#issuecomment-4732920114)). I said we'd root-caused our prod OOM to the `Evt::State(_) => ()` gap in `go_pion.rs` (dead-peer PeerConnections never freed). **That root-cause attribution was wrong** and I want to set it straight here.
>
> We deployed #194 + #199 fleet-wide, binary-verified, and the off-heap OOM **persisted unchanged** — so the zombie-PeerConnection path was not what was driving our leak. Reading per-VMA smaps showed the growing anon living in **glibc malloc secondary arenas** (the `0x77xx` mmap region) serving the conductor's native Rust/C allocations; the **Go heap (`0xc0…`) stayed flat ~52 MB.** My "flat Rust heap ⇒ Go" inference was the mistake — glibc arenas are anon `mmap` classified `other` (never the brk `[heap]`), and Rust-on-Linux uses glibc malloc by default, so off-heap anon there is normal. Switching the conductor's global allocator to **jemalloc** (with unprefixed-malloc interposition) flattened it for many hours past the prior OOM cadence — pointing to reclaimable glibc-pinned retention, an allocator-fit issue, rather than a never-freed code leak in tx5.
>
> None of this diminishes the work in this PR. The `peer_map` cleanup, `wait_for_ready` timeout, and the 32-vs-1024 channel-capacity fix are real connection-management hardening, and #194/#199 are a worthwhile liveness fix — we're keeping them downstream. I just can't honestly attach a memory-OOM justification to them anymore, so I'll leave prioritization to you. Apologies for the noise on the original attribution.

---

## C. NO-ACTION (audited correct/leak-independent)
doorway-metrics handoff · the 2026-06-18 resilience-cards sprint handoff+RESULT (already correct) ·
conductor-memory-attribution-instrument-plan (correct methodology) · design-decision-toolkit-plan ·
upstream-self-protection / inbound-admission-backpressure / stability-surface-read-model plans ·
node-resource-tunables spec · dataplane-actuation/proofs plans · P2P-DATAPLANE CONTRACT-LEDGER
(correctly defers) · node-consolidation plan (go-pion = Docker FROM only) · `genesis/docs/content/elohim-protocol/history/2026-06-1*-conductor-leak-*`
(investigation-trail truth docs, relocated from `.claude/data/`) · brit/gix CHANGELOG false-positives.
