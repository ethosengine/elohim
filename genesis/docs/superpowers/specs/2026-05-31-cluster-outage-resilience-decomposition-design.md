# Cluster-Outage Resilience — Decomposition Design

**Date:** 2026-05-31
**Status:** Decomposition spec (umbrella). Each track gets its own design → plan → implementation cycle.
**Forcing function:** Live outage of the on-prem cluster while operating from a laptop. The outage exposed that `elohim.host` / `alpha.elohim.host` cannot serve the landing page + core LMS path from the P2P dataplane when half the cluster is down.

---

## 1. Purpose & non-goals

**Purpose.** Decompose "make `elohim.host` and `alpha.elohim.host` survive half the cluster going into outage" into independently-shippable tracks, fix their interfaces, and state the build order — so the tracks can proceed in parallel without drifting.

This is an **iteration on substrate we have already built** (REA storage-replication commitments, reach grading, RS sharding, delivery race-fetch, pkarr discovery). It is **not** a new replication system. The canon constraint holds: **zero new DHT entry types** (`genesis/docs/architecture/rea-compute-commitment-primitive.md`).

**Non-goals (this umbrella).**
- Not designing entity shapes — that happens in each track's own spec (behind the `p2p-design-gate`).
- Not relocating Adam or decoupling Shem from the cluster (see §2).
- Not building the Shem failover-routing deployment now (designed-for, deferred — see §6).

---

## 2. Target topology

The survivable target is **two anchors in two failure domains**, with the on-prem hub authoritative:

| Anchor | Role | Hosts | Failure domain |
|--------|------|-------|----------------|
| **Family hub (on-prem Dowell)** | **PRIMARY** (authoritative) | Matthew, Jessica, James — hostpath StatefulSets, MUST-NOT-land-on-shem | Household power/ISP |
| **Shem (remote microk8s)** | **Replication tier** | Adam & company (non-family personas, shem-pinned) | Remote DC + cluster-comms link |

**Design decisions (per operator, 2026-05-31):**

1. **On-prem is primary; Shem holds the replication stories.** We do **not** decouple Shem entirely — it stays integrated because we need it for deployment/test modeling.
2. **Adam is not relocated.** The 2026-05-27 placement directive (Adam shem-pinned, no on-prem fallback) stands. Survivability comes from *content being replicated on both sides* (Track A), not from moving Adam.
3. **Two survival directions, both via Track A content replication:**
   - *On-prem hub down* → Shem/Adam serves (requires Shem to hold commons + core — Track A).
   - *Cluster-comms / Shem control-plane down* → on-prem serves (primary), **and** orphaned-but-running Shem pods stay externally reachable via a **failover routing layer** (future — see §6).
4. **The failover routing layer** = a lightweight front-door in front of Shem's microk8s, supplying an **independent WAN IP/DNS**, so external traffic reaches running-but-orphaned peers when cluster communication drops. This is a *deployment of the Track B crate*. **Deferred** from current scope, but it is *why* Track B is built now.
5. **k8s recedes** over time to modeling peer hardware + chaos scenarios ("kill half the peers", "sever Shem comms"), not the production lifeline.

---

## 3. Substrate we iterate on (do NOT reinvent)

### 3.1 Implemented and working

| Capability | Where | Notes |
|------------|-------|-------|
| `replicates-dwelling` commitment | `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:134-193`; schema `elohim/sdk/schemas/v1/commitments/replicates-dwelling.schema.json` | 11 required fields incl. `scope_filter` (epr_kinds, bytes_per_blob_max, requires_attestations, kinds_excluded), `ratio_attestation`. |
| Commitment → fetch projection | `elohim/elohim-storage/src/services/replication_prioritizer.rs:98-133` | `score_advertised_blob` → High/Skip by recipient hub + scope. |
| Bilateral mutuality audit | `elohim/elohim-storage/src/services/mutuality_audit_service.rs:55-115` | Classifies Matched/Pending/Breached; `reciprocity-imbalance` FeedbackSignal (debit_weight=8, decay_days=60). |
| REA entry types | Elohim DNA `content_store_integrity/src/lib.rs` (Agreement, Commitment, EconomicEvent, Intent, EconomicResource); Mishpat::Commitment (`delegates-compute`, `replicates-dwelling`) | `RESOURCE_CLASSIFICATIONS` includes `compute`, `storage`. In-kind = `medium_of_exchange_id` NULL. |
| Reach grading | `elohim/sdk/schemas/v1/enums/reach.schema.json`; `genesis/seeder/src/seed-sqlite.ts:499-604` | 8-tier `REACH_ORDER` (private:0 → commons:7). Authored floor + raise-only. Landing+core are `reach: commons`. |
| Commons → anonymous access | storage gate; `genesis/a2o/features/auth/reach-commons.feature` | `reach == commons \|\| public` → anonymous GET 200. |
| Constitutional donut | `content_store_integrity/src/lib.rs:109-114`; manifest `constitutional_ratio_registry` | DNA-locked floor/ceiling per tier (COMMONS_MIN_FLOOR_PCT=10, …). |
| RS sharding + delivery | `elohim/elohim-storage/src/sharding.rs`; `doorway/.../shard_resolver.rs` | RS-4+3 (>64MB); GET /blob → local → race-fetch peers via `/elohim/shard` → reconstruct (any 4 of 7). |
| pkarr discovery (in-tree) | `doorway/.../routes/pkarr_resolver.rs`; `genesis/manifests/doorway-pkarr-resolver.yaml`; `genesis/docs/.../2026-05-08-iroh-libp2p-complementarity.md` | Self-hostable `/pkarr/{z32-key}`; named as the external-dns/Cloudflare replacement (peer-discovery layer). |
| DDNS/ACME stubs | `steward/node/src/dashboard/setup.rs` (`configure_ddns`, `configure_https`) | Empty TODO match arms: Cloudflare/DuckDNS/No-IP/ddclient; ACME/LetsEncrypt. |

### 3.2 Reserved / stubbed / missing (the actual gaps)

- `replicates-commons` and `replicates-collective` actions — **schema-reserved, zero logic.**
- **No transitive / meta-replication** — `scope_filter` selects by content properties only; cannot express "replicate everything hub B is committed to."
- `mutuality_audit_service::find_counter` returns `None` (stub); `emit_reciprocity_imbalance` is log-only — mutuality **doesn't actually audit yet**.
- **No genesis-pair 2-replica guarantee** — seeder uploads to a single `DOORWAY_URL` (`genesis/seeder/src/seed.ts:459`); "Plan 1: distribute-at-ingest" is unimplemented.
- **"Half the cluster" unsurvived** — RS-4+3 tolerates ≤3 custodian losses; no cross-blob shard-diversity orchestration; no auto re-sync on node return; no CustodianAssignment TTL renewal; verifier sweeps (Plan 2) + reconstruction orchestration (Plan 3) not deployed.
- Doorway has **no in-process TLS** (plain HTTP :8080 behind ingress) and **no DNS self-registration** (`DOORWAY_HOSTNAME` read-only, defaults localhost).
- Clients **hardcode the cluster**: `BOOTSTRAP_DOORWAYS = doorway-alpha.elohim.host`, signal `wss://signal.elohim.host`, DID gateway `gateway.elohim.host`.

---

## 4. The three tracks

### Track A — Reciprocal replication guarantee  *(CURRENT SCOPE — primary focus)*

**Goal.** The PRIMARY (on-prem) and the replication tier (Shem/Adam) mutually hold (a) the commons (landing + core path) and (b) each other's dwelling sets, so whichever side is up can serve.

**Composes on:** `replicates-dwelling`, reach/commons, constitutional donut, RS sharding, delivery race-fetch. **Zero new DHT entry types.**

**Sub-scope:**
- **A1 — Bootstrap + finish the reciprocal pact.** Author the reciprocal `replicates-dwelling` pair (Adam-hub ↔ Matthew-hub) and **wire the stubs** `find_counter` + `emit_reciprocity_imbalance` so bilateral mutuality actually audits.
- **A2 — Land `replicates-commons`.** So commons-reach content (landing+core, manifesto) is held by every participating hub via the donut floor — surviving **any** node loss, not just the Adam↔Matthew pair. (Resolves the "floor-via-declaration → floor-via-pledge" gap.)
- **A3 — Close the genesis 2-replica gap.** Plan 1: distribute-at-ingest — seeded commons fans out to ≥2 anchors atomically rather than single-`DOORWAY_URL`.
- **A4 — Delivery resiliency.** Plan 2 (verifier sweeps) + Plan 3 (reconstruction orchestration) + cross-blob shard diversity so a fetch survives loss of an anchor.

**Open fork (deferred to Track A's own spec, behind `p2p-design-gate` + `rea-economics`):** does "replicate each other's commitments" mean **(i)** literal transitive meta-replication (a new commitment-set `scope_filter` selector — conflicts with property-only model) or **(ii)** reciprocal dwelling pact (A1) + commons custody floor (A2) composed from existing actions (canon-aligned, zero new entries)? Working hypothesis: **(ii)**.

**Interface produced:** "content C is held at anchors {X, Y}" (queryable custody/inventory).

### Track B — DDNS + ACME crate  *(CURRENT SCOPE — parallel, independently shippable)*

**Goal.** A runtime-agnostic `crates/` crate that lets any doorway/node self-register a stable public hostname + obtain/renew TLS, removing cert-manager/external-dns from the critical path. The enabler for the (future) Shem failover front-door.

**Lives in:** `crates/` (consumed by **both** `doorway-service` and `steward/node`), not `bridges/` (those are protocol-interop). Fills the existing `setup.rs` stubs by extraction.

**Sub-scope:**
- **B1 — WAN-IP discovery** (provider API / observed-addr / STUN).
- **B2 — Dynamic DNS update** (Cloudflare API first; pluggable provider trait for DuckDNS/No-IP/ddclient).
- **B3 — ACME issuance + renewal** (`rustls-acme` / LetsEncrypt).
- **B4 — In-process TLS termination** in doorway (today plain HTTP behind ingress).

**pkarr tension resolved:** traditional DNS+cert = browser-facing HTTPS (browsers can't resolve pkarr); pkarr stays the peer-discovery layer (Track C). Complementary.

**Interface produced:** "register/renew {hostname → this box's WAN}" + "serve valid cert for {hostname}", independent of k8s ingress.

### Track C — Discovery de-hardcode  *(DESIGNED-FOR — deferred)*

**Goal.** External traffic + clients reach surviving anchors when the primary path is down: de-hardcode `BOOTSTRAP_DOORWAYS`/signal/gateway → failover set + pkarr-backed resolution; signal/gateway served by any surviving anchor (env flags already exist).

**Depends on:** B (anchors reachable) + A (anchors hold content). Includes the **Shem failover-routing front-door** (§6).

---

## 5. Cross-track interfaces & contract

```
Track A ──"content C held at anchors {X,Y}"──▶ Track C (routing/failover decisions)
Track B ──"{X,Y} reachable at stable hostname+TLS"──▶ Track C
Track C ──"client/edge traffic lands on a surviving anchor"──▶ end-user serves landing+core
```

The **anchor-holds-content** contract is the spine: C must never route to an anchor A hasn't confirmed holds the content (else 404). A's custody/inventory query is the source of truth C consumes.

---

## 6. Out of scope now (designed-for, deferred)

- **Shem failover routing front-door.** Lightweight ingress/proxy in front of Shem's microk8s with independent WAN IP/DNS (a *deployment* of the Track B crate), keeping orphaned-but-running Shem pods reachable when cluster comms drop. This is the operator's stated future-modeling concern.
- **Full client-side discovery de-hardcode (Track C).**
- **Encryption envelope / per-recipient key custody** for replicated blobs (already deferred in the dwelling-hub spec).
- **Proof-of-storage** cryptographic primitive (trust-and-debit sufficient for now).

---

## 7. Build order & parallelization

| Track | Scope | Can start | Depends on | Parallel-safe |
|-------|-------|-----------|------------|---------------|
| **B** (DDNS+ACME crate) | now | immediately | — | ✅ fully independent |
| **A** (reciprocal replication) | now | immediately | — | ✅ (own subsystem) |
| **C** (discovery de-hardcode) | deferred | after B + A2 | B, A | partial (C1 failover-list ∥ A2) |

- **B starts now in parallel** — zero coupling, clean crate boundary, unblocks the future Shem front-door.
- **A is the current focus** — A2 (commons floor) and A1 (reciprocal pact) are the survivability core.
- **C deferred** — but A and B are designed to produce the exact interfaces C will consume.

---

## 8. Success criteria

**End-state (requires all three tracks):**

1. **Outage drill:** with Shem comms severed (or the on-prem hub down), `alpha.elohim.host` still returns HTTP 200 for the landing page + core path (`elohim-protocol`) to an anonymous client. *(Track A content + Track B reachability + Track C routing — the full goal.)*

**Current-scope milestones (A + B):**

2. **Mutuality real:** the Adam↔Matthew reciprocal `replicates-dwelling` pair audits as `Matched` (not a `find_counter` stub), and a withdrawn counter produces a real `reciprocity-imbalance` signal. *(Track A1.)*
3. **Commons floor:** every participating hub provably holds commons-reach content per the donut floor; killing any single anchor never loses landing+core. *(Track A2.)*
4. **Ingest fan-out:** seeding commons content lands on ≥2 anchors atomically, not a single `DOORWAY_URL`. *(Track A3.)*
5. **Off-cluster reachability (crate-level):** a doorway can obtain a stable hostname + valid TLS via the crate with no k8s ingress/cert-manager. *(Track B.)*

Criterion #1 is the end-state proof the whole effort builds toward; #2–#5 are the current-scope deliverables. Each should land as an a2o regression scenario in its track's spec.

---

## 9. Next step

Proceed to **Track A** deep design (this session). Before proposing entity/route shapes, invoke `p2p-design-gate` and the `rea-economics` reference. Track B can be picked up in parallel (own spec).
