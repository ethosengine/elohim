---
title: "Private-layer blind custody — encrypted replication as the resiliency floor, with bond-decay re-settlement"
id: private-layer-blind-custody-resiliency-floor
status: Draft
class: protocol-canonical
sovereignty-frame: bounded
stewardship-frame: bounded
domain: D5 (dataplane / confidentiality plane 3.13 × replication)
sprint: unscheduled — design capture only (operator directive 2026-08-09). Mixed plan by
  convention: no doc-level requires_env; the encryption round-trip and custody legs are
  household-nodes-testable, only fleet-breadth legs would carry inline @requires:shem.
topic: [confidentiality, blind-custody, encryption, key-envelope, custody-commitment, rea-decay, qahal-consent, mishpat, blob-durability, performance-floor]
cites:
  - "elohim-seam-map-concern-routing | Routes this concern: confidentiality is seam 3.13 (encryption plane), NOT authorization 3.4 — the atlas line \"make this private is the encryption plane, not a permission flag\" is the load-bearing warrant for decoupling may-store from may-read | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "substrate-trust-contract-runbook | Source of the fills-never-moves invariant the bond-decay lifecycle inherits — re-placement must be honored before a lapsing custody commitment releases, so decay never dips coverage | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "weave-epic-arc-design | Where the KeyEnvelope entry + ShardManifest field-add were first gated and HELD (#4, Wave C); this doc re-opens that gate for the blind-custody case and keeps the held status | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md"
  - "waves-3-5-epr-compute-encryption-execution-plan | Landed Wave 5.1, the encrypt-then-erasure-code round-trip proof this floor composes on; also carries the p2p/mod.rs:1492 encryption-ordering LANDMINE that any live private path must resolve first | sha256:c727a9a443334274 | path: genesis/docs/superpowers/plans/2026-06-22-waves-3-5-epr-compute-encryption-execution-plan.md"
  - "epr-durability-replication-arc-plan | The commons-side durability arc this extends to the private layer — owns distribute_shards/shard_locations and the custody-sweep replication the blind-custody commitments would ride | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md"
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
  - genesis/research/p2panda-cross-pollination-2026-08-04.md
  - elohim/elohim-storage/src/services/private_replica.rs
  - elohim/elohim-storage/src/p2p/reach_authorization.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/services/salvage_commitment_author.rs
  - elohim/elohim-storage/src/services/advertiser_health.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/sharding.rs
  - genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature
  - genesis/manifests/habits.yaml
memory_anchors:
  - feedback_reach_head_replication_distinct_planes
  - feedback-identity-sovereignty-ontology-guard
  - feedback-justice-mishpat-not-punishment-guard
  - project_p2panda_cross_pollination
  - project_inventory_exchange_not_byte_replication
  - project_hub_optional_floor
---

# Private-layer blind custody

> **Operator directive (2026-08-09, verbatim intent):** *another way we can really drive the
> performance floor here is driving resiliency on the private layer — private
> content/sharded/encrypted gets replicated for free, because local privacy can be an afforded
> right, and if we can store each-other's backups, but not know what is in them, it's the floor
> this system should support, with a lifecycle being when someone moves from one social context
> to another, those bonds degrade and the commitments/and claims (and to who) on storage change
> over time.*

**Design capture only.** No implementation is proposed for any sprint by this document. Its job is
to place the concern at its seam, clear the P2P design gate before anyone reaches for a table or a
route, and name the questions only the operator/council can answer.

**Source of truth (declared up front, per §2):** every entity below is either DHT-notarized (the
Holochain DHT is authoritative; SQLite is a read projection), private-source-chain (the agent's
chain is authoritative), or operational (recomputed-on-read, SQLite-only, reconstructable from the
graph). No new table is proposed here; nothing in this document introduces storage whose
authoritative home is unstated.

---

## 1. The four-plane ontology

Three orthogonal planes are already canon (`feedback_reach_head_replication_distinct_planes`):
**reach** (audience — earned at authoring), **content_head** (version — declared), **replication**
(availability — custody). This directive adds the fourth:

> **Custody-readability**: *may-store* is decoupled from *may-read*.

Today they are fused. `p2p/reach_authorization.rs` gates **byte flow** by reach standing: a peer
without embodied responsibility in a scope neither subscribes, advertises, nor serves. Correct for
*plaintext*; exactly wrong for *ciphertext* — it means a household can only help hold what it is
entitled to read, capping the resiliency floor at the size of each person's trust circle.

Blind custody inverts that: **authorized non-readable flow**. A custodian is authorized to hold
shards it can never open. Confidentiality is enforced by the encryption plane (seam 3.13), *not* the
authorization plane (3.4) — the atlas already says so ("make this private" is the encryption plane,
not a permission flag). Reach keeps its meaning for readability; a new, much wider gate governs
custody. Because ciphertext carries no reach semantics, **private replication is reach-invariant on
the hot path** — that is why it is "free", and why it drives the performance floor (§5).

---

## 2. P2P Design Gate output

### Entity: BlindCustodyCommitment
- **Classification**: **Notarized (A) — zero new entry types.** Reuses the existing REA
  `Commitment` entry and the live `custody-blob` action (44 call sites), authored through
  `conductor_writes::call_create_rea_commitment` exactly as `salvage_commitment_author.rs` does.
  The only delta is `resource_classified_as: "content:private-encrypted"` plus a `readable: false`
  discriminator, so `peer_selection.rs`'s contract-bounds filter admits custodians who have *no*
  reach standing in the content's scope.
- **Head-plane cost budget**: this is the sharp risk. Per-(blob × custodian) commitments would put
  a household's whole photo library on the head plane — thousands of A-class heads each paying an
  uncancellable conductor round-trip per sweep tick (measured anchor: ~3,469 heads ≈ 2.5h quiesce).
  **Bundling shape = composite root**: one commitment per *(quilt manifest × custodian)*, never per
  shard and never per file; a household's backup set is one manifest tree. Above that, the
  **capacity pledge** (§3) is the coarser root — a custodian pledges bytes to a household, not to
  each artifact. Seed: O(households × custodians) ≈ tens. 1yr: low hundreds. Any design that
  reaches per-blob heads is out of budget and must return here.
- **Network stakes**: must behave under all four stages. Custody *placement* cost is
  stage-priceable (Simulacra may skip full-chain re-verification of an already-witnessed manifest).
  **Confidentiality is floor-protected and never cheapens** — a dev-stage custodian must be as
  unable to read plaintext as a production one. A `DEV_MODE` plaintext shortcut would make every
  fixture a leak rehearsal.
- **Address**: agent-scoped composite — `(custodian agent_cid, manifest CID, "custody-blob")`.
  Two custodians of the same manifest are two commitments; that is the point.
- **Source of truth**: Holochain DHT; `rea_commitments` is the projection (`dht_anchor_hash` set on
  witness, amber until the sweep reaches it).
- **Coordinator / signal / route (in that order)**: `content_store::create_rea_commitment` →
  post-commit `ProjectionSignal::ReaCommitmentCommitted` → `rea_projection` → **no new POST**;
  reads extend the existing resilience/weave surfaces.

### Entity: EncryptedShardManifest
- **Classification**: **Derived (A2)** — a field-add on the already-notarized `ShardManifest`
  (`sharding.rs`): `encryption`, `plaintext_cid`, `key_envelope_ref`. No standalone identity, no new
  entry type. This is the field-add already scoped-and-held by the weave epic.
- **Address**: content-derived CID (`bafyrei…`, dag-cbor). Note the discipline: `blob_hash`'s bare
  `sha256-…` is legacy wire, not an address — new fields are CID-shaped.
- **Anti-pattern check**: encryption metadata is authored **once through the conductor**, never
  stamped per-host. A per-storage local write of `encryption: true` would mint a divergent
  un-witnessed head on each backend — the exact amber/green class.

### Entity: KeyEnvelope (per-reader sealed DEK)
- **Classification**: **Agent-Scoped + Attestation (B2)** — recommended, *not settled*. The sealed
  DEK is a secret and must not be a per-(reader × manifest) DHT entry, which burns headroom and the
  head plane at once. Shape: the sealed envelope is a private source-chain entry of the author,
  delivered to the reader over the authorized reach path; a notarized attestation ("an envelope
  exists for reader R on manifest M", reusing the imagodei `Attestation` subtype string) gives
  discoverability *without* revealing the key. Confidentiality-backlog #3 reached the same "likely
  B2" and explicitly did **not** pre-clear it — this stays a gate answer, not a decision. A
  DHT-resident envelope would be 1 new entry type against Lamad's ~73/~100 and needs its own pass.
- **Address**: content-derived CID over the sealed bytes.
- **Blocker (named, unbuilt)**: agent keys are ed25519; every sealed-DEK path needs X25519.
  Conversion-vs-dual-key-binding is undecided (backlog #5) and gates every production leg here.

### Entity: BondDecayState / custody re-settlement pressure
- **Classification**: **Operational (C)** — load-bearing. Decay is **recomputed on read** from
  notarized inputs (relationship/membership edges, commitment timestamps, `advertiser_health`
  liveness), never stored as a trust score. A persisted "decay" number is a per-host authored
  judgement about another human — the shape the protocol refuses. Reconstruction: recompute from the
  graph; losing the cache loses nothing.
- **Re-settlement events** (custodian released, new one adopted) are **A, reusing the existing
  `EconomicEvent`** with an action discriminator, same as `placement-gap`. Zero new types.

### Entity total: **0 new DHT entry types** in the recommended shape.

### Identity
`agent_cid` (`uhCAk…`) is canonical for author, reader, and custodian. libp2p `12D3Koo…` and iroh
`NodeId` resolve **to** it via `AgentPeerBinding → peer_identity_bindings → peer_transport_manifest`
— never raw string-compared (the all-zeros resilience-card class). Framing: custody standing is
**community-backstopped imago-dei** — a household earns custodial standing through witnessed
relationship and honored commitments. There is no "self-sovereign" apex tier here; holding your own
keys is a mechanical fact about key location, not an ascent. Cryptography **accelerates** recovery
of a lost DEK; it must never be the gate (see §7 Q2).

### Concern-canon answers (Step 4) — for the one genuinely new predicate
`classify_custody_authorization(scope, custodian) -> {MayStoreOnly | MayStoreAndRead | Refused}`,
sibling of `classify_pre_authorization`:

| id | class | answer |
|---|---|---|
| C0 | plane location | **answered** — confidentiality (3.13) enforces secrecy; authorization (3.4) enforces may-act. The predicate lives beside `reach_authorization.rs` but returns a *custody* verdict, not a reach one. |
| C4 | honest absence | **unbound** — "I hold shards I cannot read" must be distinguishable from "I hold nothing"; today inventory gossip cannot say it. |
| C5 | evidence-not-authority | **answered** — verdict derives from notarized commitments + graph edges, never from a peer's self-claim. |
| C12 | consent/authorization | **partial** — custodian consent is the capacity pledge (exists in shape); *reader-set* consent has no surface. |
| C13 | graduated authority | **partial** — may-store/may-read is the first graduation; wards/guardians unmodeled. |
| C14 | witnessed residual | **unbound** — what a released custodian may retain (and for how long) is undefined; see §7 Q3. |
| C1/C2/C3/C6a/C6b/C7/C8/C9/C10/C11 | — | answered at registration time; the predicate is born-registered in `elohim-storage`'s `seam-registry.yaml` when code is written, not before. |

**Fail-closed note carried forward:** the existing classifier *fails open* on DB-pool errors
(confidentiality backlog #1). A custody predicate that inherits that default would hand shards to
anyone during a pool blip. Fail-closed is a precondition of this design, and it is already a
standalone bounded item.

---

## 3. Lifecycle — bond decay as REA commitment re-settlement

The directive's second half is the harder half. People move between social contexts; the storage
commitments and *whose* claims they answer must move with them.

**States.** `rea_commitments` is effectively `active | cancelled` today. The lifecycle needs a
middle: `active → lapsing → re-settled → released`, where **release cannot complete before
re-placement has** — mirroring the live dataplane invariant that healing *fills, never moves*, so
decay never dips coverage.

**Decay signal.** Recomputed, never stored: relationship-edge staleness × membership change ×
honored-commitment history × `advertiser_health` liveness. Decay first lowers a custodian's
*priority* in `peer_selection.rs`'s diversity-greedy ranking, long before touching an existing
commitment — a fading bond stops attracting NEW content, which is gentle and reversible.

**Consent (qahal).** Three surfaces, none automatic: the departing custodian is never conscripted
into continuing nor punished for leaving; the adopting custodian's pledge is opt-in (the
Good-Samaritan salvage door has this shape already); the author's reader-set is the consent surface
for *readability*, and it moves independently of custody — leaving a community may change who can
read without changing who holds.

**Consequence framing (Mishpat).** A lapsed custodian is not a defaulter; release is not revocation.
The frame is **renegotiated boundary and restored capability**: terms change, the network re-places,
and the departing custodian's honored history is *recognized* (an `appreciation`-shaped event), not
zeroed. A design rendering decay as a falling trust score with penalties has imported the punishment
ontology and must be sent back.

---

## 4. Composition inventory

**Already exists — compose, do not reinvent:**

| Piece | Where | State |
|---|---|---|
| encrypt → RS-shard → drop shard → reconstruct → decrypt round-trip | `services/private_replica.rs` | proven in tests, incl. `reader_with_envelope_recovers_custodian_cannot` |
| DEK sealing/unsealing to a reader key | `private_replica.rs` (`seal_dek`/`unseal_dek`, dryoc) | proven; substrate for real reader keys missing |
| RS-quilt sharding + manifests | `sharding.rs`, `shard_service.rs`, `p2p/shard_protocol.rs` | shipping |
| custody commitments + reconcile-fetch | `reconcile/custody.rs`, `rea_commitment_service.rs` | shipping |
| self-selecting salvage that authors notarized intent | `services/salvage_commitment_author.rs` | shipping |
| household-diversity placement, contract-gated | `services/peer_selection.rs` | shipping (dormant in prod on NULL household_id) |
| custodian liveness scoring | `services/advertiser_health.rs` | shipping |
| felt-resilience projection ("held by 3 households") | `grandma-photos-survive-node-loss.feature` | green scenarios |

**Genuinely new (and only this):** (a) the may-store/may-read split in the authorization vocabulary;
(b) the reader-key substrate (ed25519→X25519) that turns the proven envelope math into a live path;
(c) the capacity pledge as a commitment root; (d) the decay→re-settlement state machine.
`p2panda-encryption` remains an **audit-gated candidate** for (b)'s group-key layer — the announced
Feb-2025 audit was never confirmed published, and adopting it is an operator decision, not an
engineering one.

---

## 5. The performance-floor argument

Today the dataplane moves bytes only when someone publishes commons content — bursty, rare, mostly
deploy-triggered. That is why convergence defects surface as post-deploy spikes and limit cycles
instead of steady-state signal: **there is no continuous load**.

Blind custody supplies one. Every household with private data and spare disk becomes a continuous
producer *and* custodian of ciphertext, exercising exactly the machinery whose weaknesses currently
hide — fan-out pacing, RS reconstruct, salvage self-selection, diversity placement,
`advertiser_health` scoring, inventory→byte-replication (the gap
`project_inventory_exchange_not_byte_replication` names) — and exercising it **reach-invariantly**
(authorization once at commitment time, not per byte), so the hot path stays cheap.

It is "free" politically too, which is the directive's actual point: nobody has to be persuaded to
donate capacity for a stranger's public content. They trade **symmetric blind custody** — I hold
yours, you hold mine, neither can read the other's. Local privacy as an afforded right is what makes
the exchange acceptable; the resiliency floor is the byproduct.

**The counterweight, stated honestly:** constant floor traffic is constant cost. Unbounded pledges
would make the floor a self-inflicted load test on the very devices (phones, 2019 hardware) the
backpressure exists for. The pledge must be a household-set ceiling, and the floor must degrade to
zero on a withdrawing device — the hub-optional floor holds: no household must custody anything to
be a member.

---

## 6. a2o scenario sketches (in-doc — no `.feature` file this pass)

Born-red targets for whenever this is scheduled. Tags follow the resilience family:
`@e2e @resilience @felt @concern:blob-durability @dataplane`.

```gherkin
Scenario: My private backup survives peer loss, and no custodian can read it
  Given a household has published a private content item "family-2019" with reach "intimate"
  And "family-2019" is encrypted before erasure-coding and placed with 3 custodian households
  And none of the 3 custodians has reach standing in the "intimate" scope of the author
  When one custodian household goes offline permanently
  Then "family-2019" is reconstructable by the author from the surviving custodians
  And the reconstructed plaintext CID equals the original plaintext CID
  And no custodian's local store or HTTP surface yields plaintext for "family-2019"
  And each custodian's resilience view names it as held-but-unreadable, not as absent
```

```gherkin
Scenario: A bond fades and custody re-settles without a gap or a penalty
  Given a custodian household holds shards of "family-2019" under an active custody commitment
  When that household's relationship bond with the author lapses past the decay threshold
  Then the household stops being selected for NEW placements of the author's content first
  And a replacement custodian is placed and honored BEFORE the lapsing commitment is released
  And the coverage count for "family-2019" never drops below the household's declared floor
  And the departing household's honored custody history is recognized, not zeroed
  And no view describes the departing household as failing, defaulting, or untrusted
```

---

## 7. Open questions (operator / council)

1. **Key management ontology.** Where does a household's reader key live, and what is its
   relationship to `agent_cid`? ed25519→X25519 conversion vs a dual-key binding is the concrete
   fork, and it gates every production leg (confidentiality backlog #5).
2. **Escrow vs social recovery.** If the reader key is lost, is the DEK recoverable? The protocol's
   stance says cryptography accelerates community recovery and never gates it — which argues for a
   social recovery quorum over any escrow agent. But a quorum that can reconstruct a DEK is, by
   construction, a quorum that can read your private data. **This is the sharpest unresolved
   tension in the design** and it is a council question, not an engineering one.
3. **Decay curve governance.** Who sets the decay function — the author, the custodian, the
   collective, or the constitution? And what is the **witnessed residual**: when a custodian is
   released, must they delete the ciphertext, and how would anyone know? (An unenforceable deletion
   promise stated as a guarantee is worse than an honest "they still hold it, unreadable.")
4. **Adversarial custody.** Is blind custody a vector — can a hostile peer volunteer to hold
   everything and gain traffic-analysis signal even without plaintext? (Sedimentree's
   ciphertext-only relay seam and p2panda's PSI discovery are the named prior art.)
5. **`p2panda-encryption` adoption.** Consume the crate or build from primitives? Gated on the
   unconfirmed audit; a one-hour check gates a large fork in effort.
6. **The witnessed-harm limit (operator note, 2026-08-09).** Privacy carries the highest
   protection here, but it is a *qualified* right (ICCPR Art. 17 bars only arbitrary
   interference), and the CSAM / violent-crime edge is where it meets its limit. Nobody has
   resolved this tension; the best available practice *manages* it, and the design should adopt
   that shape rather than pretend a resolution exists:
   - **Duty attaches on knowledge, not a duty to scan** (the 18 U.S.C. §2258A / NCMEC
     CyberTipline shape). Blind custody concentrates this correctly: custodians *cannot* know,
     so the duty lives at the authoring edge and in explicit roles — never via unsealing custody.
   - **Content-ID without sight**: perceptual hash-matching against curated lists (PhotoDNA,
     NCMEC/IWF lists, Project Arachnid, Thorn Safer) at the edge where content is plaintext —
     the StopNCII shape (victim hashes locally; only hashes travel) is the closest prior art
     for this architecture.
   - **Accountable witnessed reporting over mass inspection**: message-franking-style schemes
     (a party with legitimate sight proves what they saw without platform scanning) — the
     El Roi witnessed-sight frame, not a panopticon. The unresolved fight is mapped by Apple's
     withdrawn 2021 NeuralHash client-side scanning, the "Bugs in Our Pockets" rebuttal
     (Abelson/Rivest/Schneier et al.), the Levy/Robinson counterpoint, and the deadlocked EU
     CSA Regulation — that corpus ends in disagreement and IS the state of the art.
   - **Institutional scaffolding** (no single UN operational standard exists): CRC + OPSC,
     CRC General Comment No. 25 (privacy-vs-protection balancing for children), the 2024 UN
     Cybercrime Convention CSAM provisions, WeProtect Global Alliance (Model National
     Response), INHOPE hotline network.
   - **Agent memory as the frontier gap**: no standard exists. Nearest analog is human
     moderator practice — exposure minimization (classifier-first, need-to-know, rotation),
     evidence held only under legal-hold then purged, because possession is itself the harm.
     Translation for agents: classifiers/hashes see content so agents see *verdicts*; when an
     agent must see, that context is quarantined (no memory formation), a report artifact is
     produced, and disposal is a governed ceremony, not retention. What an agent has witnessed
     is not healthy residue to carry — same as for a person.
   - **Mishpat framing**: trust-and-safety sight is a *bounded stewardship role* with the
     highest commitment requirements on the network (content-ID/non-proliferation + T&S, strict
     need-to-know, witnessed and accountable), never a standing capability of ordinary peers or
     agents. Same decision family as open question 2 (who may ever see, under what commitment,
     with what disposal) — council question, not an engineering one.

## 8. Non-goals

- **No implementation this sprint.** Nothing here is scheduled; nothing here modifies a habit.
- **`p2panda-encryption` stays audit-gated** — surveyed and cited, not adopted.
- **No new DHT entry type is proposed.** If `KeyEnvelope` proves to need one, it re-enters the gate.
- **No new register, ledger, or ranking script.** The measure is the existing `@concern:blob-durability`
  scenario set plus the reach-enforcement reds.
- **Doorway does not become a custody dependency.** A ciphertext-only relay is a *seam question*
  (§7 Q4), not a refactor, and the zero-doorway mesh must retain the full floor.

## 9. Habit linkage

Serves **`blob-durability`** (green) as its named expansion path: today the habit proves *commons*
bytes survive peer loss; blind custody extends the same invariant to the private layer, where the
capacity actually is. It is also the **enforcement counterpart that makes `reach-enforced-everywhere`
(unwired) coherent** — that habit's invariant says "enforcement, not exclusion", and blind custody is
what the distinction *means* in the dataplane: authorized flow that is deliberately non-readable. As
written today, a literal reading of "scoped tiers flow only to authorized receivers" would forbid the
very traffic this floor depends on. **No habit is added or modified by this document.**

## 10. Missing nodes (story-graph maintainer)

- **chain** `blob-durability` / **between** "content is placed with N diverse households" → "content
  survives peer loss" / **missing node** "a custodian holds bytes it cannot read" (assertion: no
  custodian surface yields plaintext for a private blob; probe: custodian-side plaintext-scan step)
  / **state** unbuilt — proven in-process in `private_replica.rs`, no fleet-level probe exists.
- **chain** `reach-enforced-everywhere` / **between** "the reach classifier authorizes egress" →
  "scoped tiers reach only authorized receivers" / **missing node** "authorized non-readable flow"
  (assertion: a peer unauthorized to READ is authorized to STORE) / **state** unnamed — the reach
  vocabulary has no may-store/may-read split.
- **chain** custody lifecycle / **between** "commitment active" → "commitment cancelled" /
  **missing node** "lapsing, with re-placement honored before release" (assertion: coverage never
  dips during re-settlement) / **state** absent — `rea_commitments` state transitions are
  active/cancelled only.
- **chain** confidentiality / **between** "reader holds a sealed DEK" → "reader decrypts" /
  **missing node** "reader-key substrate resolves an X25519 key from `agent_cid`" / **state**
  named-but-unbuilt (confidentiality backlog #5); blocks every production leg above.
- **chain** custody release / **between** "custodian released" → "content re-placed" / **missing
  node** "witnessed residual — what the released custodian retains" / **state** undefined; §7 Q3.
