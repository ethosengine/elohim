---
title: "Freenet — Peer Confrontation: same problem, different bet"
id: freenet-peer-confrontation-2026-07-27
status: Capture
date: 2026-07-27
---

# Freenet — Peer Confrontation

*Not a cross-pollination survey. Freenet is the closest peer we have found to our own problem — a full-stack, actively-developed, decentralized substrate with a live network, a whitepaper, published telemetry, and an agent-development discipline that mirrors ours. This document compares, attacks in both directions, and separates what we **take**, what we **watch**, and what we **leave behind**.*

**Method:** four repos cloned and read directly (`freenet-core` @ 0.2.112, `freenet-git`, `freenet-agent-skills`, `paper-1`), the org's remaining 16 repos surveyed from source; a 13-agent parallel recon + 4-lens adversarial panel + completeness critic, then a 3-agent red/green pair on onboarding-at-scale plus a governance-gap analysis. Their `crates/topology-sim` was **built and run at HEAD** (seed 42, N = 300 and 3000). Elohim-side claims are grounded in our own tree with `file:line`, and several of the survey's own early conclusions were refuted by the critic and are marked as such.

**Verification key:** ✅ verified in source or by running it · ◐ single-source, plausible · ⚠ UNVERIFIED / contested.

---

## 1. The bet each project is making

Freenet's thesis, verbatim (`paper-1/sections/02-thesis.tex`):

> *"separate **what merges** from **how it propagates**. The application defines the algebra of its state… The platform is generic over the algebra; the application carries the lattice."*

Four primitives: **contracts** (idempotent commutative monoids on byte state + a validity predicate), **summary/delta sync** (`summarize / get_delta / apply_delta`, contract-supplied), **small-world adaptive routing** (Kleinberg 1/d on a ring), and **delegates** (WASM modules holding private device-local state).

Ours is the inverse trade. We fix the substrate's semantics — a validating DHT with agent-scoped source chains, notarized provenance, an 8-level reach ordinal, and REA commitments — and let applications compose within them. Freenet buys generality and pays in refusal; we buy refusal and pay in generality.

That single sentence predicts nearly every difference below.

### One hash where we use two layers

Freenet founders describe the BLAKE3 WASM contract as the platform's key feature. It is worth being precise about what it addresses, because the natural reading is wrong — and their own paper says so.

`ContractKey = blake3(params ‖ wasm_code)` ✅ (`wasm_runtime/secrets_store/store.rs:402`). **This is Holochain's DNA-hash pattern, not our EPR CID pattern:** code + parameters → instance identity → namespace boundary, with the ring location `ℓ(contract)` derived from it. Same code with different params is a different key and effectively a different network — precisely as a Holochain DNA hash (integrity zomes + modifiers) partitions the DHT.

The difference is that Holochain has **two** layers and Freenet has one. Holochain addresses the *namespace* by DNA hash **and** addresses the *entries* by their own content hash, peer-validated. Freenet content-addresses the **validator** and never the **validated** — their only per-state hash is a non-cryptographic `u64` change-detector. Their paper is explicit and unusually careful about it (`05-routing.tex:71`):

> *"The requester can verify on arrival that the served code and parameters hash to k. **State is not directly verifiable from the key**: the key is derived from code and params, not from state, and state is mutable. This is a meaningful difference from immutable content-addressed stores such as IPFS… **Readers carrying that mental model should note that the guarantee does not transfer to Freenet contracts.** State is trusted only to the extent that the contract's validity predicate accepts it."*

So the trust chain terminates at *"a validator I can verify ran"*, never at *"bytes whose identity I can check."* That is what makes §8's censorship-primitive concern structural rather than incidental.

**The corollary is a genuine win for them, and it is instructive.** Because the key is code-addressed, upgrading is free: a new version is simply a new key, the old key keeps working, and there is *"no version negotiation, no upgrade protocol."* Our equivalent move is gated behind `ALLOW_DNA_REINSTALL` and is painful — not because the hashing differs, but because **our agent identity is bound to the namespace and theirs is not.** A Holochain reinstall mints a new agent key; a Freenet delegate holds its keys locally, independent of any contract key. Same content-addressing pattern, opposite ergonomics, and the deciding variable is *where identity lives*. Worth holding when we revisit the reinstall path.

---

## 2. Where Freenet is simply ahead

State this first, without hedging.

- **Real deployment.** Hundreds to low thousands of live peers ⚠ (three unreconciled figures: ≈440 in the whitepaper, "634–678" in #4956, "1343 → 1770" in #4970 — do not quote a point estimate). Ours: 14 declared humans, 7 suspended, a ~6-peer live fabric, **all synthetic users**, largest corpus 3,654 docs.
- **They publish their worst numbers.** #4965: anti-entropy summaries are **53.7% of all outbound bytes**, with a `never_populated` arm at **99.8%**. #4956: 67% of applies are no-ops carrying 80% of bytes. #4800: ~50% of large streamed GETs fail. A whitepaper §Open Problems that says *"a single data point at a single N cannot distinguish O(log n) from O(√n), O(n^0.4), or a constant."*
- **Budgets that actually bind.** `clamp(total_ram/8, 128 MiB, 1 GiB)` from `min(RAM, cgroup)`, plus a disk axis and a pre-write admission gate ✅. Ours: `enable_eviction: true` with **zero readers**, `max_storage_bytes` default `0` (unlimited) with **zero storage-write callers**, `get_lru_candidates` with **zero callers**, and `identity.rs` advertising 10/100/500 GB tiers that nothing enforces ✅.
- **A simulator that catches real bugs**, and 7 releases in 34 hours.

**Correction to an earlier framing in our own analysis:** "Elohim publishes no measurement" is **false**. Our backlog RCAs carry real fleet numbers (`content_local_anchored: 4188`, `content_divergent_anchor: 3599`, zero healed; 62 Matthew-advertised REA rows returning none on Adam). The accurate claim is narrower: *no aggregate fleet-bandwidth accounting, and no artifact published outside the repo.*

---

## 3. What happens when everyone comes onboard

Red/green pair. GREEN built and ran their simulator; that settles most of it.

| N | strategy | greedy success | avg hops |
|---|---|---|---|
| 300 | current | 100.0% | 8.0 |
| 300 | small_world | 99.7% | **2.8** |
| 3000 | current | **43.1%** | 26.9 |
| 3000 | small_world | 99.9% | **4.8** |

✅ (30k/100k arms are O(n) per `k_nearest`; killed after ~25 min — ⚠ above N=3000.)

The catastrophic `current` row is labelled *"Current Freenet algorithm (as of v0.1.128)"* while HEAD is 0.2.112; production migrated to `kleinberg_target` / `gap_target_directional` (`topology.rs:661,666`) and scores inbound candidates via `kleinberg_score` (`connection_manager.rs:1013`). **The failing arm is a museum piece — the simulator exists to have caught it.** The shipped arm fits `c·log²N`, c ≈ 0.037 → ~6.5 hops at 10k, ~10.2 at 100k, ~14.7 at 1M.

**Adjudication:**

| Concern | Verdict |
|---|---|
| Routing quality | **Scales.** 10× peers ≈ 1.7× hops. Best-behaved subsystem they have. |
| Hosting capacity | **Anti-fragile.** Each joiner adds 128 MiB–1 GiB; aggregate capacity is linear in N while per-contract demand is sublinear. Newcomers bring more than they take. |
| Anti-entropy 53.7% | **A defect class, not a scale law.** `summary_cache_count_target(hosted)` sizes to hosted-contract count, not network size; budgets are RAM-clamped. A scale law does not present at 99.8% on one arm and 0.2% on three others. They have named in-flight fixes per arm. |
| **HTL = 10** | **The real ceiling.** `DEFAULT_MAX_HOPS_TO_LIVE = 10` (`config.rs:46`, `ring.rs:529`). Median path crosses it near ~100k peers; requests then die as `HtlExhausted`, which *looks like* unroutability and is not. A one-line constant. |
| Admission at launch spikes | **Unrefuted RED win.** Rejected candidates are still recorded (`connection_evaluator.rs:33-41`) and the threshold is a running max over a 60 s/300 s window — classical record-value statistics, so accepts ≈ `ln λ` while offered load is `λ`. 10 offers/min → 29% accept; 1,000/min → 0.75%. Join latency diverges superlinearly *exactly* during a popularity spike. |
| CGNAT / IPv6 locations | **Structural, both directions.** Location is a hash of the IP *prefix* — `& 0xFFFFFF00` (/24) or three IPv6 segments (/48). IPv4 mass onboarding **collapses** the ring (routing-identical peers make zero greedy progress); IPv6 makes locations **free to forge**. There is no configuration between those two failures. |
| Cold / niche content | **Unrefuted RED win.** New content has zero subscribers by construction, so it is always first evicted. `freenet-git rescue` as a cron job is the empirical proof. |
| Sybil / abuse | **Declared OPEN by them.** Uncontested. |

**Their own summary of the compounding case, which they do not make:** at ~10³ peers, >50% of egress is coordination whose useful-work fraction is 0.2% on the dominant arm, ~50% of large GETs fail — **while the headline findability metric reads 97–98% because retries hide it.** That metric-design trap is worth naming in our own instrumentation rules.

---

## 4. The reach gradient — what Freenet structurally cannot build

This is the deepest divergence, and it is not a maturity gap.

Freenet's contract interface is `validate_state(state, params, related)` (`wasm_runtime/contract.rs:16-21`) ✅. **There is no requester parameter.** A contract cannot know who is asking, so authorization is *inexpressible* at the layer that owns state. Their model is binary with no middle:

| | Freenet | Elohim |
|---|---|---|
| Replicated / open | contract state — served to whoever routes a GET | `commons`, `public` |
| Device-local secret | delegate state — never replicated | `private`, `self` |
| **Everything between** | **encryption only** — the "shared-secret contract", marked **Partial, "not yet wired up"** in their own status table | `intimate → trusted → familiar → community` |

Encryption is not a gradient: a binary keyholder set, no revocation without re-encrypt-and-redistribute, no consent record, no per-viewer decision. And because hosting is demand-driven with every-hop placement, **your ciphertext lands on strangers' disks by construction** — custody is involuntary on both sides at once.

**Their surveillance surface is the inverse of the one we want.** `ring/interest.rs:59-66` ✅: `INTEREST_HEARTBEAT_INTERVAL = 300s`, and *"Each heartbeat sends a full `Interests { hashes }` message"*, TTL 20 minutes. Every five minutes a peer broadcasts its **complete set of contract hashes** to every neighbour (up to 200). Your reading list is public to your neighbourhood. Compose with hop-by-hop-only encryption (intermediates see payloads) and location-as-IP-prefix, and interest + ring position + timing correlate to a real subscriber.

### What we have

Canonical, DNA-notarized as `REACH_LEVELS` in `content_store_integrity`, `_ordinal: true` ✅:

```
private → self → intimate → trusted → familiar → community → public → commons
```

`commons` is the top of the ladder — the commons pool *is* reach-8, and "below commons reach" is exactly levels 1–7. `elohim/epr/src/reach.rs` deliberately refuses `#[derive(Ord)]`, with a comment noting that derived ordering would make `Private < Commons` — the inverse of semantic openness — so comparisons must route through `openness()`. Freenet has no analog at any layer.

**Shipped and fail-closed** ✅ — `elohim-storage/src/epr_service.rs:334 check_reach_authorization`, whose tier arms are *relationship-derived, not label-matching*:

- `community` — consented collective membership
- `familiar` — shared collective with a content steward
- `trusted` — relationship with steward at intimacy ≥ trusted
- `intimate` — **mutual** intimate relationship (both consents)
- `self` / `private` — agent is the creator

with an ambient fast-path off the P2P trust cache for community-and-below, `commons`/`public` needing no identity check, and everything above resolving the requester to a `Human` or denying. *"The tier arms live in ONE place"* — shared by the P2P transport path and the HTTP serve path. `bounds_validator.rs:228-262` independently enforces reach-rank ≤ `reach_ceiling`, unknown strings → `ReachCeilingExceeded`, fail-closed.

**Honest caveat from that same file:** end-to-end enforcement of the steward/relationship arms is *"coupled to `humans.agent_pub_key` population… On the household stack the gate is fully provable today."* Correct code, data-starved on alpha — an identity-plumbing gap, not a design flaw.

### The doorway is a projector, not the authority

The canonical gate is in the **p2p dataplane**; the doorway carries identity to it (`routes/api.rs:214,231` — `parse_requester_identity`, then `resolve_with_identity(doc_type, id, requester)`) ✅. Three traffic classes, each correct on its own terms:

1. **Conductor-backed session** — the doorway projects what *your imagodei* is entitled to see. It grants nothing.
2. **Anonymous web2** — absorbed at the doorway; `commons`/`public` is the entire visible surface. Identity-free, therefore safely cacheable. This is why `CacheKey { dna_hash, zome, fn_name, args_hash, reach }` has no requester dimension — that is the right key *for this class*.
3. **Outside-in negotiation** — the doorway as **venue**, via bridge-crates (`bridges/{did,pkarr,valueflows}`; doorway already consumes `did-bridge`) and sync services.

The residual is therefore **classification hygiene, not a missing gate**: ensure above-`community` responses ride the identity-bearing conductor path (session-keyed or uncached) and never land in the identity-free cache. The dead vocabulary in `doorway/.../cache/access_control.rs:44-66` — still matching the geographic-8 (`invited|local|neighborhood|municipal|bioregional|regional`) that the 2026-07-22 spec renamed **out** of reach, with canonical tiers falling through to `_ => false` — is the fossil of attempting this at the wrong layer. Delete it rather than repair it.

### The ceremony we already half-own

The negotiation the outside world should have to conduct needs three parts. **We hold all three and none are connected:**

| Part | Where it lives | State |
|---|---|---|
| A verdict meaning *"negotiate this"* rather than refuse | `reach_earning.rs` — `ReachVerdict::Pending → Refer`, never `Refuse` | **zero production call sites** (one caller, `epr_compose.rs:52`, reached only from a test) |
| A primitive naming an outside counterparty | `bridges/did` — consumed by doorway | wired, unused for this |
| A venue for the ceremony | doorway + bridge seam | exists |

Freenet cannot express even the first. Their merge is **total** — *"the merge model cannot express 'one of these updates is rejected unconditionally'"* — so there is no refusal, no deferral, no counterparty, and no venue. Their outside-in options are: publish it (world-readable forever) or encrypt it (nobody-readable, no revocation).

---

## 5. Their sovereignty frame — and why we reject the framing, not just the answer

*Canon: [`stewardship-over-sovereignty.md`](../docs/architecture/stewardship-over-sovereignty.md) §1; `values-forward.md` Stance II.4. The protocol does not treat cryptographic self-custody as sovereignty, and does not accept sovereignty as the governing frame at all. What follows describes Freenet's stance in Freenet's terms; it is not a contest between two sovereignties with a winner.*

Freenet's identity story is `ghostkeys`: blind-signed anonymous credentials, unlinkable, donation-gated — the crypto-libertarian lineage in its most refined form. In-tree greps across `freenet-core/crates/`: `reputation` **0 hits**, `moderation` **0 hits**, `author_id` **0 hits** ✅. (The 614 "governance" hits are MAD-based *resource* outlier detection; the 284 "attribution" hits are *peer* telemetry attribution.) There is **no community tier at any layer** — the apex unit is the unlinkable individual, and the guarantee offered is **invisibility**.

That frame produces exactly the failures canon §1 enumerates, and Freenet inherits them wholesale: no notion of duress, no notion of capacity, no recourse for the person who loses their keys, and a substrate legible only to the crypto-literate. It is not that they chose the wrong apex — it is that an apex-individual model has nowhere to put the grandmother, the child, or the coerced spouse.

Ours is not a competing sovereignty. `intimate / trusted / familiar / community` are **social positions, not cryptographic ones**: what you can reach is a function of relationships others also consent to, so the community is the backstop *above* the individual rather than a service beneath them. The property is **negotiated legibility** — the inverse of invisibility, and not a stronger form of it.

**A substrate where content has no author and abuse has no address structurally selects for uses that want no author.** That is the darknet attractor, and it is not a slur — it is their lineage and partly their intent. Our counter-mechanism is real and narrow: `bounds_validator.rs` is fail-closed on un-notarized provenance (`FetchError::NotarizedRequired` → `CommitmentNotFound`, refusing before the conductor is contacted) ✅.

### The tension, stated plainly

> The `imago-dei` principle that *"a node is never conscripted"* is what gives us consent-based custody — and it is **precisely** what leaves `salvage_capacity_enabled: false` and no durability floor.

These are one coin. `elohim-storage/src/services/salvage_commitment_author.rs` already implements the resolution: under-replication → self-selection → *"a NOTARIZED placement intent — a `custody-blob` REA commitment… through the conductor"*, wired at `main.rs:1786-1866`, target 2 replicas ✅. It is gated `salvage_capacity_enabled: false`, and `SALVAGE_CAPACITY_ENABLED` appears in **zero** deployment manifests ✅.

**A commitment is consent that binds.** An agent voluntarily enters a hosting obligation; the obligation then becomes enforceable against them. The primitive that fixes durability *is* the primitive that delivers negotiated custody. Freenet has neither, because it has no counterparty vocabulary at all — which is exactly why their **#4868** is structurally unfixable: an over-budget peer permanently diverges because the growth UPDATE *and* its ResyncResponse heal hit the same local admission gate. With only a local budget and nobody who *owes* you the bytes, there is no convergence path. Our `ReplicatesDwellingPayload` names an addressee, so eviction can order *"shed uncommitted first"* and an over-budget node has somewhere to push.

**But the critic's verdict stands, and it is the honest one:** a durability floor must name (i) who owes bytes, (ii) a probe that detects non-delivery, (iii) a consequence. We have (i) only, and degenerately — `dht_anchor_hash` NULL, `let pledged_commons = 0u64`, zero grep hits for `shortfall|breach|unmet|under_replicat`, and no eviction to make the pledge bind. *"A promise with no breach detector and no consequence is a preference."* **Today both durability floors are project-operated centralized infrastructure**: theirs is an owner-run node plus a GitHub Actions cron; ours is full-arc replication on operator-run PVCs reconciled by Jenkins. The decentralization comparison is currently a comparison of two roadmaps.

---

## 6. TAKE

| # | Pattern | Landing site | Verdict |
|---|---|---|---|
| 1 | **Capability-relative budget from `min(RAM, cgroup)`** | `system_metrics.rs:132` **already reads** `/sys/fs/cgroup/memory.max`; the consumer is missing. Wire → a `hosting_budget` → the blob write path. `config.rs:99-108` already declares the knobs. | **ADOPT NOW** (~2–3 days) |
| 2 | **"Distance is not an eviction input"** — placement input, never retention input | `reconcile/placement.rs` `XorDistanceStrategy`. Write it into the module doc. Freenet demoted their own estimator to telemetry with *"Do NOT re-wire this estimate back into the eviction sort."* | **ADOPT NOW** (~1 hr) |
| 3 | **"Upstream is computed, not stored"** — derived state self-corrects, formation flags rot; strict total order ⇒ acyclic by construction | The sharpest contradiction with current design: `replicates_dwelling.rs` is a stored formation flag with no breach detection. **Keep the pledge** (it is the primitive Freenet lacks) and add a *computed* liveness derivation the fold reads. | **ADOPT NOW** (principle) / **LATER** (impl) |
| 4 | **"Instrumentation is horizontal"** — a PR adding a mechanism without telemetry *for that mechanism* gets bounced | `.epr-meta` compose-gate over `elohim-storage/src/{reconcile,p2p,services}`. Would have caught every inert mechanism we have. | **ADOPT NOW** (hours) |
| 5 | **Six-arm cost decomposition** — turn "N GB/window" into N separately-fixable numbers | Our three unbudgeted anti-entropy loops. Their `broadcast_payload_mix.rs` killed an arm worth **41% of all wire bytes**. | **ADOPT NOW** |
| 6 | **`pr-review` mechanics** — dedicated worktree checkout (never `gh pr checkout`), "open every cited `file:line` and confirm the finding is real", mandatory disclosure of which lenses ran; plus their CI gate failing any PR touching `skills/**`/`agents/**`/`hooks/**` without a version bump | Our review skills + the shared-worktree co-commit hazard | **ADOPT NOW** (~1 day) |
| 7 | **Build-time lineage guard** — a registry that fails the build on an unregistered hash change | Would have caught the alpha genesis-pair partition class. (The rest of `freenet-migrate` is v0.4.0/6 commits — study only.) | **ADOPT LATER** |
| 8 | **Deterministic seeded simulation** (the discipline, not their crate) | We verifiably have **no simulator**; highest N ever run is 14 containers. Their `topology-sim` is 1061 LOC with zero tests and zero asserts — take the idea. | **ADOPT LATER** (spike) |
| 9 | **Ontology audit: "there is no relay category"** | Audit the `salvage_capacity` / `peer_blob_inventory` / `custody_announce` / `placement` quadruple for a phantom tier. Symptom on record: 3430 inventory items exchanged every minute for 36 h while one peer held bytes. | **ADOPT NOW** (~4 hrs) |
| 10 | **Repeat-offender history table** — named source-level regression pins so a revert fails CI with an issue-numbered message | Fold into existing `.epr-meta` manifests | **ADOPT NOW** |

---

## 7. WATCH — take with modification, or take the lesson not the code

- **Demand-ordered eviction is not settled, even for them.** Their own epic cannot remove the redundancy it calls waste: dropping `relay_put_replicate_forward` moves distant-GET findability **0.854 → 0.521**, below their own 0.60 floor. Take the budget and the rule; do not assume the ranking.
- **Their budget shape punishes small nodes — the opposite of our hub-optional floor.** Executor pool is `available_parallelism() - 1` clamped `[1,16]`, so a 2-core peer gets `pool_size = 1` and **exports are disabled entirely**; cache budget is `clamp(total_ram/8, …)` where *"the divisor binds before the ceiling, so raising the clamp doesn't help small nodes."* A 2-core node logged **159 WASM timeouts/hour**. Adopt capability-relative budgeting; **reject the divisor-first shape.**
- **Exempt the heal path from any admission gate.** #4868 is the pre-learned lesson: gate growth, never convergence. This is also the failure mode our own fail-closed `bounds_validator` shape would have if a heal ever crossed it.
- **Canonical serialization is load-bearing.** Byte-compare over HashMap-ordered summaries *"flag[s] a converged peer stale and fire[s] a full-state heal every heartbeat"*, and their semantic probe is rationed at 32/round and **fails open to byte-compare**. Non-canonical encoding is simultaneously a bandwidth amplifier and — via `broken_invariants.rs`, whose TTL doubles to "mostly-dark" — an availability weapon whose only confirmed firings were false positives (#4295).
- **Beware headline metrics that retries repair.** 97–98% findability over ~50% large-GET failure.
- **Do not size constants for the network you have.** `MAX_HOPS_TO_LIVE = 10` is their version; **`target_arc_factor = 1` is ours, one layer down.**

---

## 8. LEAVE BEHIND

- **Location derived from network address.** Their `connection_manager.rs:818-843` is a signed confession: *"SECURITY (eclipse) — DISCLOSED and ACCEPTED tradeoff… a single routed IPv6 allocation already yields a VAST number of distinct /48 `Location`s essentially for free… The project lead has EXPLICITLY ACCEPTED this tradeoff."* Lattice admission is deterministic and the score-based swap was removed, so **a well-behaved eclipser is unevictable**. Our location is `blake2b_128(32-byte key)` XOR-folded with **zero network input** — the grinding surface does not exist for us. It also means honest mobile/VPN/CGNAT users teleport on their ring (#4951: suspend/resume → 5.5 h at 1 connection).
- **State unverifiable from the key.** `ContractKey = BLAKE3(BLAKE3(wasm) ‖ params)` binds *code*, never state; the only per-state hash is a non-cryptographic `u64` change-detector. Composed with the eclipse, an eclipsed requester has no oracle: you can hold a room, a repo ref, or a moderation list at an arbitrary past instant, undetectably.
- **Total merge as the state model.** It forbids the class our governance is built from — `AgentPeerBinding` immutability, `ContentSuccession` rejecting self-succession, qahal rejecting `initial_tier != T0`, `commons_pool_tribute ∈ (0.0,1.0]`. Anything with uniqueness or non-reissuance is unexpressible on a total-merge lattice. *(Honest mirror: our Automerge plane has the same limit — refusal lives only in the DNA.)*
- **Isotonic-regression adaptive routing.** Their own code demoted it out of eviction; aging is a 500-count FIFO with no time decay; the refit cost is ~5× the quoted figure; and 96% of the time the cost function collapses to `failure_estimate * 3.0` because timed GET successes are ~4% of operations. Decisive for fit: we are full-arc, so there is no next-hop choice to optimise.
- **Path-scoped advisory rule files.** Our `.epr-meta` compose-gate is a strict superset (a PreToolUse hook that *denies*).
- **Their status table as an internal artifact.** `spine.yaml` is stricter (max-12 covenant, `unwired` is not schedulable, evidence-gated flips). Their table is where their contradictions live — it claims deployed WASM "fuel + memory limits" while `enable_metering: false`. *Publishing* a dated status artifact externally is still worth studying.

---

## 9. What this changes in our specs and architecture

1. **`2026-06-13-conductor-authority-arc-memory-scaling.md` needs a second lever and a correction.** It proposes arc-shrink — a *keyspace* partition — and explicitly parks the hard part: *"the open design work is the **policy**: how arc-factor is chosen per device archetype."* Freenet **deletes that policy question** by partitioning on *demand* within a capability-relative budget, and deliberately rejects distance as a retention input. Also, the spec's own premise is partly falsified in-tree: the `arc=0` node sawtoothed identically to full-arc (*"the shape is identical, only the ceiling differs"*) and the jemalloc verdict records flat ~2.7 GB vs glibc's 8–8.5 GB OOM. **The OOM was a glibc-arena leak, not corpus-proportional authority.** Arc-shrink remains right for DHT authority; the *storage plane* wants demand-driven budgeting, and it needs no kitsune2 change.
2. **`sync-scale-honesty` (spine, red) has a shipped reference design.** Their `summarize/get_delta/apply_delta` is the cure for our `O(peers × corpus)` opener. We are closer than the node records: `ListDocumentsSince` is **handled on both transports** with a digest short-circuit (*"digests match — answering InSync, enumerating nothing"*) and full-enumeration fallback — and is **never constructed**. `SyncChanges.bloom_filter` is declared, always `None`, ignored by both receivers. **We built the answering half and never built the asking half.** Two warnings come free: their paper says approximate summaries leave replicas *permanently* divergent with no built-in fallback (our digest+fallback is the safe shape); and #4965 says keep the summary O(1) per scope, not O(corpus).
3. **The reach spec should absorb the negotiation ceremony.** `ReachVerdict::Pending → Refer` is the verdict that means *negotiate*; `bridges/did` names the counterparty; the doorway is the venue. Connect the three. This is the one genuinely new design work, and it is the thing Freenet cannot build.
4. **Root `LICENSE` is a day-one anti-enclosure gap.** There is none, and four incompatible declarations coexist — AGPL-3.0 (storage, storage-client, sdk), Apache-2.0 (workspace default, doorway, steward, bridges, eprfs), CAL-1.0 (epr, epr-rea, elohim-views, elohim-facings), MIT-OR-Apache (bitswap) — plus a live compatibility problem: Apache-2.0 `doorway-service` takes CAL-1.0 `elohim-views` as a path dep. `README.md:299` offers a mission statement, not a grant; absent a licence this is all-rights-reserved. **Recommend CAL-1.0 at the root:** it is the only licence in the set whose text encodes the thesis — AGPL's network clause protects the *code*; CAL obliges portability of the *user's data and keys*, which is what "a commons owned by no-one" actually means.
5. **Governance binds only volunteers.** `genesis_self_check` is `Ok(Valid)` with an **unread `_data` param in all four integrity zomes**, and there are **zero `membrane_proof` occurrences** anywhere in the DNA. Joining is as unconditioned as Freenet's. Embedded governance is today *opt-in* governance — the identical critique as the opt-in durability floor.
6. **A "constant or flag with no reader" lint** is the most reusable artifact this survey produced, and it indicts both projects. Ours: `enable_eviction`, `max_storage_bytes`, `salvage_capacity_enabled`, `ELOHIM_TRANSPORT_BACKEND`, `AnnounceChange`, `ListDocumentsSince`, `bloom_filter`, the arc actuator. Theirs: `enable_metering`, the random-walk constant, the 100 Mbps docs. **The shared pathology is prose accreting faster than enforcement — dead configuration that reads as shipped capability.**

---

## 10. What we did not verify

- Their live peer count ⚠ (three unreconciled figures).
- Simulator behaviour above N=3000 ⚠.
- Eclipse cost in currency ⚠ (external pricing, not in repo).
- `freenet-stdlib` was not cloned; third-party developer experience is inferred. Related and unremarked elsewhere: **zero third-party apps exist** — all 16 satellite repos are under the same org.
- Neither project's real user population. Ours is entirely synthetic fixtures.
- Their bug list is dominated by residential NAT/churn; our edge nodes run in operator k8s behind public ingress. **Absence of that bug class is not robustness — we have never met it.**

## Credit

Ian Clarke and the Freenet contributors built this, published their worst numbers, wrote a whitepaper with an honest §Open Problems, and documented an accepted security tradeoff in a source comment rather than a footnote. The quality of this survey is a direct function of that openness. Their `demand-driven-hosting.md` is the single best piece of distributed-systems design writing either project has produced.
