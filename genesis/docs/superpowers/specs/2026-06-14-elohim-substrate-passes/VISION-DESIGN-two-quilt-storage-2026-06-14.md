# Vision Design Pass — The Two-Quilt Storage Architecture

**DHT trust-plane ⊕ RS(4,7) byte-plane quilt**

Date: 2026-06-14 · Author: Rust Architect (truth layer) · Status: **PROPOSAL for operator blessing** (working draft, not cite-sealed, not code)

> This pass escalates D1-option-(iii) of `P2P-DATAPLANE-REVIEW-2026-06-14.md` ("move corpus off the conductor DHT plane into the RS quilt byte-plane") from a *deferred design spike* into the **architectural backbone the arc pass implies**. The arc worked-example concluded: split into two quilts. This pass formalizes the split, names what lives where, and shows how grandma's photos load fast and provably-held emerges from it — coupled to story + value + governance.

---

## 1. What the VISION REQUIRES here

The north star, decomposed against this layer:

- **"a quilt-tier's replicated dataplane"** — the heavy corpus (photos, blobs, video, large content bodies) must live on a *replicated, erasure-coded, CID-addressed* plane that survives flood-in-one-city / outage-in-another / court-order-in-a-third (`tiered-quilt-stewardship-design.md:50-55`). This is the byte-plane.
- **"maintains that high-integrity of the Holochain DHT, that allows people to build trust on the values that are negotiated through it"** — identity, notarization, commitments, observations, care-events, and the *pointers* to bytes must stay on the validated DHT. This is the trust-plane. Crucially: the DHT's integrity is **what people build trust on** — so it must stay lean enough that every node can hold a real arc of it. A DHT bloated with corpus bytes is a DHT only datacenters can anchor → capture.
- **"those mutual compute agreements"** — *who holds which shard* is not a config fact, it is a **negotiated, revocable REA commitment** (`custody-blob` → the `rea-compute-commitment-primitive`). Custody-as-bytes is queried against the ledger, not stored in a second registry.
- **"collectives continue to serve the humans that use it"** — a collective (household → factory) is a set of stewards whose ∪ custody-commitments cover the corpus their humans need. Serving is fulfilling commitments.
- **"hubs — households to factories — that scale the sensemaking"** — hubs are byte-plane capacity concentrators (high-pantry nodes) AND custody-commitment aggregators, but they are **convenience, never a gate** (`project_hub_optional_floor`). A laptop alone must still load grandma's photos.
- **"governance contracts that set policies, enforce decisions"** — the coverage invariant (∪ custody ⊇ corpus, ≥ r_floor holders per shard) is **enforced through governance**, exactly as arc-as-coverage-commitment governs the keyspace.
- **"donut-like commons — the trust-economy, the care-based economy stories where value is minted"** — every tier transition and custody fulfillment emits an REA EconomicEvent that flows into shefa's reciprocity math. Holding grandma's photos *for her family* is minted care; serving them to the commons is minted contribution.
- **"stay in stasis when actuating a capture-resistant state"** — the split is the capture-resistance: bytes cannot be subpoenaed from one jurisdiction (RS-distributed across r_floor stewards in r_floor jurisdictions), and trust cannot be forged (DHT-notarized). Neither plane alone is capturable.

**The one-sentence requirement:** *The protocol must let a content HEAD on the high-integrity DHT point at erasure-coded bytes on a replicated quilt, where holding those bytes is a governed, revocable, value-minting REA commitment — so that the felt experience (photos load fast, provably held) and the economic experience (care is minted) and the governance experience (coverage is enforced, capture is resisted) are the same act.*

---

## 2. Is the substrate CAPABLE? Dig to WHY — exact layer

**Verdict: the substrate is ~80% capable. The split EXISTS at the data-structure layer. What is missing is the POLICY/RECONCILIATION layer that makes the split self-tending and value-coupled — and that layer is buildable-now, not a fork.** This mirrors the arc finding exactly: the continuous capability is already in the substrate; what's missing is a *policy* over it.

### What the substrate already speaks (the split is real, not aspirational)

The two quilts are **already two distinct entry types on the DHT and two distinct byte paths** — the architecture was always split (`dht-is-a-notary-not-a-byte-store.md:38-39`). The drift was only in *implementations* reaching for the wrong layer.

**Trust-plane (already notarized, lean by design):**
- `Content` entry — `content_store_integrity/src/lib.rs:490`. **Critically, it already carries the HEAD→bytes pointer**: `blob_cid: Option<String>` (`:521`), `content_size_bytes: Option<u64>` (`:524`), `content_hash: Option<String>` (`:527`). The doc-comment at `:478-487` literally states the design: *"DHT stores proofs (manifests), not data (content bodies)... if blob_cid exists, fetch from elohim-storage."* The HEAD-points-to-bytes mechanism **exists**.
- `ShardManifest` entry — `content_store_integrity/src/lib.rs:631`. Carries `blob_hash`, `encoding` (`none`/`chunked`/`rs-4-7`/`rs-8-12` — `:612-617`), `data_shards`, `total_shards`, `shard_hashes: Vec<String>` (`:655`). This is the byte-plane's *recipe*, notarized on the trust-plane. **It exists.**
- `ShardLocation` entry — `content_store_integrity/src/lib.rs:683`. Maps `shard_hash` → `holder` (pubkey) + `holder_did` + `storage_tier` (`hot`/`warm`/`cold`). The "who holds which shard" *fact* has a DHT home.
- DNA entry budget: **71 entry types in `content_store_integrity/src/lib.rs`** (`rg -c hdk_entry_helper`), ~75 total across the zome's sub-modules, of ~100. **The trust-plane types we need already exist — zero new entry types required for the core split.** (Confirms `dna/CLAUDE.md`: content_store at 75.)

**Byte-plane (already erasure-coded, already CID-addressed):**
- `sharding.rs` — the full RS(4,7) encoder/reconstructor. `ShardEncoder::create_manifest` (`:141`), three-band sizing `determine_encoding` (`:125`: `none ≤16MB`, `chunked ≤64MB`, `rs-4-7 >64MB`), `reconstruct` recovers from any 4-of-7 (`:301`, tests `:524`-`:582` prove drop-3-shards reconstruction). **The byte math is done and tested.**
- `blob_store::BlobStore::compute_addresses` produces a CIDv1 (`bafkrei...`, raw codec — `sharding.rs:435`) — content-addressed identity, not UUID. ✔ P2P-native.
- Two transport stacks already move shards: libp2p custom shard protocol + iroh-blobs. (Per architect prompt + `P2P-DATAPLANE-REVIEW` F6.)

**Custody-as-commitment (already an REA action, already reconciled):**
- `reconcile/custody.rs` — the custody reconciliation controller. `reconcile_pass` (`:114`) walks every `custody-blob` commitment in `rea_commitments` (`:128-131`), and for each: *if I'm the provider and the blob is missing locally, kick a fetch* (`:149-174`); *if I'm the receiver and the provider hasn't honored it, emit a `placement-gap` REA EconomicEvent* (`:175-251`). **Custody-as-REA-commitment with eager reconciliation is BUILT** — this is `project_principle_p1_reconciliation_controller` (DHT=manifest, libp2p=reality, controller=diff) instantiated for bytes.

### So WHERE is the limit? Three precise gaps (all policy, none structural)

**GAP-1 — The HEAD→bytes pointer is populated by HAND, not by a content-ingest policy.** `Content.blob_cid` exists but there is no enforced ingest path that says "content body > threshold ⇒ stock into the byte-plane quilt, set `blob_cid`, leave `content` empty." Today legacy entries still inline the full body in `content` (`:482`, `:496`). **The limit:** no *re-homing policy* that decides "this belongs on the byte-plane" and mints the corresponding `ShardManifest` + `custody-blob` commitments at ingest. This is the same shape as the arc gap — the data structure is continuous (`blob_cid: Option`), the policy that *uses* it is `{inline, by-hand}`.

**GAP-2 — `ShardLocation` (DHT) and `peer_blob_inventory` (operational projection) are not unified by the custody controller as the coverage authority.** `reconcile/custody.rs` reconciles `custody-blob` commitments against `peer_blob_inventory` (libp2p-gossiped reality), but **there is no coverage-invariant gate** — nothing computes "∪ active custody-commitments ⊇ corpus, each shard ≥ r_floor holders" and *refuses/elevates* when a custody change would open a gap. Contrast `arc_actuator::coverage_admits` (`arc_actuator.rs:152`), which DOES exactly this for the keyspace arc. **The limit:** the byte-plane has no `coverage_admits` analog. Custody can be dropped into a gap silently — the exact failure `project_inventory_exchange_not_byte_replication` warns about (6 peers agree inventory=3430, only genesis holds bytes).

**GAP-3 — The conductor DHT plane still carries corpus working-set, because the re-homing of GAP-1 was never enforced.** This is the *root* the arc pass hit: `project_per_node_memory_is_conductor_authority_arc` — per-node RAM ∝ corpus because the embedded conductor holds a full-authority arc *of content that should be byte-plane bytes, not DHT entries*. The `arc_policy::derive` (`arc_policy.rs:138`) already computes the right fractional aim, but `arc_actuator` (`arc_actuator.rs:33-35`) can only express `{0,1}` because **kitsune2 0.3.2/0.4.1 has no fractional lever** (`genesis/docs/content/elohim-protocol/history/2026-06-14-arc-factor-feasibility-spike-findings.md`, review F1 — CONFIRMED, REJECT fork). **The limit, dug to its floor:** the conductor's RAM problem is NOT solved by shrinking its arc (no lever, and a leecher breaks "laptop = full participant"). It is solved by **making the corpus not-be-DHT-entries in the first place** — i.e., enforcing GAP-1's re-homing so the conductor's arc is lean *identity/provenance/anchor* entries (which a laptop CAN hold full), and the heavy bytes live on the byte-plane quilt (RAM-independent of arc).

**The synthesis:** all three gaps are the SAME missing thing — **a Two-Quilt Reconciliation Policy that (a) routes content bytes to the correct plane at ingest, (b) enforces the byte-plane coverage invariant through governance, and (c) thereby keeps the trust-plane lean enough for full-arc laptop participation.** No fork. No new DHT entry type. The substrate already speaks both quilts; we are writing the policy that governs the seam — exactly the arc lesson ("write our policy, not fork Holochain").

---

## 3. The PATH / PIVOT / FORK LADDER (cheapest → deepest)

### Rung 0 — Diagnostic gate (must precede investment; review D1 demands it)
**Cost:** operator-side, ~1 hr. **Blast radius:** none.
Confirm leak-vs-bounded-large: `ps -o rss,comm` on a heavy node + `target_arc_factor:0` ablation on one node (`project_per_node_memory_is_conductor_authority_arc` operator actions). **Unlocks:** the certainty that GAP-3 is a corpus-on-wrong-plane problem (re-homing fixes it) and not a leak (which re-homing would NOT fix). This is the same "diagnose before fixing" discipline as review D7.

### Rung 1 — Content-ingest plane-routing policy (GAP-1) — **buildable NOW, no fork, no entry type**
**Cost:** M (one new service `services/content_plane_router.rs` + ingest-path wiring). **Blast radius:** medium — touches the content create/update path; additive (`blob_cid` already optional). **Unlocks:** new content automatically stocks bytes > threshold into the byte-plane, sets `Content.blob_cid` + `content_hash` + `content_size_bytes`, mints a `ShardManifest`, leaves `content` empty. The HEAD→bytes link becomes *automatic and correct*, not by-hand. Reuses `sharding.rs` wholesale; reuses the existing `Content` + `ShardManifest` entry types. **Care must be taken with `project_local_stack_dht_anchor_gap`**: every byte-plane row must carry `dht_anchor_hash` from the `ShardManifest`, or `require_provenance` 404s every read.

### Rung 2 — Byte-plane coverage-invariant gate (GAP-2) — **buildable NOW, no fork**
**Cost:** M (a `coverage_admits`-analog for custody, in `reconcile/custody.rs` or a sibling `services/quilt_coverage.rs`). **Blast radius:** small (pure decision fn + an elevate sink, mirrors `arc_actuator::coverage_admits` verbatim). **Unlocks:** the governance enforcement clause. Before any `custody-blob` commitment is dropped/expired, compute `remaining_holders(shard) ≥ r_floor`; if not, **REFUSE and ELEVATE** ("add a steward / keep holding — do NOT open a byte-gap"), exactly as the arc coverage gate refuses a leecher that breaks keyspace coverage (`arc_actuator.rs:163-171`). This is what makes "governance contracts enforce decisions" *true for bytes*. Promote `ActuationRefusal`/`RefusalCode` into the shared `elohim-compute` crate (review D3) so arc-coverage and quilt-coverage consume ONE definition.

### Rung 3 — Custody-as-coverage-commitment, governed (GAP-2 + value coupling) — **buildable NOW, the operator-native move**
**Cost:** M-L (extend `custody-blob` commitment bounds to carry a *coverage pledge*; emit fulfillment EconomicEvents into shefa). **Blast radius:** medium (REA ledger + shefa projection). **Unlocks:** the donut. A steward commits (`Mishpat::Commitment`, `custody-blob` action) to hold shard-set X for collective Y with bounds {tier, duration, reach}. The ∪ of active custody-commitments IS the coverage guarantee; revocation is real (drop the commitment → reconcile detects the gap → elevate). Every honored custody-fulfillment emits a care-class EconomicEvent → shefa mints value. **This is arc-as-coverage-commitment's twin: custody-as-coverage-commitment.** One substrate, now THREE instantiations proven: arc-as-commitment ≡ compute-as-commitment ≡ **custody-as-commitment**.

### Rung 4 — DEEPER PIVOT: re-home corpus off the conductor DHT plane (GAP-3 resolution) — **design spike, large blast radius**
**Cost:** L-XL (design spike per review D1; NOT a sprint). **Blast radius:** large — re-homes content truth. **Unlocks:** the conductor holds only a lean identity/provenance/anchor arc (full-arc-able on a laptop — keeps `project_hub_optional_floor` true), while heavy corpus bytes live RAM-independent on the byte-plane. **This is the durable answer to the arc RAM problem that the rejected fractional-arc fork could never give** (a leecher contributes nothing; a lean-trust-plane laptop contributes its full identity arc AND its pantry bytes). **Gated on Rung 0's discriminator** and on Rungs 1-3 being live (you cannot re-home what the ingest policy hasn't already been routing).

### Rung 5 — Upstream / fork candidates (only if a deeper limit surfaces)
**Cost:** XL. **Blast radius:** ecosystem. **Only if** Rung 4 reveals the trust-plane *still* can't go lean enough on a laptop — then the fork candidate is **kitsune2 fractional sharding** (the arc pass's rung-c, on-mission upstream). But note: the two-quilt split is *designed to make this fork unnecessary* — by moving bytes off the DHT, a `{0,1}` arc on a lean trust-plane is fine. **The split is the alternative to the fork.** This is the headline pivot: we don't fork Holochain's arc; we make the arc not need to be fractional by making the DHT lean.

---

## 4. Recommended ESCALATION (defended) + what it COMMITS US TO

**Recommendation: formally adopt the Two-Quilt architecture as the storage backbone, and build Rungs 1→2→3 now (buildable, no fork, no new entry type), gating Rung 4 (the corpus re-homing spike) on Rung 0's diagnostic. Explicitly REJECT the kitsune2 fractional-arc fork (Rung 5) — the two-quilt split is its replacement.**

Why this and not "accept the conductor-RAM ceiling" (the tactical floor I'm escalating from): the tactical layer (review D1) correctly *named* option (iii) but *deferred it as a spike* and left option (i) `{0,1}` arc as the near-term answer. But `{0,1}` arc **cannot satisfy the vision** — a leecher contributes nothing, breaking "collectives serve humans" and "laptop = full participant." The escalation is: **the byte-plane split is not a deferred spike, it is the backbone — and most of it (Rungs 1-3) is buildable now without the spike.** The spike (Rung 4) is only the final re-homing of legacy inline corpus; the *forward* path (new content routed correctly) lands immediately.

**What it COMMITS US TO:**
1. **A roadmap commitment** — Two-Quilt is named the canonical storage architecture; the tiered-quilt spec (`2026-05-11-tiered-quilt-stewardship-design.md`) becomes its byte-plane temperature-gradient detail, and `dht-is-a-notary-not-a-byte-store.md` its trust-plane invariant.
2. **A new primitive instance, not a new primitive** — `custody-blob` as a *coverage commitment* with a coverage-invariant gate. This is the THIRD proof of the `rea-compute-commitment-primitive` (after arc-as-commitment and compute-as-commitment). **No new DHT entry type; no fork.** A `signal_kind`/bounds extension at most.
3. **A shared-crate commitment** — `ActuationRefusal`/`RefusalCode`/coverage-gate logic moves to `elohim-compute` so arc-coverage and quilt-coverage are one definition (review D3 alignment).
4. **A NON-commitment, stated to kill a recurring question** — we do **NOT** commit to forking holochain_p2p/kitsune2 for fractional arc. The two-quilt split makes a lean-trust-plane `{0,1}` arc sufficient.

---

## 5. COUPLING — story + value + governance as one act

The two-quilt split is where the technical *becomes* the felt + economic + governed whole:

**Story (the felt spine — grandma's photos):** Grandma uploads a photo album. The ingest router (Rung 1) sees bytes > threshold, RS(4,7)-encodes them into the byte-plane quilt, and writes a tiny `Content` HEAD on the DHT with `blob_cid` pointing at the bytes. When her grandson opens the album on his laptop, the HEAD resolves instantly from the lean trust-plane (his laptop holds a full arc of it — `project_hub_optional_floor`), and the bytes draw from the nearest of 7 shards across the family's pantries. *Photos load fast (lean HEAD + nearby shards) AND are provably held (the HEAD's `content_hash` verifies the reconstructed bytes; the custody ledger proves ≥ r_floor stewards hold them).* She never sees a tier (`tiered-quilt-stewardship-design.md:59`).

**Value (the donut, care-minted):** Each family member's node holds shards under a `custody-blob` coverage commitment (Rung 3). Honoring it — keeping grandma's album alive through a flood in one city — emits a **care-class** EconomicEvent into shefa. Holding *for the family* mints care; serving *to the commons* mints contribution. The donut's inner ring (the family's guaranteed coverage) and outer ring (commons over-replication) are both REA-accounted. **Care-class and compute-class stay isolated** (`signal_kind` discrimination): a placement-gap (compute breach, `reconcile/custody.rs:214`) never debits grandma's care attribution.

**Governance (capture-resistant stasis):** The coverage invariant (Rung 2) is enforced like a governance contract — ∪ custody ⊇ corpus, ≥ r_floor holders per shard, each in a distinct jurisdiction. A court order in one jurisdiction reaches one steward's shards; 4-of-7 reconstruction means the album survives (`sharding.rs:524`). The DHT's notarized HEADs mean no one can forge "grandma deleted this" — the trust people build on the negotiated values is structurally un-capturable. When a steward must drop custody, the gate **refuses to open a gap and elevates** ("add a steward first") — the system actuates toward coverage and **stays in stasis** against the messy real world: it self-heals (`reconcile/custody.rs` kicks a fetch), never silently degrades.

**The unification:** routing a byte to the right quilt (technical) = minting care for holding it (economic) = enforcing coverage so it can't be captured (governance). One act. That is the coupled story+value+governance the north star asks the substrate to make real — and the two-quilt split is the substrate that makes both **arc** AND **the felt spine** real at once.

---

### Appendix — file:line evidence ledger

| Claim | Evidence |
|---|---|
| HEAD→bytes pointer exists | `content_store_integrity/src/lib.rs:521,524,527` (`blob_cid`/`content_size_bytes`/`content_hash`); design at `:478-487` |
| Byte-plane recipe notarized on DHT | `ShardManifest` `content_store_integrity/src/lib.rs:631`; encodings `:612-617` |
| Who-holds-shard has DHT home | `ShardLocation` `content_store_integrity/src/lib.rs:683` |
| RS(4,7) math done + tested | `sharding.rs:141,301,524-582`; CIDv1 addressing `:435` |
| Custody-as-REA, eager reconcile | `reconcile/custody.rs:114,128-131,149-174,214` |
| Coverage-gate pattern to copy | `arc_actuator.rs:152-172` (`coverage_admits`) |
| Arc policy computes continuous aim | `arc_policy.rs:138` (`derive`) |
| `{0,1}` lever is the kitsune limit (no fork) | `arc_actuator.rs:33-35`; review F1 / `genesis/docs/content/elohim-protocol/history/2026-06-14-arc-factor-feasibility-spike-findings.md` |
| Conductor RAM ∝ corpus (the root) | `project_per_node_memory_is_conductor_authority_arc` |
| Inventory ≠ bytes (gap warning) | `project_inventory_exchange_not_byte_replication`; `reconcile/custody.rs:159-173` |
| DNA entry budget headroom | `rg -c hdk_entry_helper` = 71 in lib.rs (~75/100); `dna/CLAUDE.md` |
| Trust-plane invariant | `dht-is-a-notary-not-a-byte-store.md:86-94` |
| Byte-plane capability bar | `tiered-quilt-stewardship-design.md:50-55,59` |
| REA commitment primitive (the shape) | `project_rea_compute_commitment_primitive` |
