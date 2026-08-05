---
title: "Freenet lift-and-shift — applying the peer-confrontation lessons"
id: freenet-lift-and-shift
tier: plan
status: Open
created: 2026-07-27
maintainers: Matthew Dowell + Claude Opus 5
requires_env: []
topic: [freenet, prior-art, sync-scale-honesty, hosting-budget, reach-negotiation, durability-floor, licensing]
cites:
  - genesis/research/freenet-peer-confrontation-2026-07-27.md
  - genesis/manifests/spine.yaml
  - stewardship-over-sovereignty | stewardship-over-sovereignty | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
---

# Freenet lift-and-shift — implementation plan

Source: [Freenet — Peer Confrontation](../../../research/freenet-peer-confrontation-2026-07-27.md). That document holds the evidence and the take/watch/leave verdicts; this one sequences the work.

**Sequencing principle, taken from Freenet's own failure:** they shipped budgets before they could attribute cost, and got #4868 (an over-budget peer that can never heal). We have the mirror problem — mechanisms with no readers and no telemetry. So **measurement precedes enforcement** in every phase below. The one exception is Phase 0, which has no dependencies at all.

Effort marks: **S** ≤ 1 day · **M** ≤ 1 sprint · **L** design-sized.

---

## Phase 0 — Day-one, zero-dependency (do these first)

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 0.1 | **Root `LICENSE` = CAL-1.0**, plus a relicense sweep. Four incompatible declarations coexist today (AGPL-3.0 storage/client/sdk · Apache-2.0 workspace default/doorway/steward/bridges/eprfs · CAL-1.0 epr/epr-rea/elohim-views/elohim-facings · MIT-OR-Apache bitswap), and Apache-2.0 `doorway-service` takes CAL-1.0 `elohim-views` as a path dep — a live compatibility problem. Absent a root licence the repo is all-rights-reserved; `README.md:299` is a mission statement, not a grant. CAL is the only licence in the set whose text encodes the thesis: AGPL's network clause protects the *code*; CAL obliges portability of the *user's data and keys*. Keep Apache-2.0 only on genuinely reusable bridges. | S | Root `LICENSE` present; `cargo-deny` licence check green; no CAL→Apache path-dep inversion |
| 0.2 | **Write the two retention rules into module docs.** (a) *"XOR distance is a **placement** input, never a **retention** input"* → `reconcile/placement.rs`. (b) *"Gate growth, never convergence — the heal path is exempt from any admission gate"* → wherever the budget lands (Phase 3). Both are one-paragraph rules that prevent a class of bug we have not yet written. | S | Rules present; referenced from the Phase 3 PR |
| 0.3 | **Ontology audit: is there a phantom tier?** Freenet's *"there is no relay category"* collapse. Audit `salvage_capacity` / `peer_blob_inventory` / `custody_announce` / `placement` — is "advertised custody" a real category or a fossil? Symptom on record: 3,430 inventory items exchanged every minute for 36 h while one peer held bytes. | S | A written verdict per category: real, or fossil to delete |
| 0.4 | **Dead-config lint** — fail CI on a declared constant/flag/enum variant with no reader. Our own hit list: `enable_eviction`, `max_storage_bytes`, `salvage_capacity_enabled`, `ELOHIM_TRANSPORT_BACKEND`, `AnnounceChange`, `ListDocumentsSince`, `bloom_filter`, the arc actuator. This is the survey's most reusable artifact and it indicts both projects equally. | M | Lint runs in CI; every current hit either wired or deleted with rationale |
| 0.5 | **`pr-review` mechanics**: dedicated worktree checkout (never `gh pr checkout` — it drags uncommitted changes onto the PR branch, which is exactly our shared-worktree co-commit hazard); "open every cited `file:line` and confirm the finding is real"; mandatory disclosure of which review lenses actually ran. Plus their CI gate: fail any PR touching `skills/**`/`agents/**`/`hooks/**` without a version bump + CHANGELOG entry. | S | Review skill updated; version-check workflow green |

---

## Phase 1 — Measurement (you cannot budget what you cannot attribute)

**Rationale.** Freenet turned an unactionable "60 GB/3h" into six separately-fixable numbers, then killed an arm worth **41% of all wire bytes**. We run **three concurrent, unbudgeted anti-entropy loops** — kitsune2 (120–300 s), inventory gossip (60 s), Automerge doc-sync (60 s) — where their *single* 300 s loop already measures 53.7% of egress. We are structurally worse-positioned and entirely unmeasured.

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 1.1 | **The single highest-leverage missing experiment**: query `/metrics` (`elohim-storage/src/http.rs:1016`) + Prometheus for per-node egress split across the three loops. One query. Do it before anything else in this phase. | S | A number for each loop, dated, in the backlog |
| 1.2 | **Six-arm decomposition** of whichever loop dominates: instrument by *reason*, not by total, so each arm has its own fix. Model: their `broadcast_payload_mix.rs` + `SummaryMissingReason`. | M | Each arm separately graphed; a named owner-fix per arm |
| 1.3 | **"Instrumentation is horizontal" as an enforced rule** — a PR that adds a mechanism without telemetry *for that mechanism* gets bounced. Land as an `.epr-meta` compose-gate over `elohim-storage/src/{reconcile,p2p,services}`. Pair with the telemetry-shape rule (per-node aggregate scalars/rates only; never per-contract or per-request streams — an ingestion DoS on the collector). We already hold the cardinality half informally in `sync_round.rs`; write it down. | S | `.epr-meta` rule live; one PR bounced or amended by it |
| 1.4 | **Metric-design guard**: no headline success metric that retries repair. Freenet's findability reads 97–98% while ~50% of large GETs fail. Audit our green metrics for the same shape. | S | Written audit; any retry-masked metric split into attempt/outcome |

---

## Phase 2 — Close `sync-scale-honesty` (spine node, RED, active)

**The clearest lift in the survey.** Their `summarize / get_delta / apply_delta` is the shipped cure for our `O(peers × corpus)` opener — and we are closer than the spine node records.

Verified state: `ListDocumentsSince` is **handled on both transports** (`p2p/mod.rs:6656`, `p2p_iroh/sync_backend.rs:222`) with a digest short-circuit that logs *"digests match — answering InSync, enumerating nothing"* and falls back to full enumeration on mismatch. It is **never constructed**. `SyncChanges.bloom_filter` is declared (`sync_protocol.rs:64`), always `None` at all three call sites, and ignored by both receivers. **We built the answering half and never built the asking half.**

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 2.1 | Make `round_opener` emit `ListDocumentsSince { digest }` instead of the stateless `ListDocuments`. Keep `p2p/sync_round.rs` the **only** construction site — its module doc already warns that returning requests without a sending caller is the exact fake-green the extraction exists to prevent. | M | `tests/sync_scale_honesty.rs` `the_round_opener_reflects_what_we_already_have` passes; a 2000-doc node and an empty node emit *different* openers |
| 2.2 | Wire `announcements_for_local_change` → a real send site for `AnnounceChange`, so the 60 s poll is no longer the sole propagation path. | M | `a_local_change_is_announced_to_connected_peers` passes with a real wire assertion |
| 2.3 | **Keep the summary O(1) per scope, not O(corpus).** Their #4965 is the pre-learned lesson: once the opener carries state, the summary itself becomes the dominant cost (26.5 KB mean heartbeat, 53.7% of egress). Our digest shape is already cheaper than their per-contract summary set — do not regress it into a set. | — | Summary size constant in corpus size, asserted by test |
| 2.4 | **Do not naively fill `bloom_filter`.** Their paper: approximate summaries have false positives, so the record is omitted from the delta and *"the two replicas remain divergent… The platform offers no built-in fallback."* Our digest + full-enumeration fallback is the safe shape they prescribe. Either delete the field or wire it *with* the fallback. | S | Field deleted, or wired with a divergence-repair round |
| 2.5 | **Canonical serialization is load-bearing.** Their byte-compare over HashMap-ordered summaries fires a full-state heal every heartbeat, and their semantic probe is rationed and *fails open to byte-compare*. Assert canonical encoding for anything entering a digest. | S | Round-trip determinism test over the digest input |

**Spine delta on completion:** `sync-scale-honesty` red → green with evidence.

---

## Phase 3 — Capability-relative budget, then eviction

**Blocked on Phase 1** (no attribution ⇒ a budget masks rather than fixes). Note the correction the survey forced: our arc-scaling spec's premise is partly falsified in-tree — the `arc=0` node sawtoothed identically to full-arc (*"the shape is identical, only the ceiling differs"*) and the jemalloc verdict records flat ~2.7 GB vs glibc's 8–8.5 GB OOM. **The OOM was a glibc-arena leak, not corpus-proportional authority.** Arc-shrink remains right for *DHT authority*; demand-driven budgeting belongs on the *storage plane*, and needs no kitsune2 change.

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 3.1 | **Wire the budget.** `system_metrics.rs:132` **already reads** `/sys/fs/cgroup/memory.max` via `parse_cgroup_mem_limit`; only the consumer is missing. Route → a `hosting_budget` → the blob write path. `config.rs:99-108` already declares `max_storage_bytes` / `enable_eviction` / `min_replicas_for_eviction`. | M | A node with a smaller cgroup limit self-sizes to a smaller budget, observed |
| 3.2 | **Reject the divisor-first shape.** Their `clamp(total_ram/8, …)` punishes small nodes — *"the divisor binds before the ceiling, so raising the clamp doesn't help"* — and a 2-core peer gets `pool_size = 1` with **exports disabled entirely** (159 WASM timeouts/hour measured). That directly contradicts our hub-optional floor. Size from available headroom with a floor that keeps a laptop a *full* participant. | S | A laptop-class profile retains full participation, asserted |
| 3.3 | **Heal path exempt from the admission gate** (see 0.2b). #4868 pre-learned: gate growth, never convergence. | S | Test: an over-budget node still converges |
| 3.4 | **Demand-ordered eviction** — `(local_subs, downstream_subs, recency, key_bytes)`, no separate admission decision (a newcomer arrives with ≤1 subscriber and so can never displace a 2+-subscriber incumbent — an emergent refusal, and the free spam bound on every-hop placement). `metadata.rs:134 get_lru_candidates` already sorts by `last_accessed` and has **zero callers**. | M | Eviction runs; a real-demand item is not evicted ahead of junk |
| 3.5 | **WATCH — do not assume the ranking.** Their own epic cannot remove the redundancy it calls waste: dropping `relay_put_replicate_forward` moved distant-GET findability **0.854 → 0.521**, below their 0.60 floor. Add a findability floor metric *before* trusting eviction. | S | Findability floor graphed and alerting |

---

## Phase 4 — Make the pledge bind (the durability floor)

The survey's verdict, not to be softened: *"a promise with no breach detector and no consequence is a preference."* A durability floor must name **(i)** who owes bytes, **(ii)** a probe that detects non-delivery, **(iii)** a consequence. We have (i) only, degenerately — `dht_anchor_hash` NULL, `let pledged_commons = 0u64`, zero grep hits for `shortfall|breach|unmet|under_replicat`, and no eviction to make the pledge bind.

The primitive already exists: `salvage_commitment_author.rs` authors a **notarized `custody-blob` REA commitment** on under-replication, wired at `main.rs:1786-1866`, target 2 replicas — gated `salvage_capacity_enabled: false`, with `SALVAGE_CAPACITY_ENABLED` in **zero manifests**. The `imago-dei` rule that *a node is never conscripted* is correct and stays; the resolution is that **a commitment is consent that binds**.

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 4.1 | **(ii) Breach detection** — a probe answering "is this pledge being honoured?" The missing organ, and what makes the pledge falsifiable. | M | A pledged-but-absent byte range is detected and reported |
| 4.2 | **Plumb `dht_anchor_hash`** through the Mishpat coordinator path so the pledge is a DHT fact, not a storage-local row. | M | Pledge resolvable from the DHT |
| 4.3 | **(iii) Consequence** — ordered by canon: restored capability, never punishment. Minimum viable: eviction ordering reads the pledge (*"shed uncommitted first"*), so an over-budget node has somewhere legitimate to push. This is what Freenet structurally cannot do — #4868 is unfixable precisely because they have a local budget and **no counterparty who owes them bytes**. | M | Eviction consults commitments; over-budget node sheds uncommitted first |
| 4.4 | **Turn salvage on** behind the commitment, with the flag actually present in manifests. Consented *and* binding. | S | `SALVAGE_CAPACITY_ENABLED` in manifests; a real salvage commitment authored on alpha |
| 4.5 | **Upstream computed, not stored** — add a *computed* liveness derivation beside the stored `replicates-dwelling` pledge and make the fold read the computation. Keep the pledge (the primitive Freenet lacks). **Guard:** computed liveness must not become a covert revocation path the promisor did not author. | L | Fold reads computed liveness; revocation remains promisor-authored |

---

## Phase 5 — The negotiation ceremony (new design work)

The one genuinely novel piece, and the thing Freenet cannot build at all: their `validate_state(state, params, related)` has **no requester parameter**, so there is nobody to negotiate with; and their merge is total, so there is no verdict between "merged" and nothing.

We hold all three pieces and none are connected:

| Part | Where | State |
|---|---|---|
| A verdict meaning *negotiate*, not refuse | `reach_earning.rs` — `ReachVerdict::Pending → Refer` | zero production call sites |
| A primitive naming an outside counterparty | `bridges/did` (doorway already consumes `did-bridge`) | wired, unused for this |
| A venue | doorway + bridge seam (`bridges/{did,pkarr,valueflows}`) | exists |

| # | Task | Effort | Evidence of done |
|---|---|---|---|
| 5.1 | **Classify doorway traffic explicitly** — the doorway is a *projector*, not the authority. (a) conductor-backed session: identity carried to `check_reach_authorization`, response session-scoped or uncached; (b) anonymous web2: `commons`/`public` only, identity-free and therefore safely cacheable (this is why `CacheKey` has no requester dimension — correct for its class); (c) negotiation. Ensure above-`community` responses never land in the identity-free cache. | M | Test: an `intimate` response cannot be served from cache to a second requester |
| 5.2 | **Delete the fossil**, don't repair it — `doorway/.../cache/access_control.rs:44-66` still matches the geographic-8 vocabulary the reach-ontology split renamed *out* of reach, with canonical tiers falling through to `_ => false`. | S | Dead vocabulary gone; dead-config lint (0.4) green |
| 5.3 | **Design the ceremony**: `Pending → Refer` as the trigger, DID as the counterparty, doorway+bridge as the venue, a consent record as the artifact. Route the outcome back through `check_reach_authorization`'s relationship arms so a granted negotiation becomes ordinary relationship state rather than a special case. | L | Design doc + a2o scenario |
| 5.4 | **Close the identity-plumbing gap** that starves the gate: enforcement of the steward/relationship arms is *"coupled to `humans.agent_pub_key` population"* — provable on the household stack, data-starved on alpha. | M | Higher-tier arms provable on alpha |
| 5.5 | **Governance binds only volunteers today** — `genesis_self_check` is `Ok(Valid)` with an unread `_data` param in all four integrity zomes, and there are **zero `membrane_proof` occurrences** in the DNA. Put *something* in it — even a signed invite proving a commitment exists — so the envelope is entered, not merely offered. | M | A join without the envelope is refused |

---

## Explicitly not doing

Carried from the survey's **leave behind**, recorded so nobody re-proposes them:

- Location derived from any network address. Ours is `blake2b_128(32-byte key)` XOR-folded with zero network input; their own code calls their scheme a *"DISCLOSED and ACCEPTED"* eclipse tradeoff with an unevictable attacker.
- Total merge as the state model — it forbids the refusals our governance is built from. *(Honest mirror: our Automerge plane has the same limit; refusal lives only in the DNA.)*
- Isotonic-regression adaptive routing — demoted out of eviction by their own code, and moot for a full-arc fleet with no next-hop choice.
- Path-scoped advisory rule files — `.epr-meta` is a strict superset. *(Do adopt their repeat-offender history table convention: named regression pins so a revert fails CI with an issue-numbered message.)*
- Their status table as an internal artifact — `spine.yaml` is stricter. Publishing a dated status artifact *externally* remains worth studying.

## The rule to carry out of all of it

> **Do not size constants for the network you have.** `MAX_HOPS_TO_LIVE = 10` is theirs — the median path crosses it near ~100k peers and requests then die as `HtlExhausted`, which *looks* like unroutability and is not. **`target_arc_factor = 1` is ours, one layer down.**
