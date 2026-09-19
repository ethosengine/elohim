---
epr-habit-version: 1
id: authority-in-integrity
invariant: >
  Every entry or link that grants one agent standing over another is authenticated in an
  integrity zome, where every validator runs it. Standing means: a claim that this agent may
  act for, serve, vouch for, hold, or bind another — a stewardship grant, a collective
  Steward role, a portal host, a content server, a node registration, a peer-status claim, an
  REA commitment naming a counterparty. The claim's authenticity — WHO may author it — is a
  deterministic property of the entry bytes plus `action.author()`, checkable with `must_get_*`
  alone, and is therefore refused identically by every peer. A rule enforced only by a
  well-behaved coordinator is not enforced: coordinators are hot-swappable application code,
  so any peer running modified coordinator WASM can mint standing that every validator accepts.
status: unwired
active: false
first_move: >
  Write the red. The runnable check this habit needs does not exist, and prose does not advance
  it (covenant rule 2). Two candidates, in order of cost: (1) a source-shape test in the DNA
  workspace — `no integrity zome dispatches FlatOp::Link(OpLink::CreateLink{..}) to an
  unconditional Ok(Valid), and no standing-conferring entry validator is called without
  &action` — which is deterministic, runs in the existing DNA gate, and fails today in four of
  five DNAs; (2) a sweettest in `elohim/holochain/tests/sweettest/` that commits a
  counterfeit — agent B creating an `AgentToPeerStatus` link on agent A's base, or a
  `StewardshipGrant` naming a steward B never consented to be — through a direct source-chain
  write, and asserts the second conductor REFUSES it (`two_agent_conductors` +
  `exchange_peer_info` + `await_consistency` before the assertion, per
  feedback_sweettest_cross_agent_consistency; `#[ignore]` is a CI no-op here, the DNA suite
  runs `--run-ignored all`). (1) makes the habit red cheaply and is a shape-guard, not a
  behaviour proof; (2) is the behaviour proof and is what the invariant actually claims. Write
  (1) first so the habit is measured, then (2) so it is true.
guard: >
  1. THE COUNTERFEIT CURE. Adding the check to a coordinator pre-commit gate makes the
     symptom disappear and changes nothing this habit claims — a peer running modified
     coordinator WASM is precisely the adversary. `PortalHost` already documents its
     authorship check as living "in the coordinator pre-commit gate"; that is the shape to
     refuse, not to copy.
  2. THE DISPATCH SWALLOWS THE AUTHENTICATOR. `content_store_integrity` and
     `node_registry_integrity` destructure `OpEntry::CreateEntry { app_entry, .. }` and pass
     only the entry to `validate_create_entry`. `action.author()` is discarded at the dispatch
     site, so NO validator below it can authenticate anything, however well written. A new
     validator added under that dispatch inherits the hole silently.
  3. EVERY CURE IS HASH-MOVING. These are integrity-zome edits: they cannot reach a running
     conductor by `update_coordinators` hot-swap. Applied to some peers in a namespace and not
     all, they split one DHT into two. This is why the cures batch into one crossing per DNA
     rather than trickling — and why "it is only a comment" is not a safe edit in these files
     without measuring the hash.
  4. LOOSENING IS BREAKING TOO. `validate_human` accepts three `profile_reach` values where
     `VISIBILITY_LEVELS` declares five; widening it is still a validation change, so old
     validators would refuse data the new ones accept. Reconcile against the canonical reach
     taxonomy, never a fourth local list.
refs:
  - "genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md — the item table: claim, deterministic HDI check, compatibility class, landing order"
  - "elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs `validate_doorway_record` (~:326-434) and `validate_peer_status` (~:443-468) — the in-tree counter-examples: author-binding plus Ed25519 over canonical bytes, deterministic, already shipped"
  - "elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs — KeyRotation quorum verification; unimplemented variants fail CLOSED"
  - "elohim/holochain/dna/imagodei/.epr-meta/custodial-authority-answerable.habit.md — the sibling: that habit asks whether an institutional act can NAME its acting human; this one asks whether the grant behind the act was authenticated at all"
  - "genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md — the hash table, the network-seed ladder, the ALLOW_DNA_REINSTALL partition trap every cure here rides"
  - ".claude/skills/holochain-hdk-0-7/references/membranes.md — why only the AgentValidationPkg op arm enforces a membrane, and genesis_self_check is a courtesy to honest agents"
  - "genesis/data/timeline/backlog/arch-scale-risk-backlog.md rows 1, 2, 5 — the scale shapes on the same integrity path; row 2's cure is hash-moving and rides the Tasks 21-23 crossing"
retire-when: >
  When a standing-conferring entry or link cannot be committed WITHOUT an integrity-level author
  binding, by construction rather than by review: every `#[hdk_link_types]` variant is
  classified authority-bearing or not at the type level and each DNA's link arm matches
  exhaustively over that classification (so a new link type cannot compile without an answer to
  "who may author this"), every standing-conferring entry validator receives `&action` from its
  dispatch, and every cross-entry reference an authority check depends on is a hash type
  (`ActionHash`/`EntryHash`) rather than a `String` a validator cannot follow. At that point
  authentication is a property of the zomes' shape and this habit is describing the compiler,
  not a practice under watch.
---
2026-09-19: DECLARED `unwired` — committed to, with no runnable check, from a whole-DNA audit
that found ONE systemic finding rather than a list of defects. The finding: authority in this
protocol is enforced in COORDINATOR zomes, and coordinators are hot-swappable application code.
Verified at source across all five DNAs this session — `FlatOp::Link(OpLink::CreateLink{..}) =>
Ok(Valid)` unconditionally for EVERY link type in imagodei (~:1175), infrastructure (~:304),
node-registry (~:271) and the elohim content store (~:4260), while a link's base is exactly
where an ownership claim is encoded (`AgentToPeerStatus`, `AgentToContentServer`,
`AgentToPeerBinding`, `StewardToGrant`/`SubjectToGrant`, `MemberOf`/`HasMembership`,
`PortalHosts`); `StewardshipGrant` validated for field shape only, never against
`action.author()`; qahal `Membership{role: Steward}` requiring only `sponsor_cid.is_some()`
while the invite signature, expiry and replay checks live in the coordinator's
`affirm_membership`; `PortalHost` authorship documented as living in the coordinator pre-commit
gate; `ContentServer` carrying no author-binding field at all; node-registry validating only
`ShardAssignment` of six entry types, its three `signature: String` fields ("Prevents spoofing")
never verified — the only `verify_signature` call sits behind the OFF-by-default
`lineage-witness` feature; `Agreement`, `Commitment` and `EconomicResource` — named in
`elohim/holochain/dna/CLAUDE.md` as MUST-notarize because centralizing them makes someone the
bank — with no validation arm among 75 entry types; `genesis_self_check` an unconditional
`Valid` stub in all four DNAs that declare it, with no `AgentValidationPkg` arm anywhere, so no
membrane is enforced against a hostile joiner.

Nothing here is measured, so nothing here is claimed as red. The audit is a read of source, and
a read of source is not a probe: it says the check is absent, not that a counterfeit lands. That
distinction is exactly what `unwired` is for, and writing the probe is the only legal next move.

Why it is worth a declaration rather than a backlog row alone: trust in this protocol is a
performance primitive as much as a security one. The compute/trust gradient only pays — a
high-trust peer edge served fast, a commons read served witnessed-safe — if provenance is what
makes the gradient EARNED rather than configured. Authority that only a well-behaved coordinator
enforces is configured trust, and nothing downstream may safely price it cheaper. Integrity-level
enforcement is what turns a grant into a witnessed trust act the fast lane is allowed to lean on.
