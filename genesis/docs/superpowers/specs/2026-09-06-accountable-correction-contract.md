---
title: "Accountable correction — the contract for slice 0: who may amend a record, how a witnessed correction is discovered and applied once, and what a peer shows when it cannot know"
id: accountable-correction-contract
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
serves:
  - dataplane-convergence
graduation-trigger: "Draft→Active when the 2026-09-06 adversarial read (Codex) and P2P design-gate (rust-architect) findings are all addressed in the text — done in revision 2 — and reviewed once more with no blocker and slice 1's eight stations are written as scenarios bound to @concern:accountable-correction under dataplane-convergence; Active→Canonical when those stations pass on the household mesh with a receipt under genesis/a2o/reports/ and the habit atom carries the DELTA"
created: 2026-09-06
domain: D2
topic: [correction, feedback-signal, head-adoption, successor-authority, discovery, replay, idempotency, standing]
boundary: "Slice 0 of the reimplementation sequence sealed 2026-09-06 (decisions D0–D10 in the second-opinion plan). This contract governs ONE record kind (a content EPR with a declared head) and ONE act (a FeedbackSignal of kind correction). It is the entrance gate for slice 1. It does not decide space placement, cross-context evidence, or the app redesign."
cites:
  - "ai-stewarded-commons-reimplementation-plan | A Commons That Keeps Its Promises | sha256:77200f7c2cde7b00 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "holons-are-spaces-how-we-use-holochain | Holons, Spaces, and Holochain | sha256:931b9b05cae78c40 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
---

# Accountable correction — the contract (slice 0, revision 2)

Revision 2 (2026-09-06) absorbs the adversarial read (Codex, eight amendments) and the P2P design-gate review (rust-architect, ten findings). Every line names the code it binds. Where the code does not do what the contract requires, the gap is named. Nothing here is claimed as delivered.

**Scope narrowed in revision 2.** Slice 1 covers acceptance and amendment by the **root author** of the target record only. The delegate path (`HeadDelegation`) is out of slice 1 and named as a gap in §5, because the existing head path filters to the root author and a delegation is signed-and-returned, never committed to the DHT.

## 1. The authoritative act and its identity

The act is the witnessed `FeedbackSignal` action in the content cell, created by `create_feedback_signal` (`elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs:96`). Its integrity entry (`content_store_integrity/src/feedback_signal.rs:78-101`) is not changed by this slice.

**Identity = (origin DNA hash, action hash).** This is a deliberate, labelled departure from the "cid = entry_hash" rule: the *signed act* is the entity, and two authors' identical corrections share an entry hash but are two acts. The author is `record.action().author()` of that signed action. `signer_pubkey` in the entry is coordinator-derived (`feedback_signal.rs:131`), not integrity-bound to the action author (`content_store_integrity/src/lib.rs:4305`); the p2p `signed_by` is likewise a claim. Both are checked against the signed action and rejected on mismatch. Storage normalises 39-byte `AgentPubKey` against 32-byte ed25519 representations before comparing. The receiving peer's cell and installation are routing context and never enter the identity.

The origin DNA hash is not carried in the entry. In this slice every peer is in one content space; the origin DNA hash is the receiving storage's own content cell DNA hash (`elohim-storage/src/hc_client.rs:676-682`). An envelope naming a different DNA hash is rejected. **Gap (cross-context):** carrying and fetching from another space is not this slice.

The fetched record is verified before use: returned action hash equals the requested one, entry type is the public `FeedbackSignal` variant (`content_store_integrity/src/lib.rs:3692`), entry hash matches the action's entry hash, and entry bytes are present (a remote fetch may return `Hidden`/`NotStored` — `holochain-conductor/crates/holochain_integrity_types/src/record.rs:19-34`). A record without entry bytes stays pending; a signed header alone is never evidence.

## 2. Address mapping — action hash to content id

`target_cid` in the entry holds a base64 `ActionHash` of the target content record (`feedback_signal.rs:137,143`), while both doc comments still say CIDv1 (`content_store_integrity/src/feedback_signal.rs:80`; `elohim-storage/src/p2p/feedback_signal.rs:120`). This is declared debt with a migration arc: slice 1 documents the field as an action-hash string and does not redefine it; a later coordinator revision may add a separate content-id field additively.

Resolution: action hash → content id via `get_content(action_hash)` (`content_store/src/lib.rs:7061`), which needs entry bytes (`lib.rs:7074-7079`). The root author of the target is resolved from the referenced action's own Update lineage back to its Create (`lib.rs:2818-2830`), never from a content-author string, the current head's author, or `records[0]` of the ID-anchor enumeration (`lib.rs:2888` is not a safe substitute when several roots share an id). Back-propagation's predecessor key is `target_cid` as a string (`services/back_prop.rs:279`); resolving to `Content.id` does not by itself yield that key, so the envelope carries the act reference and the routing key separately (§4).

## 3. Discovery — durable, bounded, unordered, fair

Storage keeps a persisted **subscription set** with two member kinds: content targets this peer stewards, and **discovered correction actions** (acceptance vouches link from the correction action, not the content action — `feedback_signal.rs:277-284`, `LinkTypes::TargetToFeedbackSignal`). Both survive restart.

On an interval (mirror `services/release_adoption/watch.rs:68,1968,2015`) storage enumerates links per subscribed member and joins against a **per-act application table** (§7). Link enumeration is unordered; there is no high-water cursor over DHT history. A late link or a temporarily unfetchable record is picked up on a later tick; the application row records attempts and next-retry with bounded backoff.

**Fairness (not a history cursor):** a persisted rotating position over the subscription set, advanced on failure as well as success, so N members under a per-tick budget of B are each visited within ⌈N/B⌉ ticks. The reference watcher's `take(MAX_CHANNELS_PER_SWEEP)` (`watch.rs:74,1062`) starves the ninth member and is not copied. Eligible retries get service independent of fresh notifications.

**Cost, honestly:** `get_feedback_signals_for_target` fetches every linked record before returning and silently drops fetch failures (`feedback_signal.rs:307-320,343-350`). The required coordinator change (hot-swap, no DNA hash move) is a query that returns **action references plus explicit per-reference fetch outcomes**, and a `get_feedback_signal_record(action_hash) -> Record` getter, so storage fetches only unapplied acts under separate record and byte budgets. Full link enumeration still scales with accumulated history per member; that is stated, not hidden. Discovery also cannot prove global absence: an empty query at a tick means "not observed", never "uncontested" (`get_links` default strategy `feedback_signal.rs:308`; local HDK `crates/hdk/src/link.rs:125-127,144-148` — undeleted creates only, duplicates possible). Verified act references and their dependencies are retained by storage so a later rebuild does not depend on a link that was since deleted (delete-link validation is permissive, `content_store_integrity/src/lib.rs:4261`).

## 4. Notification — an envelope that carries a reference

The direct p2p `feedback-signal` message (`p2p/mod.rs:4976-4984`) gains, **additively**, a notification reference: `#[serde(default)] Option<{origin_dna_hash, action_hash, routing_key}>` alongside the existing semantic payload, with an old↔new decode compatibility test, so mixed-version peers never drop corrections. The receiver (`epr_atom_service.rs:284,457-468`, new branch) rejects a foreign origin DNA, fetches and verifies the record (§1), resolves the target (§2), and hands the **existing semantic type** (`p2p/feedback_signal.rs`) plus the act reference to the projector path of §3. The action signature is over the action, not over the semantic canonical bytes (`p2p/feedback_signal.rs:141-145`); the adapter never presents one as the other. Forwarding hops (`services/back_prop.rs:279-287`) carry the act reference and the resolved routing key with the semantic signal. Per-notification conductor work is bounded before the uncancellable `call_zome`.

## 5. Three acts, never collapsed — and who may perform each in slice 1

1. **Feedback (correction)** — the claim. Any agent in the space may author it; `standing_impact` is a proposal, not an effect.
2. **Acceptance** — `create_vouch` (`feedback_signal.rs:221`) with `signal_kind = vouch`, `vouch_kind = accept-correction`, `target_cid` = the correction's action hash, `evidence_cid = None` (`feedback_signal.rs:259-265` hardcodes it). Integrity admits any vouch by anyone (`content_store_integrity/src/feedback_signal.rs:107-169`); the coordinator only refuses a signer equal to the target's payload signer (`feedback_signal.rs:250-253`). Therefore **acceptance is a storage-verified projection classification, not an integrity guarantee**: every consumer that confers acceptance must verify the vouch record (§1), follow its target to a verified correction, follow that to the exact target content action and its root Create (§2), and require the vouch author to equal that root author. **Same-cell self-acceptance** (root author and correction author are the same cell agent) is refused by the coordinator and is **unsupported in slice 1**: it stays visibly pending, never reported as accepted. **Gap — delegates:** `HeadDelegation` is signed and returned, not committed (`lib.rs:3916-3924,3944-3951`), so no remote consumer can fetch it; and its expiry uses verifier `sys_time()` (`lib.rs:3999-4004`), which makes replay nondeterministic. Delegate acceptance needs a committed, retained proof and a pinned historical-validity rule — a later slice. The cross-root path `authorize_canonical_head_declarer` (`lib.rs:3852`) is an open stub and is not used.
3. **Content successor** — the root author publishes the amended content as an Update through `update_content` (`lib.rs:2494-2512`) and declares it via `declare_content_head` (`lib.rs:5313`). Both filter to the root author (`lib.rs:5345,5377`), which is exactly slice 1's scope. **Gap:** a delegate-authored Update is excluded by those filters, and an existing cross-root canonical answer is preferred by the ordinary resolver (`lib.rs:4147-4179`); both are outside slice 1. A valid FeedbackSignal never moves a head by itself.

## 6. Partition behaviour and conflict

- If a dependency of acceptance (the correction record, the target content, its root lineage) cannot be fetched, the acceptance is **pending** and shown as pending; nothing is projected as accepted. A positive mismatch (wrong author, wrong type) is **rejected**, not retried as a network failure.
- **Conflict in slice 1 is same-root:** two Updates by the root author naming the same predecessor (two devices, or a replayed submission). Storage surfaces it as **contested**, listing both branches with their predecessor, from chain enumeration (`gather_content_chain`, `lib.rs:2888`, scoped to the referenced action's lineage). The deterministic pick is the existing ordinary resolver (newest action timestamp, `lib.rs:4173-4176`); the contested marker is not cleared by that pick. Identical re-declarations of one target are deduplicated and do not count as branches; ordinary sequential amendments (each naming the prior head) are not contested. **Gap:** the tiered `canonical_head` election (`select_canonical_winner`, `lib.rs:3032-3045`, tiebreak on the CreateLink hash) returns one winner plus at most one staging candidate (`lib.rs:3110-3116`, `ContentHeadOutput` `lib.rs:2651-2670`) and does not expose same-tier losers; exposing them is a coordinator change owed before any multi-authority conflict station.
- Action timestamps give stable ordering for replay, not trusted real time; an author-controlled timestamp can dominate the pick. Slice 1 states this trust assumption and uses ancestry, not time, for successor relationships.

## 7. Projection — transactional, deterministic, rebuildable

- New migration `feedback_application(generation_id, origin_dna_hash, action_hash, status, attempts, next_retry_at, applied_at, PRIMARY KEY(generation_id, origin_dna_hash, action_hash))` with the mandatory `-- Source of truth:` comment (class C, rebuilt from the DHT per this section). A **generation** = one (evaluator, pinned policy bytes by CID) projection; per-act application is tracked per generation, so a second evaluator is never suppressed by the first. `standing_view` (`migrations/2026-05-01-040000_standing_view/up.sql:6-13`) stays one aggregate per (evaluator, subject); the per-act contribution is the application row.
- Application row and aggregate change commit in **one diesel transaction** that propagates failure and rolls back together (the syntax example `db/economic_events.rs:806-817` swallows per-item errors; that error handling is not copied). Replay after commit is a no-op by the application row.
- **Contributions:** a correction alone contributes **zero** (an allegation debits nobody — a change from today's immediate debit at `standing_projector.rs:51-60`); an accepted correction contributes per the pinned policy against the target's **root author** as subject (never `signed_by`, today's `standing_projector.rs:91-93`). Repeated acceptances of one correction count once. `score` derives from the checked cumulative sum (`score_for_debit_sum`, `standing_projector.rs:121-130`, permutation-invariant); `debit_weight_sum` is i32 (`db/standing_view.rs:22`) with unchecked add (`standing_projector.rs:99`) — use checked arithmetic and refuse overflow. `last_signal_at` = the **maximum** included action timestamp, not the last enumerated (`standing_projector.rs:109` overwrites). Policy bytes are pinned per generation; `from_registry` (`standing_projector.rs:233-256`) reads the live registry and is not used for replay. The policy input gains vouch subtype and acceptance status (today `standing_projector.rs:28-31` sees kind and impact only).
- **Rebuild** = a fresh generation built to completion then atomically published, or an in-place reset that serialises live discovery/notification writers and clears the generation's application rows and aggregates together. Pending evidence and the subscription set live outside the cleared state. Readers see `rebuilding` until every retained eligible input for the generation has been replayed. Equality = identical canonical logical rows (policy CID, acceptance dependencies, contributions), excluding SQLite layout and operational timestamps. Extension-signal contributions (`standing_projector.rs:187-223`) are either replayable inputs of the generation or excluded from it; they are not silently lost.

## 8. Submission binding — one intended act survives a lost response

No operation id exists today. Slice 1 adds a storage-side outbox `feedback_operations(operation_id PRIMARY KEY, origin_dna_hash, submitting_cell_agent, request_bytes_cid, target_action_hash, evidence_cid, signal_kind, vouch_kind, action_hash NULL, status, created_at)` (class B: local durable intent, not DHT-rebuildable; `-- Source of truth:` comment says so). An operation pins the origin DNA, the **submitting content-cell agent** (`hc_client.rs:681-682`; the zome derives the signer from `agent_initial_pubkey`, `feedback_signal.rs:131,169`) and the immutable request bytes; reuse of an `operation_id` with different request bytes is refused; execution is **single-flight** per operation.

**Durable binding without an integrity change:** for a correction, the client embeds `operation_id` inside the content-addressed Correction EPR it authors as evidence, so `evidence_cid` is unique per operation and the tuple `(submitting_cell_agent, target_action_hash, evidence_cid, signal_kind)` identifies the operation on chain. For an acceptance the tuple is `(submitting_cell_agent, target_action_hash = correction action hash, signal_kind, vouch_kind)`. Recovery after an uncertain call enumerates `list_feedback_signals_by_signer` (`feedback_signal.rs:331-339`) for the pinned cell agent and matches the tuple; if several acts match (concurrent double-mint), the lowest action hash is the operation's act and the others are marked `duplicate_of` in `feedback_application` and contribute nothing. If **zero** match, the operation stays `unresolved`: it is never auto-resubmitted; the user may explicitly resubmit, which reuses the same operation id and therefore the same tuple, so the worst case is a deduplicated second act.

Residuals, stated: (i) two humans behind one storage cell submitting the same evidence CID against the same target are conflated at the cell level — the zome author is the cell agent, not the human (`identity-cross-signed` territory); (ii) `standing_impact` is not in the tuple, so two same-tuple submissions with different impacts collapse to one act; (iii) an SQL outbox plus tuple recovery is not an exactly-once remote commit protocol and does not claim to be.

## 9. Design-gate answers (P2P design gate, 2026-09-06)

| Entity | Class | Identity | Creator / zome / hash effect | Projection |
|---|---|---|---|---|
| Correction `FeedbackSignal` | A (existing entry, `content_store_integrity/src/lib.rs:3692`) | (origin DNA, action hash) — labelled departure from entry_hash, §1 | `create_feedback_signal`, content_store coordinator, no hash move | local post_commit `FeedbackSignalCommitted` (cell-local); remote by discovery §3 |
| Acceptance vouch | A (existing entry, `vouch`/`accept-correction` in `SIGNAL_KINDS`/`VOUCH_KINDS`, `content_store_integrity/src/feedback_signal.rs:47,54`) | same | `create_vouch`, coordinator, no hash move | same |
| `get_feedback_signal_record` + reference/outcome query | n/a | n/a | coordinator only, no hash move | n/a |
| `feedback_application` | C (rebuilt from DHT, §7) | composite (generation, origin DNA, action) | storage migration | n/a |
| `feedback_operations` | B (local durable intent; not DHT-rebuildable) | client UUID — justified: intent precedes the act's existence, and the id is bound on chain via the evidence CID (§8) | storage migration; HTTP route `POST /api/v1/feedback/operations` declared in `http.rs` `build_manifest()` | n/a |
| Notification reference | C | reference | additive `#[serde(default)]` field on the existing p2p message (§4) | n/a |

**Head-plane cost** (plan §4 workload, 20 households, 104 corrections/yr): ≤104 correction heads + ≤104 acceptance heads ≈ 208 A-class heads per community-year, two links each (`TargetToFeedbackSignal`, `SignerToFeedbackSignal`, `feedback_signal.rs:279,290`) ≈ 416 links, ≈208 application rows per generation, ≤208 outbox rows. Linear per community and per subscribed member; measured, not assumed.

**Network stakes:** a correction is `CounterEvidence`-class — it reaches the target's root author and is never filtered out by reach or standing; that cost never cheapens.

**Sync plane:** neither the correction nor the acceptance projects into the Automerge sync plane (`elohim-storage/src/sync/projector.rs`) in slice 1; the standing aggregate is a storage view. Unresolved: whether an accepted successor's head declaration rides the existing sync-plane head path — declared drift.

**Seam registration:** a `seam-registry.yaml` row for the notification reference (contract tests: old↔new decode; foreign-DNA rejection) is owed with the code.

## 10. What this contract does not decide

Delegate acceptance and delegate-authored successors; multi-authority conflict exposure; space placement; cross-context evidence; the app host/view contract beyond one first-party action; the human-to-agent identity binding; fork or replacement of the conductor.
