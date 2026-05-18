---
name: living-doc-honesty-matrix-maintenance
description: A chapter that anchors load-bearing claims via a LIVE/DESIGNED/GAP matrix is only honest if the matrix migrates as the underlying code lands. The resilience epic's Part IX gap matrix names ~80 rows; every signal_kind whitelist edit, ReconcileController handler wiring, feature file passing on alpha, schema additions etc. should bump the matrix to reflect new state. Without migration discipline, the chapter's honesty rots and the trillion-dollar-civic-claim becomes overclaim.
metadata:
  type: feedback
---

When a foundational chapter anchors its civic claim via a "what is built / what is designed / what is gap" matrix, the matrix is **the chapter's load-bearing honesty mechanism**. If the matrix doesn't migrate as the underlying substrate work lands, the chapter's claims rot from defensible to misleading without anyone editing the prose.

**The pattern:** Resilience epic Part IX (`genesis/docs/content/elohim-protocol/resilience/README.md`) names ~80 rows across 9 layers: Notary, Data Ops, Reconciliation, Storage Projection, Topology, Reach+Trust+Standing, Attribution+ContributorPresence, Distribution/Succession/T&S Civic Substrate, Resilience-Specific, Hub+Operator. Each row claims LIVE / DESIGNED / GAP with a code-path or design-doc citation. The chapter explicitly says "the chapter is responsible only for the claims that compose" — so when something composes more than the row claims, the row must be migrated.

**Migration triggers** (events that should cause matrix updates):
- Any `signal_kind` whitelist edit in `content_store_integrity/lib.rs` → check rows naming missing signal_kinds
- Any new `resource_classified_as` entry → check classification rows
- New role record in `genesis/data/lamad/content/` → check role-record gap rows
- Feature file moving from `@wip` to passing on alpha → matrix row migrates to LIVE
- `ReconcileController` handler added → check P1 / reconciliation rows
- New view module / `services/*.rs` file → check topology / projection rows
- Phase 12 (iroh peer_transport_manifest) wiring tasks landing → check transport rows
- Reach enum reconciliation landing (see `project_reach_enum_drift_reconciliation`) → check reach rows
- Recognition transfer-on-claim mechanism implemented → check ContributorPresence row
- Substrate-native node-health observable landing → check the load-bearing GAP row in Reconciliation layer
- Compute-class vs attribution-class typed taxonomy split → check Notary layer

**How to apply:**
- Memory-ceremony: include "review the resilience epic's gap matrix against the substrate as it now exists" as a standing ceremony beat. The chapter is foundational; a stale matrix means the foundation looks more wobbly (or less wobbly) than it actually is.
- Cartographer: when scoring "what's next" against vision × readiness, the matrix is one of the readiness inputs. A row migrating from GAP to LIVE doesn't just close a backlog item — it removes a hedge from the trillion-dollar civic claim.
- Operator commits that land matrix-named work: include a note in the commit message saying which matrix row(s) should migrate, so the chapter author has a clear trigger.
- Optional CI discipline: a script that greps code/schema for the specific tokens named in matrix rows (e.g. `"recovery-share-custody"` in `feedback_signal.rs`) and warns when matrix says GAP but the token exists in code. Low-cost guard against silent rot.

**Open thread:** The matrix as of 2026-05-18 is current. No CI discipline yet. Migration happens via human review during ceremony. Worth adding the lightweight grep-based CI gate in a future hygiene pass.
