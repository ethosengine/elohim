---
id: "backlog-arch-authority-in-integrity-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Authority-in-integrity cluster — every claim of standing over another agent is enforced in a hot-swappable coordinator, not in the integrity zome every validator runs"
slug: "arch-authority-in-integrity-backlog"
written: "2026-09-19"
author: "rust-architect (whole-DNA authority audit, operator-dispatched 2026-09-19)"
status: "backlog"
priority: "high"
area: "protocol/dna-integrity"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:authority-in-integrity"
  - "habit:custodial-authority-answerable"
  - "habit:notary-authority"
  - "habit:happ-lineage-migration"
cites:
  - "dna-upgrade-governance | the hash table, seed ladder, lineage and ALLOW_DNA_REINSTALL policy every cure here rides | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md"
  - "holochain-evolution-epic | Tasks 21-23 sunset-hardening crossing — the hash move rows 10-11 must ride rather than duplicate | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "holons-are-spaces-how-we-use-holochain | where the genesis_self_check stub is already recorded | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - elohim/holochain/dna/CLAUDE.md
  - elohim/holochain/dna/.epr-meta/authority-in-integrity.habit.md
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
tags: [security, holochain, dna, integrity-zome, authority, trust, hash-moving, p2p-design-gate, risk, decision, constraint]
---

# Authority-in-integrity cluster

One finding, many faces. Across five DNAs, the rules that decide **who may claim standing over
whom** live in coordinator zomes. Coordinator zomes are hot-swappable application code — they sit
outside the DNA hash by design, and the conductor's `update_coordinators` path exists precisely so
they can be replaced without a network event. A peer running modified coordinator WASM therefore
authors whatever standing it likes, and every validating authority in the network accepts it,
because the integrity zome asks nothing.

This is not a list of nine bugs. It is one architectural placement error with nine visible
symptoms, and it is filed as a cluster so the cures batch into the minimum number of DNA-hash
moves rather than trickling one crossing at a time.

## Why this is worth a crossing: trust is a performance primitive

Trust in this protocol is meant to be earned, and an earned gradient is what lets a high-trust peer
edge be served fast while a commons read stays witnessed-safe. **Provenance is what makes the
gradient earned rather than configured.** Authority that only a well-behaved coordinator enforces is
configured trust: it is a convention among the polite, and nothing downstream may safely price it
cheaper, because the cheap path would be leaning on an assumption no validator checks.
Integrity-level enforcement is what turns a grant into a witnessed trust act the fast lane is
allowed to lean on. That is the whole argument, and it is the reason this is a protocol concern
rather than a hardening chore.

The framing to hold while designing the cures: authority here is **relational capability**, not
rank, and not crypto self-sovereignty. A key is not a person. The question an integrity validator
can answer is narrow and honest — *did the agent who signed this action have the standing this
entry claims for them?* — and the question it cannot answer (is this the right person, is this
grant wise, should it be revoked) belongs to Mishpat and to the elohim ceiling above the substrate.
Do not smuggle discernment into a validator. Do not leave authentication out of one.

## What HDI 0.8 actually gives a validator

Every cure below is written against this list, and where a check cannot be built from it, the item
says so instead of pretending.

| Available in an integrity validator | Not available |
|---|---|
| `action.author()`, `action.timestamp()`, `prev_action` | `get()` / `get_links()` — any non-deterministic read |
| the entry's own bytes (all fields) | cross-DNA `call()` — no imagodei lookup from elohim |
| `must_get_entry(EntryHash)`, `must_get_action(ActionHash)`, `must_get_valid_record(ActionHash)` | wall-clock, randomness, network state |
| `must_get_agent_activity(agent, ChainFilter)` — one agent's chain in THIS DNA | anything a second validator could disagree with |
| `verify_signature(key, sig, bytes)` — Ed25519 over canonical bytes | |
| `dna_info()?.modifiers.properties` — e.g. `progenitor_pubkey` | |

Two consequences shape every item:

1. **A reference a validator must follow has to be a hash type.** `StewardshipGrant.delegated_from`
   is `Option<String>`; `Commitment.provider`/`receiver` are `String`. A validator cannot resolve a
   string into a record, so any authority rule that depends on "who authored the thing this names"
   is unbuildable until the field is `ActionHash`/`EntryHash`. That is why several rows are
   shape-breaking rather than pure-logic.
2. **Authorship binding beats self-signature.** Three node-registry entries carry a
   `signature: String` field commented "Prevents spoofing". It prevents nothing: the action is
   *already* signed by its author, so a self-signature adds no information a validator did not
   have. The real check is `claimed_identity == action.author()`. Reach for `verify_signature` only
   for a *third* party's assertion (an invite issued by a sponsor, a doorway's operator key) — which
   is exactly what the in-tree counter-examples do.

## The counter-examples are in-tree, so the pattern is proven achievable

Nothing below needs invention. `infrastructure_integrity::validate_doorway_record`
(~:326-434) binds both `operator_agent` and `signing_key` to `action.author()` and then verifies an
Ed25519 signature over canonical bytes. `validate_peer_status` (~:443-468) asserts
`peer_id == action.author()` and bounds the timestamp against `action.timestamp()`. imagodei's
`recovery_v2.rs` `KeyRotation` verifies a real quorum and makes unimplemented variants fail CLOSED.
Three deterministic, shipped patterns; the rest of the tree simply did not adopt them.

## Item table

Compatibility classes, used in the last column:

- **A — logic-only.** No entry shape changes. Existing well-authored data still validates; only
  counterfeits are refused. Hash moves (all validator changes do), migration is nothing.
- **B — shape-additive.** A new `#[serde(default)]` field. Old bytes still deserialize; hash moves;
  migration is mechanical.
- **C — shape-breaking.** A field's type changes or is removed. Old bytes do not deserialize; needs
  an authored transform, or a reseed under an `_alpha` seed.
- **D — membrane.** Changes who may join the network at all. Not a data-compat question.

| # | Claim left unauthenticated | Deterministic integrity-level check | Compat | DNA | Order |
|---|---|---|---|---|---|
| 1 | **Any agent may create any link on any base.** A link's base is where ownership is encoded — `AgentToPeerStatus`, `AgentToContentServer`, `AgentToPeerBinding`, `StewardToGrant`/`SubjectToGrant`, `MemberOf`/`HasMembership`, `PortalHosts`. Every link type passes unconditionally. | Match on `link_type` in `OpLink::CreateLink { base_address, target_address, link_type, action }`. For an agent-keyed base: `AgentPubKey::try_from(base_address)? == *action.author()`. For an entry-keyed base whose authority derives from the target: `must_get_valid_record(target)?.action().author() == action.author()`. `OpLink::DeleteLink` carries the original `CreateLink` action — require `action.author() == original_action.author`. Make the match **exhaustive** over `LinkTypes` so a new variant cannot compile without an answer. | A | imagodei, infrastructure, node-registry, elohim | 1 |
| 2 | **`PortalHost` authorship.** "This human's portal is hosted here" — the validator already calls `must_get_valid_record(human_action_hash)` but discards the record; the authorship check is documented as living in the coordinator pre-commit gate. | Keep the record and compare: `must_get_valid_record(h)?.action().author() == *action.author()`. Two lines. This is the cheapest cure in the cluster and the template for the rest. | A | imagodei | 1 |
| 3 | **`StewardshipGrant` — who asserted this grant.** Validated for field shape only (non-empty ids, `authority_basis` membership, depth ≤ 3); `action.author()` is never consulted. Anyone may author a grant naming anyone as steward of anyone. | The grant's author must be the subject's own agent (consented stewardship) or the delegating steward: resolve `delegated_from` with `must_get_valid_record` and require its author to equal `action.author()`, with `delegation_depth` equal to the parent's plus one (checked against the parent's bytes, not asserted). **Blocked on shape:** `delegated_from: Option<String>` cannot be resolved — it must become `Option<ActionHash>`. Binding the *subject* requires an `AgentPubKey` on the entry (`subject_agent`), because `subject_id: String` is not followable either. | C (both fields) | imagodei | 2 |
| 4 | **qahal `Membership{role: Steward}`.** Integrity requires only `sponsor_cid.is_some()`; the invite signature, expiry and replay checks live in the coordinator's `affirm_membership`. The founder path writes a synthetic `sponsor_cid = "founder"` that satisfies the gate with a string literal. | Carry the sponsor's `Signature` over canonical bytes (`collective_cid ‖ member_cid ‖ role ‖ expires_at`) plus the sponsor's `AgentPubKey`; `verify_signature(sponsor_key, sig, bytes)` and bound `expires_at` against `action.timestamp()`. Require `member_agent == action.author()` so a third party cannot replay someone else's invite. **Founder:** replace the string literal with `must_get_entry(collective_entry_hash)` + require the Collective's author to equal `action.author()` — a founder is provably the agent who created the Collective, not an agent who typed a word. | B (sponsor key + signature) / C (founder path) | imagodei | 2 |
| 5 | **`ContentServer` has no author-binding field at all**, and `get_latest_peer_status_for_agent` trusts whatever an `AgentToPeerStatus` link resolves to. | Add `operator_agent: AgentPubKey` and require it to equal `action.author()` — the `DoorwayRegistration` pattern, one DNA over, minus the Ed25519 leg (no third party asserts here). The link half is item 1. | B | infrastructure | 1 |
| 6 | **node-registry validates one entry type of six.** `NodeRegistration`, `NodeHeartbeat`, `HealthAttestation` and `CustodianAssignment` fall through a TODO to `Valid`; their `signature: String` ("Prevents spoofing") is never verified anywhere, and the only `verify_signature` in the crate sits behind the OFF-by-default `lineage-witness` feature. | Bind identity, drop the theatre: `NodeRegistration` gains `agent: AgentPubKey` checked against `action.author()`; `NodeHeartbeat` and `HealthAttestation` reference their registration by `ActionHash` and require `must_get_valid_record(reg)?.action().author() == action.author()` (a node reports its own health; an attester attests as itself). Delete the three `signature: String` fields — an action is already signed. **`CustodianAssignment` is not answerable here**: "who may assign custody of this content to that node" is a policy question, not an authorship question, and integrity can only enforce *self*-assignment (a node accepting custody) or an assignment signed by a named authority. See operator decision D4. | C (hash-typed refs, field removal) | node-registry | 3 |
| 7 | **The dispatch swallows the authenticator.** `content_store_integrity` and `node_registry_integrity` destructure `OpEntry::CreateEntry { app_entry, .. }` and call `validate_create_entry(&app_entry)`. `action` is discarded at the dispatch site, so no validator beneath it can authenticate anything, however well written — and any validator added later inherits the hole silently. | Change the dispatch to `{ app_entry, action }` and thread `&action` through `validate_create_entry`. Mechanical, touches every existing arm (10 in content_store, 1 in node-registry), and is the precondition for item 8. imagodei and infrastructure already bind `action` at their dispatch sites — only these two throw it away. | A | elohim, node-registry | 1 (precondition) |
| 8 | **REA primitives have no validation arm.** 10 of 75 entry types in `content_store_integrity` are validated. `Agreement`, `Commitment` and `EconomicResource` — the three `elohim/holochain/dna/CLAUDE.md` names as MUST-notarize *because centralizing them makes someone the bank* — have none. | `Commitment`: the author must be its `provider` or `receiver` — an agent cannot commit two strangers to each other. Requires `provider`/`receiver` to be resolvable (an `AgentPubKey` field beside the display string; see D2). Plus `action` ∈ the REA verb set and `resource_classified_as_json` ⊆ the whitelist — both pure-data checks buildable today. `EconomicEvent`: `bounded_by` names a commitment by `EntryHash`; `must_get_entry(bounded_by)`, decode, and require the event's provider to match the commitment's — the bounds-gate becomes an integrity invariant instead of a storage convention. `Agreement` is genuinely thin (id/name/note/created_at) and carries no standing: validate shape, and say so rather than inventing a rule. | A (verb/whitelist, `bounded_by`) + B (agent keys) | elohim | 4 |
| 9 | **No membrane anywhere.** `genesis_self_check` is an unconditional `Valid` stub in all four DNAs that declare it, and no DNA has an `AgentValidationPkg` arm — which, per the skill's membranes reference, is the *only* place a membrane is enforced against a hostile joiner. Anyone who can reach the network joins it and writes to it. | An `AgentValidationPkg` arm verifying an invite signed by `dna_info()?.modifiers.properties.progenitor_pubkey`, with `issued_to == action.author()`; the same check mirrored into `genesis_self_check` so an honest agent fails fast and locally. | D | **operator decision D5** — recommended for infrastructure + node-registry only | 5 |

### Riders — same hash move, filed here so they are not a second crossing

| # | Item | Note |
|---|---|---|
| 10 | `refuse_carried_after_close` walks the carrier's whole v2 activity per witness (`must_get_agent_activity` from `prev_action` back to genesis; O(W²) per carrier, re-run by every validating authority). Proposed cure: a `prev_witness_action_hash` field + `ChainFilter::new(prev).until_hash(prev_witness)` — **never `.take(n)`**, which bounds by count rather than by content and can silently step over a close, making two validators disagree. | **CROSS-REFERENCE, not absorbed.** This is `arch-scale-risk-backlog` row 2 → epic gap G9 → Task 25, and it rides the Holochain Evolution Epic's Tasks 21-23 sunset-hardening crossing. It is listed here only so that node-registry's authority crossing (item 6) is *sequenced with* that one rather than minting a second hash move on the same DNA. Do not re-plan it here. |
| 11 | Stale comment at `content_store_integrity/src/lib.rs` ~:1358-1365 (and the `post_commit_signal_dispatch_tests` module doc at ~:4620) still describes `post_commit` as an ordered `to_app_option` first-match-wins dispatch. The coordinator is now header-driven (`resolve_entry_type`, guarded by `post_commit_dispatch_tests`). The `#[serde(deny_unknown_fields)]` the comment justifies is still load-bearing and must stay; only the reasoning is out of date. | Comment-only. Fold into the elohim crossing (item 8) so the hash question is moot — see §"Is `#[cfg(test)]` really hash-neutral" below for why a comment edit in an integrity crate is not automatically free. |
| 12 | `validate_human` hardcodes `["public", "community", "private"]` for `profile_reach` while `VISIBILITY_LEVELS` (same file, ~:54) declares five (`public`, `community`, `unlisted`, `connections`, `private`) and `validate_agent` uses the constant. | Use the constant. Note this is a **loosening**, which is still a breaking validation change: old validators would refuse `unlisted` data the new ones accept, so it cannot be back-deployed piecemeal. Reconcile against the canonical reach taxonomy, never a fourth local list. |
| 13 | A `#[cfg(test)]` link-type / entry-type headroom assertion (content_store is at 225 of the 256 `u8` link-type cap). | Wanted as a guard-rail; see §"Is `#[cfg(test)]` really hash-neutral" for the answer and the one construction trap. |
| 14 | **A withdrawn Steward keeps steward standing** (read from source 2026-09-19, not executed). `require_caller_is_steward_of` (imagodei `qahal_coordinator.rs` ~:554) tests `withdrawn_at_block_height.is_none()` on records from `list_memberships_for_collective`, which resolves each membership with a plain `get()` on the ORIGINAL Create action. `withdraw_membership_clean` appends an Update and never rewrites the Create, so the Create reads "not withdrawn" forever: a Steward who has left can still issue household invites and sign agreements. The sweettest `qahal_household_query_perf_test` scenario (e) re-joins through exactly this path, so it must be revisited when this row lands. | The coordinator half (follow the update chain, latest state wins) is a hot-swap — and is **unsafe alone**: imagodei's update arm is `OpEntry::UpdateEntry { app_entry, .. }` (~:1154), `action` discarded, so nothing checks WHO authors a Membership update. Latest-wins without that check converts "a withdrawn steward keeps power" into "anyone can revoke a steward". Order is therefore fixed: the integrity rule first (a Membership update is valid only from the original author; rides the imagodei crossing with rows 1-4 and is the update-side twin of row 7), the coordinator read second. The own-chain reader `get_my_household_collective_cids` already applies latest-state-wins safely — a foreign Update cannot land on the caller's own source chain. |

## The landing design

### One hash move per DNA; the DNAs do not have to move together

Each DNA is its own network. A hash move on imagodei does not touch the elohim content DHT. So the
crossings are **independent and should be sequenced by blast radius**, not batched into one
big-bang. What must be atomic is *within* a DNA: every item for one DNA lands in one crossing,
because a second crossing is a second reinstall and a second re-key.

| Step | DNA | Items | Why here |
|---|---|---|---|
| 1 | **infrastructure** | 1, 5 | Smallest surface (4 entry types, 10 link types), already holds both counter-examples, and its re-key costs least — it is federation-native and declares no `progenitor_pubkey`. Proving ground for the link-arm pattern. |
| 2 | **imagodei** | 1, 2, 3, 4 | The densest concentration of standing claims, and the highest re-key cost: a reinstall mints new agent keys, which on the identity DNA means every human's chain restarts. Land it with a household recast, never as a quiet redeploy. |
| 3 | **node-registry** | 1, 6 — **sequenced with Tasks 21-23 / Task 25** | Its authority cure and its scale cure (rider 10) are both integrity-side. Two crossings on one DNA for one reason is waste; ride the sunset-hardening crossing. |
| 4 | **elohim (lamad)** | 7, 8, 11 | Largest corpus, longest quiesce, and item 7's dispatch change touches all 10 existing arms. Do it last, on the evidence of steps 1-3. Item 7 is a *precondition* for item 8 and can be landed in the same crossing. |
| 5 | **mishpat** | 1 (link arm completeness) | Already validates `CommitmentByState` tags — the only DNA with a real link arm. Smallest delta: make the match exhaustive rather than `_ => Valid`. |

### Seed-ladder step

Every DNA currently sits on `elohim_<dna>_alpha`, whose contract is *"WILL be reset on any breaking
change. Not production."* Class-A and class-B items are absorbed by reset/reseed at this rung; class-C
items (3, 4-founder, 6) would need an authored transform at `_beta` and do not at `_alpha`. **The
whole cluster is therefore cheapest now and gets monotonically more expensive**, which is the
strongest scheduling argument in this document: the shape-breaking rows in particular are close to
free today and require migration authoring forever after. If the intent is to promote any DNA to
`_beta`, this cluster lands first or not at all.

Per crossing: bump the seed (`elohim_<dna>_alpha` → `_alpha2`) so the old hash stays discoverable,
since `lineage:` remains gated behind `unstable-migration` and cannot be declared.

### The partition trap

A DNA-content change does **not** reach running conductors on a normal edge redeploy — the conductor
data dir is a persistent PVC and the install stale-check is role-structure-only, so a new hash reads
as "not stale". Forcing it needs `ALLOW_DNA_REINSTALL`, and reinstall mints a new agent key.
**Applied to some peers in a namespace and not all, the namespace lands on two DNA hashes — two
DHTs — a silent P2P partition.** The alpha genesis pair must both carry the flag. This is
per-crossing, not once for the cluster.

Corollary worth stating because it inverts the usual instinct: these are the changes for which
"deploy it to one peer and watch" is the *most* dangerous strategy available.

### The story that proves it

Story-first, and it does not exist yet:
`genesis/a2o/features/trust/counterfeit-standing-is-refused.feature`, sibling of
`trust-legibility-atlas.feature`. What it must assert, as the person experiences it — a peer whose
software has been modified cannot hand itself standing, and the household notices rather than
absorbing it:

1. A peer authors, by direct source-chain write, an `AgentToPeerStatus` link on **another** agent's
   base. A second peer refuses the op; the claiming peer does not appear in the first peer's peer
   status. *(item 1)*
2. A peer authors a `StewardshipGrant` naming itself steward of a human who never consented. Every
   other validator refuses it, and the subject's stewardship set is unchanged. *(item 3)*
3. A peer authors a `Membership{role: Steward}` with a `sponsor_cid` it invented. Refused; the
   collective's steward roster is unchanged. *(item 4)*
4. A peer registers a `ContentServer` claiming to serve on another agent's behalf. Refused; content
   routing never offers it. *(item 5)*
5. A node authors a `HealthAttestation` signed as a node it is not. Refused. *(item 6)*
6. **The negative control, which is the scenario most likely to be skipped and most necessary:**
   every one of the above, authored *honestly*, still validates and still lands. A validation change
   that refuses legitimate traffic is a worse outcome than the hole.

Below the story, the two runnable checks the habit's `first_move` names: a source-shape test in the
DNA workspace (no unconditional `Ok(Valid)` link arm; no standing-conferring validator called
without `&action`) which fails today and makes the habit measurable cheaply, and a sweettest per
crossing that commits the counterfeit and asserts a second conductor refuses it. Cross-agent
sweettests need explicit `exchange_peer_info` + `await_consistency` before any assertion, and
`#[ignore]` is a CI no-op here — the DNA suite runs `--run-ignored all`.

Concern-tag namespace reserved for these, entering the register only when the habit flips to red:
`authority-in-integrity-01-link-base` … `-06-negative-control`.

## Is a `#[cfg(test)]` headroom assertion really DNA-hash-neutral?

**Answered from how the hash is computed, and the answer is "semantically yes, byte-wise only under
one condition, and the assertion has a construction trap that is the real risk."**

The DNA hash is taken over the `DnaDef` — the integrity zomes' compiled WASM plus the modifiers.
`elohim/holochain/dna/.epr-meta` and the upgrade-governance seed state the operative rule the same
way: *any recompile of integrity WASM with changed code is a new DNA hash.* So the question is not
"is this test code?" but "do the WASM bytes change?"

1. **`#[cfg(test)]` code is not in the WASM.** The zomes are built with
   `cargo build --release --target wasm32-unknown-unknown`; `cfg(test)` is not set for that profile,
   so the module is not compiled. Semantically neutral, unconditionally.
2. **Dev-dependencies do not leak in.** All five DNA workspaces declare `resolver = "2"`, which
   keeps dev-dependency feature unification out of a non-test build. Under resolver 1 this would not
   hold.
3. **The residual risk is line-number drift, and it is real.** Release WASM retains
   `core::panic::Location` strings (file and line) for `#[track_caller]` panic sites. That is the
   same mechanism behind the already-recorded finding that *the DNA hash depends on the absolute
   build path* — path and position strings are in the artifact. So inserting a `#[cfg(test)]` module
   **mid-file** shifts the line numbers of every panic site below it and can move the hash for a
   change that compiles to nothing. Appending it at end-of-file does not. This matches the existing
   convention without anyone having written it down: every `#[cfg(test)]` module in these integrity
   crates sits at the end of its file (content_store `lib.rs` at :4452 and :4618 of 4816 lines;
   infrastructure at :552 of 618; mishpat at :905). Today content_store's only `unwrap`/`expect`
   sites are themselves inside those test modules, so the current exposure is via dependency code
   only — but that is a property of today's source, not a rule.
4. **The construction trap, which is the part worth the warning.** The assertion is cheap; making
   the count *available* is not. A `#[derive(EnumCount)]` or a `strum`/`variant_count` dependency
   added to reach `LinkTypes::COUNT` is a **non-test** derive on a **hashed** enum — that moves the
   hash for a test. Build the assertion from what already exists: inside the test module, cast the
   last declared variant's discriminant (`LinkTypes::GovernanceActionChild as u8`) and assert it
   against a hand-maintained expected value. No derive, no dependency, no new import in the hashed
   path.

**Verdict:** a headroom assertion appended at end-of-file, using only a discriminant cast, is
expected hash-neutral — and "expected" is the correct word. Prove it the one cheap way available:
`hc dna pack workdir/ && hc dna hash workdir/<name>.dna` before and after, in the *same* directory
(the hash is path-dependent, so a comparison across worktrees proves nothing). Same procedure and
same caveat for the comment-only edit in rider 11 — which is why the safe route is to land it inside
the elohim crossing, where the hash moves anyway.

## Operator decisions

Each has a recommendation. None should be taken by an implementing agent.

**D1 — Is this cluster scheduled before any `_alpha` → `_beta` promotion?**
*Recommendation: yes, and treat it as the gating condition.* Class-C rows (3, 4-founder, 6) are
absorbed by reseed at `_alpha` and require authored transforms at `_beta`, forever. This is the only
row in this document with a closing window.

**D2 — Do REA `Commitment.provider`/`receiver` gain agent-key fields?**
*Recommendation: yes, additively — keep the display `String`, add `provider_agent`/`receiver_agent:
Option<AgentPubKey>` with `#[serde(default)]`, and validate the binding when present.* This keeps the
row class-B instead of class-C, and lets the check tighten from "when present" to "required" in a
later crossing without a second shape change. The alternative — retyping the existing fields — is a
class-C break on the largest corpus for no additional guarantee.

**D3 — `StewardshipGrant`: who is the legitimate author?**
The subject (consented self-grant), the delegating steward (verified via `delegated_from`), or
either? *Recommendation: either, with the distinction recorded on the entry.* A grant a subject
cannot initiate is paternalistic; a grant a steward cannot delegate breaks the depth-3 chain the
entry already models. This is the row where the custodial-authority-answerable habit and this one
meet: the *answerability* of an institutional grant is that habit's question, and the cure here must
not foreclose it. Design them together.

**D4 — `CustodianAssignment`: what is the authority model?**
Integrity can enforce self-assignment (a node accepting custody it authored) or an assignment signed
by a named authority; it cannot enforce a placement policy. *Recommendation: self-assignment plus an
optional signed placement directive* — the node's acceptance is the authenticated act, and the
placement decision stays in the substrate's placement strategy where it belongs. Do not try to
express diversity-aware placement as a validation rule.

**D5 — Is there a membrane, and on which DNAs?**
*Recommendation: infrastructure and node-registry only; explicitly none on the elohim content DNA.*
Who may claim to *be infrastructure* is a bounded, capture-relevant question with a natural issuer.
Who may *read and author content* is not: an invite-gated commons is a capture vector of exactly the
kind the doorway's thin-projection posture exists to avoid, and gating participation on a progenitor
key would make the founding key an apex the protocol says it does not have. Two hard preconditions
if adopted: `progenitor_pubkey` is still the `~` placeholder in `happ.yaml` (a membrane shipped
against an unset key refuses every join, including ours), and the invite-issuance path must exist
before the refusal does.

**D6 — Does item 1's link arm become exhaustive (no `_ =>` wildcard)?**
*Recommendation: yes, and this is the item with the longest-lived value.* An exhaustive match makes
"who may author this link" a question the compiler asks at every new link type, forever — 225 of 256
in content_store, so the answer will be asked often. It is also the largest single diff in the
cluster, and the one most likely to be trimmed under time pressure. Trimming it converts a permanent
guarantee into a one-time sweep.

## Cluster registration

Registered in `CLUSTERS.md` (row `arch-authority-in-integrity-backlog`) in the same pass, per
`backlog/.epr-meta`'s cluster-first rule.

