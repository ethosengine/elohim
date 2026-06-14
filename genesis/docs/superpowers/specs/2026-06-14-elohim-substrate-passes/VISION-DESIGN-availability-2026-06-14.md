# VISION DESIGN PASS — Availability vs Truth: collectives keep serving when a steward falls

> Working draft for operator blessing. NOT cite-sealed. Escalates the path/pivot the vision requires, not the immediate blocker.
> Supersedes the tactical 2026-06-14 SPRINT-KICKOFF framing for the D6 / apex-failover question.
> Read against: `project_doorway_ab_edge_islanding`, `project_inventory_exchange_not_byte_replication`, `project_principle_p1_reconciliation_controller`, `project_per_node_memory_is_conductor_authority_arc`, and the tactical F-COHERENCE plan (`genesis/docs/superpowers/plans/2026-06-14-federation-coherence-plan.md`) — which delivers a *detector* and is the floor this pass builds the *dataplane* on.

---

## Headline

**Reframe: "apex failover" is not a load-balancer problem — it is a missing dataplane plane.** The byte plane already keeps serving when a steward falls (content-addressed RS(4,7) + race-fetch + heal-on-read; the chaos suite proves dead-provider-fails-fast, failover-to-survivor, returning-provider, kill-mid-transfer). The **head plane does not.** When `matthew` falls, `doorway-alpha` serves nothing; when `elohim.host`/`adam` is the only survivor, it serves *its own local projection* of the EPR head with no proof that head is the network's latest, and no detection if it has diverged. Availability is a byte-plane property today and an edge-LB accident at the head. The vision demands the inverse: **the head must be a content-addressed, signed, quilt-replicated object that any surviving peer can serve and any client can verify — so a down hub degrades to "served by a sibling" without the apex ever silently serving a stale or divergent truth.**

---

## 1. What the VISION REQUIRES here

The north star clauses this scope is accountable to, and what each demands of availability:

- **"collectives continue to serve the humans that use it"** — when a steward/hub falls, the humans that collective serves keep reading. Not "the operator reboots a pod"; the *quilt itself* routes around the loss. This is the explicit D6 facet.
- **"maintains that high-integrity of the Holochain DHT … that allows people to build trust on the values negotiated through it"** — availability must NEVER be bought with integrity. Serving a stale or divergent head to keep the lights on is a capture vector: it lets a partitioned/compromised edge present a forked reality as canonical. **Capture-resistance ⇒ a surviving peer may serve only a head it can prove is current, or it must say "catching up" — never confidently serve a fork.**
- **"on a quilt-tier's replicated dataplane"** — the *unit of availability* is the quilt, not the edge. The head is content the quilt should carry, exactly as it carries bytes.
- **"backed by those mutual compute agreements" / "fractal stewards"** — who serves the failover is not arbitrary; it is whoever holds a **coverage commitment** for that content's reach. Failover is the redemption of a commitment, not luck-of-the-mesh.
- **"hubs — households to factories — that scale the sensemaking"** — a hub falling must degrade the *sensemaking surface* gracefully (read-still-works; write-and-governance "catching up"), not collapse the collective.
- **"capture-resistant stasis against the real world, its externalities, and its messiness"** — pod crash-loops, deploy skew, DNA-hash partition, a hostile edge: all "messiness." The system stays in a *truthful* stasis through all of them — degraded-but-honest beats available-but-lying.

The one-line synthesis: **availability is a dataplane property of a signed, quilt-replicated head, redeemed against reach-scoped coverage commitments, and bounded by a freshness proof that fails closed to "catching up" rather than open to a fork.**

---

## 2. Is the substrate CAPABLE? Dig to WHY, the exact layer.

### CAPABLE today — the byte plane is already a vision-grade availability primitive.

- **Content-addressing + RS(4,7):** `sharding.rs:125` (`determine_encoding`), `:301` (`reconstruct` — any 4 of 7 shards). Loss of 3 of 7 holders is a non-event.
- **Race-fetch failover:** `p2p/blob_fetch.rs:64` `race_fetch` — first verified responder wins, dead peers fail fast (per-peer `tokio::time::timeout`), batch advances. Hash-verified on arrival (`:130` `verify_blob_hash`).
- **Heal-on-read with connected-fallback:** `http.rs:2293` `get_blob_or_heal` — local miss → inventory candidates → **connected-fallback** when inventory is empty (`:2360`, the post-pod-restart `InsufficientPeers` case). This is precisely "route around the down hub" at the byte layer.
- **Chaos proof:** `tests/chaos_dataplane.rs` — `chaos_failover_to_surviving_provider` (:286), `chaos_kill_during_transfer_is_bounded_and_failover_completes` (:341). The dataplane property the vision wants for the *head* is already proven for *bytes*.
- **Coverage-aware selection exists:** `services/peer_selection.rs:103` `select` reads `rea_commitments` (action `provide`, `state active`, `resource_classified_as = content:{reach}`) × liveness × household/archetype diversity. Failover-to-a-committed-steward is **already the placement model** — it is simply not yet wired as the *read-path* failover selector.

### NOT CAPABLE — the head plane. Two precise limits.

**Limit A — the EPR head is resolved from a node's OWN local projection, not the quilt.**
`doorway/src/routes/epr.rs:18` `handle_epr_head_request` forwards `GET /epr-head/{id}` to a **single** `storage_url` (correct per No-Fan-Out). Storage's `handle_get_epr_head` (`http.rs:1284`) serves from that node's local store. The EprHead struct itself (`epr_codec.rs:101`) carries `version, id, content (CID), lamad/shefa/qahal context, author, updated` — **but no signature and no predecessor link.** So:
- A surviving peer cannot *prove* its head is the latest — there is no freshness witness on the object.
- A client cannot *verify* the head it received is authentic and current — `author` is an unsigned string field; nothing binds the head bytes to the author's key at this layer (the EPR *envelope* in `epr/src/epr.rs:50` verifies signatures, but the gossip-friendly `EprHead` metadata envelope does not).
- Two edges can serve two different heads for the same `id` with **no detection** (`project_doorway_ab_edge_islanding`: the `e0352a7`/`8a2c65e` symptom — and the operator could not even tell content-skew from deploy-skew).

**Limit B — head availability has no quilt failover; it rides entirely on conductor↔conductor DHT gossip.**
The byte plane heals from any holder. The head plane has no analogue: each edge's `EprRouter` is seeded from its own `STORAGE_URL` only (F-COHERENCE plan §1; `epr_router.rs` boot + 30s refresh). There is no "fetch the head from a sibling who holds a fresher one" path, because (a) heads aren't content-addressed in a way that lets a client/peer ask "give me the head whose predecessor chain is longest," and (b) `predecessor_records.rs` lineage exists on the EPR *atom* transport (`p2p_iroh/epr_atom_backend.rs`, `back_prop::record_predecessor`) but is **not surfaced as a freshness comparator at head-resolution time.**

**Root cause, named:** the head is treated as *projection* (P1 reconciliation controller territory — DHT manifest → local Diesel projection) but consumed as *truth* at the edge. The projection is correct as a cache; it is wrong as the thing a lone survivor serves as canonical under partition. **The head needs to become a first-class quilt object with a freshness proof — same treatment the bytes already get.**

This is a layering artifact, exactly like the ARC worked example. The substrate already speaks every primitive required (content-addressing, signatures, predecessor lineage, coverage commitments, race-fetch); they are just not *composed* at the head-resolution layer.

---

## 3. PATH / PIVOT / FORK LADDER (cheapest → deepest)

**Rung 0 — Divergence DETECTOR (buildable NOW; already planned).**
The F-COHERENCE plan: per-edge head fingerprint + cross-edge compare + structured WARN alarm. Cat-C, detect-only, no reconcile.
- *Cost:* ~1 doorway crate, 6 tasks, in flight.
- *Blast radius:* doorway-local; zero substrate change.
- *Unlocks:* visibility — the A/B fork stops being invisible. **Necessary, not sufficient.** It tells you the apex is lying; it does not stop it, and it does not keep the collective serving when matthew is down.

**Rung 1 — Head freshness witness on the EprHead object (buildable NOW, additive).**
Add to `EprHead` (`epr_codec.rs:101`), additively (`#[serde(default)] Option<…>`, MessagePack/DAG-CBOR additive per the wire-evolution discipline): `predecessor: Option<String>` (CID of the prior head for this `id`), `seq: Option<u64>` (monotonic per-id), and `proof: Option<EprHeadProof>` (author Ed25519 detached signature over the canonical head bytes, reusing `epr/src/proof.rs:51` `verify`). Now a head is **self-verifying and self-ordering**: any holder can prove "mine has seq N, signed by the author" and any client can verify it.
- *Cost:* one struct + one signer at PUT (`handle_put_epr_head` http.rs:7512) + one verifier at GET; additive migration, no DNA entry (heads are Tier-3 content blobs, DAG-CBOR addressed).
- *Blast radius:* EprHead producers/consumers; additive so old heads decode (seq absent ⇒ treated as seq 0 / unproven).
- *Unlocks:* the **freshness comparator** every higher rung needs. Capture-resistance floor: a client can reject an unsigned or lower-seq head.

**Rung 2 — Head heal-on-read: the head joins the quilt (buildable NOW, composes Rung 1 + existing primitives).**
Generalize `get_blob_or_heal` into `get_head_or_heal`: on a local head miss OR a head whose `seq` is older than `freshness_grace`, race-fetch the head **from coverage-committed peers for that id's reach** (`peer_selection.rs:103` supplies the candidate set; `blob_fetch.rs:64` `race_fetch` supplies the mechanism — heads are tiny, the existing batch/timeout machinery is ideal). Accept only a head that (a) verifies its signature and (b) has `seq >= mine`. **Highest verified seq wins** — the same "first verified responder" shape, with seq as the tiebreak.
- *Cost:* a head-flavored sibling of the blob heal path; reuses race-fetch, peer-selection, and Rung 1's proof verbatim.
- *Blast radius:* storage head-resolution path; no doorway change (single-target dispatch preserved — doorway still forwards to one storage, which now heals its own head from the quilt).
- *Unlocks:* **THE vision clause** — "collectives continue to serve." When matthew falls, adam's storage heals the head from any committed sibling and serves it. A down hub becomes a re-fetch, not an outage. And it *cannot* serve a fork: an unverified or stale head fails the gate.

**Rung 3 — Fail-closed-to-"catching-up" at the apex (buildable NOW; the capture-resistance latch).**
When a peer is partitioned (cannot reach any coverage-committed sibling to confirm freshness) AND its head is older than `freshness_grace`, it returns **`503 catching-up + Retry-After`** for that id, NOT a 200 with the stale head. Doorway already honors this exact contract (`storage_proxy.rs:240` — upstream 429/503 → catching-up to the browser, Retry-After preserved). The latch: **degraded-but-honest, never available-but-lying.**
- *Cost:* a freshness gate on the head GET path + reuse of the existing 503/Retry-After plumbing.
- *Blast radius:* head GET only; the byte plane and all fresh-head reads are unaffected.
- *Unlocks:* capture-resistant stasis. A partitioned or hostile edge **structurally cannot** present a fork as canonical — the worst it can do is say "catching up."

**Rung 4 — Arc-as-coverage-commitment for the HEAD plane (ROADMAP; the operator-native pivot, parallels the ARC pass).**
Today head-coverage is implicit (whoever's conductor happens to hold the DHT entry). Make it a **`Mishpat::Commitment` with action `holds-head-coverage` for reach-scope X**, with the `∪ committed-coverage = full` invariant enforced through governance — exactly the arc-as-REA-coverage-commitment move from the ARC pass, applied to the head plane. Failover then redeems a *named, audited, revocable* commitment: "adam is the standby head-steward for the genesis corpus because adam committed to it," not "adam happened to survive."
- *Cost:* one `signal_kind` + `resource_classified_as` whitelist entry on the existing REA primitive (NOT a new entry type — the compute-commitment primitive already exists, `project_rea_compute_commitment_primitive`); a coverage-invariant checker; governance surfacing.
- *Blast radius:* REA/Mishpat layer; the head-heal path (Rung 2) consumes the commitment as its candidate filter (it already filters on `content:{reach}` commitments — this just sharpens the action).
- *Unlocks:* the full coupling — availability becomes a *negotiated, governed property* of the commons, not an infrastructure accident. **One substrate, three instantiations: arc-as-commitment ≡ compute-as-commitment ≡ head-coverage-as-commitment.**

**Rung 5 — DEEPER PIVOT (ROADMAP): two quilts, one for truth, one for bytes — applied to the head.**
The ARC pass already named the split: the DHT/head plane is the trust/value/governance plane (small, validated, wants near-full arc and strong freshness); the byte plane is the heavy-corpus RS(4,7) quilt. This pass *operationalizes* that split for availability: the head quilt replicates with **higher coverage and a freshness proof** (because a stale head is a capture risk), while the byte quilt replicates with **erasure-coded breadth** (because a missing shard is just a re-fetch). Different availability SLAs for different truth-criticality — the same insight, made into a dataplane policy.
- *Cost:* policy + config; no new mechanism beyond Rungs 1–4.
- *Unlocks:* the architecture the vision describes, fully realized: a high-integrity head plane and a high-breadth byte plane, each with availability tuned to its capture-risk.

---

## 4. Recommended ESCALATION (defended) + what it COMMITS US TO

**Recommend: ship Rungs 0–3 as one coherent "head joins the quilt" arc now; commit Rungs 4–5 to the roadmap as the governance/architecture pivot.**

Why this cut, defended:

- **Rung 0 alone (the tactical plan) is the trap.** A detector that watches the apex lie, without a mechanism to keep serving truthfully, satisfies the *diagnostic* but not the *vision*. The operator explicitly asked to escalate past the blocker to the path. The path is "the head becomes a quilt object."
- **Rungs 1–3 are buildable NOW and compose only existing primitives** — RS/race-fetch/heal (`blob_fetch.rs`, `http.rs:2293`), coverage selection (`peer_selection.rs`), signatures (`epr/src/proof.rs`), predecessor lineage (`predecessor_records.rs`), and the doorway 503/Retry-After contract (`storage_proxy.rs:240`). No new transport, no DNA entry, no fork. This is "compose the substrate we already have at the layer that's missing it," exactly the ARC-pass shape.
- **Rung 3 is the non-negotiable capture-resistance latch.** It is what makes "available" safe: the survivor serves a *proven* head or says "catching up." Without it, head-heal could itself propagate a fork. It must ship *with* Rung 2, not after.
- **Rungs 4–5 are genuine roadmap/governance commitments** — they turn an implicit infrastructure property into a negotiated, audited one, and they make the two-quilt split a deliberate policy. They depend on the REA coverage-commitment work the ARC pass also lands against, so they share a primitive and should be sequenced together.

**What it commits us to:**
1. **A new primitive (small): `EprHeadProof` + `predecessor`/`seq` on `EprHead`** — heads become signed, ordered, self-verifying content. Additive; not a DNA entry. (Rung 1.)
2. **A new substrate capability: head heal-on-read** (`get_head_or_heal`) — the head plane gets the byte plane's failover, gated on freshness + signature. (Rung 2.)
3. **A fail-closed invariant: partitioned-stale-head ⇒ 503 catching-up, never 200 fork.** This is a *protocol invariant*, worth a chaos test sibling (`chaos_head_plane.rs`) and an a2o scenario. (Rung 3.)
4. **A roadmap item: `holds-head-coverage` as a `signal_kind` on the existing REA compute-commitment primitive**, with a `∪coverage=full` governance invariant — the arc-as-commitment pattern extended to the head plane. (Rung 4.) *This is the genuine new-vocabulary commitment; everything below it is composition.*
5. **A roadmap policy: head-quilt vs byte-quilt differentiated availability SLAs** (two quilts, different freshness/coverage). (Rung 5.)

**Mark of honesty:** Rungs 0–3 are buildable-now (composition of shipped primitives). Rung 4 is a real roadmap commitment (new `signal_kind` + governance invariant + the coverage-commitment work shared with ARC). Rung 5 is a policy decision, not new mechanism. Nothing here requires forking Holochain or kitsune2 — the head plane is entirely in our layer (elohim-storage + EPR + REA), which is *why* it is the cheapest high-value availability win on the board.

---

## 5. COUPLING — story + value + governance into one capture-resistant whole

This pass is the place where the three planes the operator wants coupled actually *touch* — because the EprHead already carries all three contexts (`epr_codec.rs:108-113`: `lamad`/`shefa`/`qahal`), and making the head a proven, quilt-served object means **story + value + governance fail over together, or honestly not at all.**

- **STORY (felt):** A learner reading the genesis corpus through `elohim.host` when `matthew` is crash-looping today gets either nothing (matthew's edge) or a possibly-stale head with no warning (adam's edge). After this pass: adam's storage heals the latest *proven* head from a committed sibling and serves it — the learner never notices the fall. If even that is impossible (true partition), they see an honest "catching up, back shortly," not a silently forked reality. **The collective kept serving its humans; the trust they place in what they read survived the failure.**

- **VALUE (REA / the donut):** Failover is the **redemption of a coverage commitment**, recorded as an REA economic event (the head-heal path emits a `serve-head` event the same way `finalize_fetch_success` emits `serve-blob`, `blob_fetch.rs:218`). Standby head-stewardship is *minted value in the care economy* — adam holding the genesis corpus's head-coverage is care work that shows up in the ledger, attributable and audited. Care-class stays isolated from compute-class (the freshness signal is a compute-class placement input; the coverage commitment is a care-class stream — they ride parallel, never cross-contaminate, per `project_compute_commitments_bounded`). The donut commons is the set of heads with `∪coverage=full`: nobody owns a corpus's availability, the collective *stewards* it.

- **GOVERNANCE (Mishpat contracts):** Who is allowed to serve a corpus's head under failover is a **governance decision**, not an infra default — the `holds-head-coverage` commitment is a Mishpat-notarized contract with on-chain standing and revocation. A captured or hostile steward's coverage commitment can be *revoked through governance*, after which its heads fail the freshness gate at honest peers. The `∪coverage=full` invariant is the governance contract that "sets policy and enforces decisions" about availability. **This is capture-resistance as a governed property:** the system stays in truthful stasis because availability is negotiated and revocable, not seized.

The technical move — *make the head a signed, ordered, quilt-replicated object that heals from committed coverage and fails closed to "catching up"* — is therefore not a plumbing fix. It is the seam where **coupled story+value+governance** becomes a dataplane invariant: the collective keeps serving its humans (story), failover redeems and mints care commitments (value), and who-serves-what-when is a revocable governance contract (governance) — all latched so that the system can **never trade integrity for availability**, which is the exact shape of capture-resistance against a messy, hostile world.

---

## Appendix — concrete landing sketch (for the implementing plan, not this proposal)

- `EprHead` (`epr_codec.rs:101`): + `#[serde(default)] predecessor: Option<String>`, `seq: Option<u64>`, `proof: Option<EprHeadProof>`. Round-trip + old↔new compat test (additive wire discipline).
- `EprHeadProof`: `{ signer: Cid, signature: Vec<u8> }`; sign at `handle_put_epr_head` (`http.rs:7512`), verify via `epr::proof::verify` (`epr/src/proof.rs:51`).
- `get_head_or_heal` sibling of `get_blob_or_heal` (`http.rs:2293`): local-miss-or-stale → `peer_selection::select` (reach-scoped committed peers) → `blob_fetch::race_fetch` over head CIDs → accept highest verified `seq`.
- Freshness gate: partition + stale ⇒ `503 catching-up + Retry-After` (doorway honors at `storage_proxy.rs:240`).
- `chaos_head_plane.rs`: mirror `chaos_dataplane.rs` — kill the head-holder, prove the survivor serves the *proven* head, prove a stale-isolated survivor returns catching-up not a fork.
- a2o: `genesis/a2o/features/federation/peer-loss-failover.feature` gains a head-plane row; new `head-never-forks-under-partition.feature` for Rung 3.
- ROADMAP: `holds-head-coverage` `signal_kind` (whitelist `feedback_signal.rs SIGNAL_KINDS`) + `∪coverage=full` checker — sequence with the ARC pass's arc-as-coverage-commitment work (shared REA primitive).
