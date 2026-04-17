# Design Breadcrumb: Peer Self-Bootstrap From DHT

**Status:** Breadcrumb only — awaits a full planning session.
**Date:** 2026-04-17

## The idea

A peer with valid keys should be able to reconstitute its account projection (SQLite) **directly from the DHT**, without depending on doorway or an external seeder/import flow. DHT is truth; SQLite is projection. Today the import path is push-only (seeder → `/account/import` → storage → DHT). The recovery path should be pull-only (peer → DHT → storage SQLite).

## Why this matters

- **Fewer dependencies in crisis.** A human recovering access shouldn't need doorway to be healthy — their keys + any reachable peer subset should suffice.
- **Consolidates the resilience story with Track A.** The seeder split (`genesis/plans/...multi-seed partition...` — see context below) diversifies *authorship*. Peer self-bootstrap diversifies *recovery*. Together, no single peer is the forceable genesis OR the forceable on-ramp.
- **Verifiable by construction.** Content-addressed CIDs mean the pulled state is provably identical to authored state — no trust in the source doorway required.

## Existing breadcrumbs to pick up in the full planning session

### The epic (north star)
- `CLAUDE-PICKS.md` §1 "Recovery Protocol - Sovereignty Through Embeddedness" — the epic summary. Phase 1 (shard tracking) complete; phases 2-5 remain (recovery request flow, shard reconstruction, work-while-recovering, verification).

### The existing architecture (what's already designed)
- `doorway/doorway-service/RECOVERY-PROTOCOL.md` — full social recovery design: 4 layers (Human Identity in imagodei DNA, Shard Distribution via node-registry DNA, content reconstruction, agency continuity). Design principles list "Recovery while rebuilding" and "Human failure modes, not just hardware."
- `doorway/doorway-service/RECOVERY-SPRINT-PLAN.md` — phased implementation plan for the recovery protocol.
- `doorway/doorway-service/src/orchestrator/disaster_recovery.rs` — Rust-side orchestrator code.
- `doorway/CLAUDE.md` — "Doorways are **projections of the DHT**, not authorities" — the architectural principle this breadcrumb extends to all peers, not just doorway operators.

### The frontend is already there
- `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts` — frontend coordinator at 98.7% coverage.
- `app/elohim-app/src/app/imagodei/components/recovery-request/`, `recovery-interview/` — UI components for initiating recovery.
- `app/elohim-app/src/app/imagodei/models/recovery.model.ts` — domain models.

### The storage-as-projection principle is already declared
- `elohim/elohim-storage/CLAUDE.md` — "snake_case never leaves the Rust boundary" is its headline rule, but the broader posture is that elohim-storage *is* the projection layer. The import path exists because genesis bootstrap needs a push mode; recovery is the pull mode that hasn't been fully wired.

### Track A connection
- `.claude-config/plans/keep-researching-so-the-calm-moon.md` — the sibling plan (seeder split across matthew + adam). That plan's design posture ("Each peer should receive its slice and negotiate the rest via normal P2P replication") is the symmetric twin of this one's ("A peer should reconstitute its slice from DHT without needing a seeder or doorway"). Both say: *nothing in the protocol should be forceable by one party*.

## Open questions for the full planning session

- What account state is **truth on DHT** vs **cached-only in SQLite**? The `AccountPackageInputView` sent through `/account/import` contains identity, content, relationships, stewardship, collectives. Are all of these shaped as DHT entries today? Which fields are projection-only and would need a DHT representation before pull-mode works?
- How does the pulling peer **discover what belongs to it**? By agent pubkey? By following steward links from a known anchor? By the shard-assignment records from layer 3 of the recovery protocol?
- What's the **progressive fallback** order when DHT is only partially available? RECOVERY-PROTOCOL.md sketches "reconstruct from any 4 of 7 shards" — how does that fallback interact with SQLite-level hydration?
- **Work-while-recovering**: RECOVERY-PROTOCOL.md principle #2 says "you shouldn't wait for full restoration to start working." How does partial SQLite hydration expose a usable UI state?
- Is the existing `recovery-coordinator.service.ts` already modeling the pull-mode flow, or is it still assuming a doorway-mediated recovery? (Quick read of its coverage & tests should answer this — it's listed at 98.7% coverage, so a lot is already worked out.)

## Out of scope for this breadcrumb

Writing the actual pull-mode. This file is just the link graph to assemble when a full planning session starts.
