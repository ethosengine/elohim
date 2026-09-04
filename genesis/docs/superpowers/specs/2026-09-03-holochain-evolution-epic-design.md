---
title: "Holochain Evolution Epic — a hApp version crossing carried by the network itself, notarizations intact, the crossing held by the elohim"
id: holochain-evolution-epic
status: Active
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
serves:
  - happ-lineage-migration
  - runtime-upgrade-propagation
graduation-trigger: graduated Draft→Active 2026-09-05 on the operator's acceptance of §4's posture (elohim-held crossing, no per-node consent, sunset irreversible; recorded in §11.3). Graduates Active→Canonical when the household-mesh a2o receipt for @concern:happ-lineage-migration passes Stations 1–10 on 3 peers with the node_registry rehearsal AND the Lane-B spike (§9) has a measured verdict recorded on this spec
created: 2026-09-03
domain: D2
topic: [dna-lineage, happ-migration, notarization, chain-switch, chain-continuation, conductor-fork, mishpat, release-channel, revert, bridge]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md ("the companion" — §1 two-conductor covenant, §2 paired revert, §7 the bridge as reconciliation; this spec is the first mechanism under its §11 "vision, no mechanism" line)
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md (hash mechanics; §6 export seam shipped / import unwired; §8 open questions 1, 2, 4 answered here)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (rung 5 — the channel, manifest, verify, vehicle registry and receipt chain this spec extends across the DNA line it fenced)
  - genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md (the epic's backlog home; its DoD is this spec's §10)
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (the velocity ladder; row 6 "deliberately last" — this epic)
  - elohim/holochain/dna/NETWORK_UPGRADES.md (stewarded coordination — the migration-window flow this spec makes executable)
  - elohim/holochain-conductor/docs/design/dna_migration.md + docs/design/dna_migration_chain_continuation.md (upstream drafts; the chain-switch worked example is Lane A's kernel, chain continuation is Lane B's)
  - elohim/holochain-conductor/crates/holochain/tests/tests/migration.rs (upstream's shipped chain-switch test — MigrationRecord validated by verify_signature against DNA-properties signers)
cites:
  - "upgrade-revert-and-constitutional-consensus | Upgrade, Revert, and Constitutional Consensus | sha256:4673f9958d96b617 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md"
  - "dna-upgrade-governance | DNA Upgrade Governance | sha256:48b79bbffd184d89 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md"
  - "runtime-artifacts-elected-content | Runtime Artifacts as Elected Content | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
  - genesis/a2o/features/delivery/happ-lineage-migration.feature
  - elohim/holochain/.epr-meta/happ-lineage-migration.habit.md
  - elohim/holochain/dna/NETWORK_UPGRADES.md
  - elohim/holochain/dna/node-registry/dna.yaml
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/elohim-storage/src/services/release_adoption/verify.rs
  - elohim/elohim-storage/src/services/release_adoption/apply.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/hc_client.rs
  - elohim/holochain-conductor/crates/holochain/tests/tests/migration.rs
  - elohim/rakia/schemas/v1/release-manifest.schema.json
---

# Holochain Evolution Epic

*Spec-level epic (not manifesto-tier): the network evolves its own rules and carries every witnessed fact across. Its first runnable concern is `@concern:happ-lineage-migration`; the fork lane (§9) and the constitutional coupling (§4.1) are the epic's further concerns.*

**One sentence:** a hApp version crossing is a release on the same channel rung 5 already
elects, refused at verify unless the manifest declares what it migrates FROM and the elohim's
migration commitment for that path is already notarized; adopted automatically by installing v2
beside v1 under the SAME agent key; carried record by record with the v1 notarization (action + signature) embedded and
re-verified by v2's own validation; bridged both ways by dual-cell peers for as long as the
window is open; reverted for free until the elohim notarize the sunset that closes the v1
chains — and that sunset is the only irreversible act, so it is the last one. Nothing in it is a
per-node or per-household consent: the network is a commons and its core protocol is stewarded by
the elohim; what humans get is diversity — branches and bespoke communities — and a COMMON LANGUAGE
(lineage · witness · bridge map · migration commitment) in which any reconciliation chain between
branches can be built, and walked only along a path that was notarized first.

**What this is NOT.** Not "the epic" — the ladder's row 6 named the class, the epic is what it became. Not the conductor-line change (0.6→0.7 was a wipe; its three breaks are
recorded in the backlog home and stay out of scope here). Not binaries, packages or config —
rung 5 and the ark carry those. This spec is one problem: **a v2 hApp holds the same witnessed
facts as v1, not fresh assertions of them.**

## 0. The operator's course-set (2026-09-03)

> We just cleared data on 0.6 to upgrade to 0.7 and that is not going to be an available option
> in production. We should be able to prove the technology with one simple change of a DNA and
> upgrading it, without redeploying or reseeding the whole holochain from the outside. It has to
> be an internal revert and upgrade between hApps, subject to the protocol governance within the
> app, while maintaining a bridge of communication. Everything else — binaries, packages — others
> have solved; it is specifically an upgrade between hApp versions without wiping out the
> notarization integrity of the network that we need to figure out. Don't rule out forking
> holochain and building our own mechanism.

And the correction that followed (same day), which §4 is written from:

> The network itself is a natural power, a true commons. Upgrades to the core protocol are not a
> "consent" act — that decision is held by the elohim within the closed system — with the
> affordance of diversity within different branches and bespoke communities, so that there is a
> common language with which to build whatever upgrade or reconciliation chain is needed for humans
> within the network to move through the diversity of the network, along a path that has been
> notarized before.

And the crux, named last:

> How do we cryptographically couple upgrades to the internal bar for evolution within the network,
> so we still maintain agent-centric compute — take the self-sovereignty of Holochain and couple it
> to the sovereignty that can only be afforded a true, protected, guarded, moderated commons of
> wisdom. If anything is "godlike" power in the protocol it's this. Maybe it suggests blockchain
> coupling.

Three planes fall out of those sentences, and each has a home below: **notarization** (§2–§3),
**authority, the common language, and the cryptographic coupling** (§4, §4.1), **bridge** (§5, §5.1 across many versions and branches).
§9 is the fork lane.

## 1. Ground truth this spec stands on (grounded 2026-09-03, four readers)

| Fact | Where | Consequence |
|---|---|---|
| An action's signature and hash are over the msgpack of the whole `Action`; **no DNA hash is in the preimage** except the genesis `Dna` action | `holochain_keystore/src/action_ext.rs:32-40`, `holochain_integrity_types/src/action.rs:665-678` | a v1 `Create` signature verifies under v2 with no cross-DNA context |
| An entry's hash is over the entry bytes alone — no DNA, no action, no agent | `holochain_integrity_types/src/entry.rs:156-178` | byte-identical content keeps its EntryHash (and our CID) across the line; only ActionHashes move |
| `verify_signature`, `dna_info()` and `hash_action` are all callable inside `validate` | `ribosome/guest_callback/validate.rs:54-62`, `host_fn/verify_signature.rs`, `host_fn/dna_info_2.rs`, `hdi/src/hash.rs:217` (pure guest) | v2's integrity zome can re-verify a v1 notarization |
| `must_get_*` is same-DNA only — no DNA parameter, resolves through the calling cell's stores | `host_fn/must_get_valid_record.rs:22-70` | a v1 proof must be **embedded**, never referenced (Lane A); a fork could lift this (Lane B) |
| `close_chain`/`open_chain` + `MigrationTarget::{Dna,Agent}` + `InitProperties` are shipped and unconditional; only `lineage:` and `GetCompatibleCells` sit behind `unstable-migration` (not in our build) | `hdk/src/migrate.rs`, `sys_validation_workflow.rs:1467` (`ActionAfterChainClose`), `holochain_state/src/conductor.rs:107-250` | chain-level lineage is native; the only sys-validation rule is "nothing after CloseChain" — reads of a closed chain are unaffected |
| Upstream shipped its own chain-switch worked example: `MigrationRecord{summary, signature, signer}` validated by `trusted_signers` from DNA properties + `verify_signature` | `crates/holochain/tests/tests/migration.rs` (backport #5842, 2026-06-26; present in 0.7.0) | Lane A's kernel is upstream-tested; it preserves **authorship** but not the original **notarization** (it re-signs a summary) — the gap §2 closes |
| Upstream says the `close_hash` one-chain-one-successor binding is **not enforceable in validation** (validators lack visibility of the old network) | `docs/design/dna_migration.md` | fork detection is an out-of-band observer's job — ours is the storage plane (§5) |
| `InstallAppPayload.agent_key: Option<AgentPubKey>` — a second app can be installed under an existing key; cross-cell `call(CallTargetCell::OtherCell)` is cap-grant-gated but allowed | `holochain_types/src/app.rs:109-113`, `host_fn/call.rs:161-233` | **no re-key**; the v1 cell is readable from the v2 coordinator on a dual-cell peer |
| Storage assumes ONE app id and the FIRST provisioned cell per role; `install_fresh` always mints a key; lineage drift = uninstall+reinstall | `happ_manager.rs:79,560,741`, `hc_client.rs:209-286` | the storage change is bounded and named (§6) |
| Rung 5's verify refuses a DNA-line crossing as a typed refusal; vehicles register by `ArtifactClass`; the manifest already carries `envelope.lineageParentCid` and per-role `dnaHash` | `release_adoption/verify.rs:476-486`, `apply.rs:189`, `rakia/schemas/v1/release-manifest.schema.json` | the crossing becomes a **positive branch** of an existing guard, not a new controller |
| Mishpat `Commitment{action, payload_json, signed_at}` with an open action vocabulary; revocation = a new commitment `revokes-commitment`; `cid = entry_hash` | `mishpat_integrity/src/lib.rs:273-278`, `mishpat/src/commitments.rs:185-206,343-359` | the notarized path is a `migrates-lineage` commitment — **no new entry type, no hash move on mishpat** |
| `node_registry` is the smallest safe rehearsal DNA: 6 entry types, 1 storage call site, its own bundle, not in the bridge-health SUPERVISED list | `node_registry_integrity/src/lib.rs:220`, `node_registry_api.rs:92`, `conductor_bridge_health.rs:648` | the mesh keeps working while node_registry is mid-migration |
| hc-rna is compiled into every DNA; `bridge_call` runs in wasm via `call(OtherRole)`; the export seam is live but **unbounded** (no cursor/limit); the migration extern returns "not yet implemented" | `rna/rust/src/bridge.rs:71`, `content_store/src/lib.rs:11476-11611`, `content_store/src/migration.rs:192` | the road exists as a door; the carry function must be **bounded** (the conductor call is uncancellable) |
| **MEASURED 2026-09-04 (Probe A, 0.7):** `verify_signature`, `dna_info()` and `Action` embedded verbatim all work inside `validate`; a v1 record re-created on v2 keeps its EntryHash (`uhCEkkcUK8Bx…` on both); the witness carrying the v1 action + signature is ACCEPTED; a flipped signature byte and a foreign lineage hash are REFUSED with typed messages; two cells under one agent key in one conductor. A coordinator-only change is DNA-hash-NEUTRAL, measured twice on a real DNA with `hc 0.7.0`. | `elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs` (probe A, 51 s, EXIT=0; re-run by the chief 164 s, 2 passed) | the core claim is proven on the fleet's line |
| **MEASURED 2026-09-04 (Probe B, 0.7):** a LATE `open_chain` (v2 already holding several actions) is accepted — Station 8 needs no fallback. **And: `close_chain` is not a fence.** No source-chain guard exists (`holochain_state/src/source_chain.rs` mentions CloseChain only under `#[cfg(test)]`); the author's next create after CloseChain SUCCEEDS; in a single conductor the agent-activity authority reports the six post-close actions as `valid_activity`, `rejected_activity` empty, warrants 0. `ActionAfterChainClose` lives only in `sys_validation_workflow::register_agent_activity` (:1355-1362) — a REMOTE authority's rule, unmeasured in a multi-conductor mesh. | same test (probe B, 118 s); test asserts the measured behaviour so a toolchain that starts enforcing goes loudly red | the sunset cannot rest on Holochain refusing post-close writes; §4 step 5 and Station 8 are rewritten below |
| **MEASURED 2026-09-04 (Probe B2, two conductors, 0.7):** the REMOTE agent-activity authority does apply `ActionAfterChainClose` — bob's authority reports alice's chain `Invalid` at seq 11, `rejected_activity = [11]`, **warrants = 1** — but ONLY the immediate successor of the CloseChain is refused: seqs 12–16 are `valid_activity` again (the rule inspects `prev_action`). Bob's `get` of the post-close record SUCCEEDS locally and over the network. Alice's own authority still says `Valid`. | `happ_lineage_migration.rs` probe B2 (66 s; re-run by the chief, 1 passed) | Holochain's fence = one rejected action + a warrant, not a stopped chain; the warrant is the substrate's own gossipable evidence that a close was violated — wire it in as evidence; our carried-proof rule stays load-bearing because the bytes remain fetchable |
| No ecosystem project ships an authorship-preserving hApp→hApp migration; Kangaroo states breaking versions do not port data; 0.8 roadmap lists DNA migration as continuing work, no date on chain continuation | web, 2026-09-03 | nobody is coming to solve this; the rails are ours to finish |

## 2. The notarization-carrying record (the core)

The v2 network must be able to check, with no access to v1, that *agent A asserted content X at
time T, and v1's validators witnessed it*. The v1 `SignedAction` is exactly that proof, and §1
shows v2 can verify it. So:

```
NotarizationWitness (v2 integrity entry type)
  lineage_dna_hash : DnaHash            // the v1 DNA this proof was witnessed in
  proofs           : Vec<CarriedProof>  // ≤ WITNESS_BATCH (16) — head-plane bundling, §3
  CarriedProof
    action    : Action                  // the v1 action (header + data) — ≈300 B
    signature : Signature               // the v1 author's signature over it — 64 B
    entry     : Option<Entry>           // ONLY when the carrier is not the author (§2.2)
```

**Validation (v2 integrity zome, every peer, deterministic):**
1. `lineage_dna_hash ∈ dna_info().modifiers.properties.lineage` — the v2 DNA declares its parents
   in its properties, so its identity commits to its lineage and every peer agrees.
2. For every proof: `verify_signature(action.signer(), signature, action)` — **`signer()`, not
   `author`**: a `CloseChain` toward `MigrationTarget::Agent` is signed by the new key.
3. `hash_action(action)` is the proof's identity (the witness stores nothing else; hash = fact).
4. If `entry` is present: `hash_entry(entry) == action.data.entry_hash`. If absent: the carrier is
   the author, and a native v2 `Create` with that `entry_hash` MUST exist on the carrier's chain
   (checked via `must_get_valid_record` on the *v2* chain — same DNA, allowed).
5. A witness cannot be updated or deleted (like upstream's MigrationRecord).

What this buys, precisely: **CID continuity** (same EntryHash where the author re-creates
natively), **notarization continuity** (author, timestamp, entry hash and signature are the v1
ones, re-verified by v2's validators), **lineage continuity** (the v1 DNA hash is bound into the
v2 identity and named on every witness). What it does not buy: v1's *DHT-level* validation receipts
(who in v1 validated the op). The witness carries the author's proof, not the validators'. That
is the same trust boundary upstream's design §4 draws ("content carrying a valid signature made
by the agent's own key on a previous network can always be trusted — even where another agent
copied it across"), and it is the boundary Lane B would remove (§9).

### 2.1 Self-carry (the author migrates)

The author's v2 coordinator calls the v1 cell (`call(OtherCell(v1_cell))`, bounded
`{cursor, limit}`), receives its own v1 records, `create_entry`s each one natively in v2
(byte-identical → same EntryHash; the v2 action is a new notarization by the same key) and
commits one `NotarizationWitness` per batch with `entry: None`. Both facts are now in v2: the
fresh v2 notarization AND the original v1 one, bound by entry hash.

### 2.2 Held-carry (the author has not migrated, or never will)

A dual-cell peer holding v1 content it did not author carries it with `entry: Some(bytes)`. It
cannot `create_entry` as the author, so the entry lives inside the witness; v2 readers reach it
through the witness index (a link from the entry-hash anchor). The original notarization is
intact; the carrier is only a courier, and says so. When the author later migrates, their
self-carry supersedes the held copy (the storage projection prefers a native v2 record over a
held one for the same entry hash). This is the companion §7 bridge — "never fully absorbed and
never finally excluded" — as a record type.

### 2.3 The rule that makes every future migration cheaper

**Every DNA carries `NotarizationWitness` and a `lineage` property from its next integrity change
onward.** That is the operator's "they have to know how to upgrade before they hold data": the
witness type in a DNA is its knowledge of how to be migrated *from* AND how to carry *back* (a
v1 that has the type can receive v2 witnesses, so a bridge peer mirrors both ways and a laggard on
v1 keeps seeing the household). The rehearsal (§8) is exactly "add the witness type to
node_registry" — the first hash move teaches the DNA to migrate.

## 3. P2P Design Gate

### Entity: NotarizationWitness
- **Classification**: Notarized (A) — the protocol would be lying if a carried proof changed silently; it is a thing in its own right (a witnessed carriage), not an attribute of the content.
- **Justification**: it is the only object in v2 that proves the v1 notarization; nothing reconstructs it.
- **Head-Plane Cost Budget**: one witness per ≤16 carried records → node_registry rehearsal: tens; lamad at 3.5k heads: ~220 witnesses, not 3.5k (batching IS the bundling shape — a composite root per author-batch). Unbounded at 1 year only if migrations recur; each migration adds ≈ heads/16. Declared.
- **Network Stakes**: all four stages; **floor-protected** (Constitutional class — a witness is a proof; never stage-priced).
- **Content Address Strategy**: Content-Derived (CID) — `entry_hash`; the witness's identity is its proofs.
- **Transport Affinity**: n/a (entry, not blob).
- **Source of Truth**: Holochain DHT (v2).
- **Integrity Zome + DNA-hash class**: `node_registry_integrity` for the rehearsal (each migrated DNA's own integrity zome thereafter) — **DNA-HASH-MOVING by definition** (v2 is a new DNA; this type is the change).
- **Coordinator Zome** (as landed, Task 9): `node_registry_coordinator::carry_from(CarryInput{v1_cell, cursor, limit}) -> CarryReceipt{carried, self_carried, next_cursor, v1_digest, v1_total, witness_hash}` — one page (≤16), ONE witness per page, `witness_hash` a base64 string (`""` = empty page), `v1_total` read from v1's export page (never derived from `carried`), `self_carried` = records re-created natively (a re-created entry whose hash differs from the carried action's is refused, never held); `get_witnesses_for(entry_hash) -> Vec<Link>`. Measured on 0.7.0: a same-agent cross-cell call needs no cap grant in `init`.
- **Projections**: SQLite — the migrated entity's existing table gains `notarized_action_hash`, `notarized_dna_hash`, `notarized_at` (from the proof) beside `dht_anchor_hash` (the v2 action); `carried_by` (author | courier). Automerge sync: no.
- **HTTP Route**: none new; `GET /db/p2p/conductor-diagnostics` and `GET /version` (runtime passport) gain the per-role `{v1, v2, authoring, reading}` cell view.
- **Anti-Pattern Check**: no UUID; no per-host authoring (the witness is authored once, through the conductor, witnessed by v2's DHT); reach/head/replication kept distinct (a witness moves neither reach nor the content head).

### Entity: migrates-lineage commitment (Mishpat)
- **Classification**: Notarized (A) — reusing `Commitment` (existing type, `cid = entry_hash`).
- **Head-Plane Cost**: one per migration + one per revocation/sunset. Negligible.
- **Network Stakes**: floor-protected (Constitutional).
- **Address**: Content-Derived (entry hash), as every commitment.
- **Integrity Zome + DNA-hash class**: `mishpat_integrity` unchanged — **DNA-hash-NEUTRAL**; the new `migrates-lineage` / `sunsets-lineage` action arms are coordinator validation (`validate_commitment_payload`) — a rung-5 hot-swap.
- **Coordinator Zome**: `mishpat::create_commitment` (existing) with `action: "migrates-lineage"`, payload `{role, from_dna_hash, to_dna_hash, release_cid, constitution_root, roster_cid, signatures: [(agent, sig)] (k-of-n over the payload CID), evidence: {soak: [cid], forecast: cid, deliberation: cid}, window: {opens_at, revert_until, sunset_requires}, revert: {release_cid}}`; revocation via the existing `revokes-commitment` arm (same quorum rule). Coordinator validation (`validate_commitment_payload`) checks the roster chain and the k-of-n signatures over a `signing_payload_cid` (the canonical payload EXCLUDING `signatures` — signing the final CID would be circular). **Honesty clause (Codex, 2026-09-04):** coordinator validation is AUTHOR-side — `mishpat_integrity` accepts unknown actions with no action-specific rule (`mishpat_integrity/src/lib.rs:806-893`), so a modified coordinator could commit an under-signed commitment and peers' integrity validation would not refuse it. Peer enforcement therefore comes from the CONSUMERS: the adoption controller's `verify_path` re-verifies the embedded roster proof itself (C5 — evidence, never authority), and the v2 integrity zome enforces the root binding via its properties (§4.1). Making mishpat's OWN integrity zome refuse an under-signed `migrates-lineage` is a hash-moving change to mishpat — which is exactly a lineage crossing of this epic's kind, scheduled as mishpat's first crossing after the rehearsal. Two more facts from the same read: the storage projection of an unknown action stores `{}` bounds (`mishpat_projection.rs:193-231`) — the controller reads the commitment's `payload_json`, never the projected bounds; and revocation sets `revoked_at` without changing `state` (`signals.rs:1010-1045`, `db/mishpat_commitments.rs:125-145`) — adoptable ⇔ `state == active ∧ revoked_at IS NULL`, and the revocation arm must carry the same quorum rule (it does not today).
- **Projections**: `mishpat_commitments` (existing; `state`, `revoked_at`).
- **HTTP Route**: existing commitment routes.
- **Anti-Pattern Check**: the commitment is the elohim's, not a household vote and not a per-node flag; no per-node veto (the constitutional posture of rung-5 §4 stands; course-set: `project_upgrade_authority_constitutional_elohim`).

### Entity: council roster (attestation)
- **Classification**: Notarized (A) — reusing the elohim DNA's `Content` entry with `kind: council-roster` (the same `metadata_json` valve the release manifest rides). DNA-hash-NEUTRAL.
- **Head-Plane Cost**: one per roster rotation per reach; tens per year. Negligible.
- **Network Stakes**: floor-protected (Constitutional).
- **Address**: Content-Derived (CID); each roster names its predecessor's CID and carries the predecessor's k-of-n signatures; the chain terminates at `constitution_root`.
- **Coordinator Zome**: `content_store::create_content` (existing) — the roster is content; verification is done by every consumer against the chain, never by trusting the author.
- **Projections**: SQLite content projection (existing); consumers cache the resolved roster per reach.
- **Anti-Pattern Check**: no standing key; no per-host roster; sovereignty vocabulary kept at the commons apex, not the individual.

### Entity: `constitution_root` + `lineage` (DNA properties)
- **Classification**: part of the DNA identity (properties fold into the DNA hash) — not a separate entity; declared here because every validator reads them via `dna_info()`.
- **Change class**: any change moves the DNA hash — an amendment IS a lineage crossing under this spec.

### Entity: release manifest, artifactClass `happ-lineage`
- **Classification**: Notarized (A) — the existing rung-5 `Content` entry (`kind: release-manifest`), DNA-hash-NEUTRAL.
- **Change**: `artifactClass` enum gains `happ-lineage`; `appliesTo.roles[role]` gains `migrateFrom: DnaHash` (required for this class) and `lineage: [DnaHash]` (what the v2 properties declare); `adoptionDiscipline` gains `path: {commitmentCid}` — the notarized migration path this release walks.
- **Everything else** as rung 5 (channel, staging→earned election, canary, soak attestation, promote, converge, revert-by-re-election).

### Design Constraints Discovered
- **MEASURED (Probe B):** a late `open_chain` is accepted — the sunset opens v2 from v1's close after the window, no fallback needed. **But `close_chain` is not self-enforcing:** the closing node's own conductor accepts writes after CloseChain, and only a remote agent-activity authority applies `ActionAfterChainClose` (unmeasured across conductors — **Probe B2**, two conductors, does a peer refuse and warrant post-close activity?). So the sunset's irreversibility is OUR rule, enforced where we already validate: (i) the storage controller disables the v1 cell and flips `authoring` — the node stops writing; (ii) **v2's witness validation refuses any carried v1 proof whose action sits after the close** — the sunset's `CloseChain` is itself carried as a proof, so v2 knows the close's `action_seq`, and a `CarriedProof` from the same author with `action_seq > close_seq` is `Invalid`; (iii) **measured (Probe B2):** Holochain's remote authority refuses exactly the first post-close action and issues a WARRANT; the chain tail after it validates again and the bytes stay fetchable — so the substrate's contribution is evidence (the warrant), and (i)+(ii) are the fence. One agent still has one chain per DNA, so a post-sunset revert remains impossible on the same key — the posture stands, the mechanism moved.
- Spec §2 validation rule (4, `entry: None` branch) is **not expressible as written**: `must_get_valid_record` takes an ActionHash and HDI has no entry-hash → carrier's-chain-action resolution. Nearest form is `must_get_entry(entry_hash)` (DHT presence, not chain membership) — weaker; the self-carry binding is instead enforced by the CARRIER's own native `Create` being validated by v2's ordinary rules, and the witness is evidence beside it. Rule kept as intent, implementation recorded.
- `Properties` is `LineageProperties { progenitor_pubkey, lineage, constitution_root }` with `#[serde(default)]` on the epic's two fields — one properties map serves the existing bootstrap-steward reader and the epic. In the probe `lineage` is injected as an install-time modifier (`DnaModifiersOpt`), which folds into the hash exactly as a `dna.yaml` declaration would; production declares it in the bundle — that IS the hash-moving act.
- **Landing rule:** the witness type is gated behind cargo feature `lineage-witness` so the default pack stays byte-identical to `dna-hashes.baseline` (CI's DNA Hash Guard) — v1 = default build, v2 = feature build. The fleet gets the witness type through this epic's own ceremony, never a CI roll.
- The 0.6→0.7 preimage change (`Action{header,data}`) means a v1 proof made under 0.6 cannot be re-encoded and verified under 0.7. Same-line crossings (this spec) are unaffected; a cross-line proof needs raw signed bytes + `verify_signature_raw`. Recorded, out of scope.

## 4. Authority and the common language — the elohim hold the crossing; humans get diversity and a notarized path

**Who decides.** Upgrade authority over the core protocol is a domain of the network itself,
vested one degree removed from humans in the elohim at the constitutional level (the 2026-09-01
course-set, `project_upgrade_authority_constitutional_elohim`). A crossing of the commons DNA line
is therefore **not a consent act** — no household ratifies it, no node vetoes it, nobody is shown a
dialog. It is a decision the elohim make inside the closed system, and they make it the only way
they hold any authority: as a **bounded, revocable, attested commitment** (the compute-commitment
primitive), never a standing key. The runtime tracks the elected head automatically; adoption
discipline (canary order, soak, thresholds) is a constitutional artifact of the release ceremony,
not per-peer preference. Humans keep introspection and escalation (companion §4): they read the
release's own explanation, look at their own elohim's reasoning, and raise mishandling up the reach
ladder — and the revert is the remedy, not a refusal to update.

**What humans get instead of a veto is diversity.** Branches and bespoke communities are the VSM
ecology working, not a deficit: a community may run a declared branch of a DNA with its own lineage.
The requirement the protocol places on that diversity is a **common language** in which any upgrade
or reconciliation chain between branches can be built — so a human can move through the diversity of
the network (join a bespoke community, leave it for the commons, carry their history with them) along
a path that was **notarized before they walked it**. That language is exactly the four primitives of
this spec, which is why they are protocol-canonical rather than one migration's tooling:

| Primitive | What it says, in the common language |
|---|---|
| `lineage` (DNA property) | "this rule version descends from those" — identity commits to ancestry |
| `NotarizationWitness` | "this fact was witnessed there, by that key, at that time — here is the proof" — facts cross branches without losing their notarization |
| migration commitment (Mishpat `migrates-lineage`) | "this path, from that version to this one, on that release, with this revert horizon, is held by these elohim" — the path is notarized before anyone moves |
| bridge map (the manifest's `migrateFrom` + `lineage` + the carry recipe) | "here is how a record on that branch is carried onto this one, and back" |

A bespoke community's branch reconciling to the commons — or the commons absorbing a branch's
innovation — is the same ceremony as a version crossing, spoken in the same language, with the
branch's own elohim holding its side of the commitment. There is no second mechanism.

**The path, executed.**

1. **Proposal.** A `happ-lineage` release is published to the channel in `staging` (rung 5 Station 1). Verify on every peer resolves its `migrateFrom` against installed reality: equal → the crossing is admissible; anything else → `DnaLineageMismatch` exactly as today. **A hApp without a recipe for its parent is a typed refusal, not a wipe.**
2. **Notarized path.** A `happ-lineage` release is adoptable only when a `migrates-lineage` commitment naming its `release_cid` is already notarized — signed k-of-n by the council roster at the reach the channel is born on, under the same `constitution_root` the release's DNA declares (§4.1; in the household rehearsal the bootstrap steward's key is the declared 1-of-1 roster, exactly as rung 5's §9 MVP authority). The adoption controller reads the commitment through the peer's own conductor (I1, C5 — never from the manifest's say-so); absence is `PathNotNotarized`, a commitment signed below the bar or by keys outside the roster is `QuorumUnmet`, a root mismatch is `RootMismatch`.
3. **Window.** The commitment opens it → peers adopt in canary order (rung 5 discipline), each installing v2 beside v1, carrying, and attesting `carried == v1 count` (`CarryReceipt.digest` over the v1 export digest). **Both cells stay live.** `revert_until` is the free-revert horizon the elohim hold themselves to.
4. **Revert (free).** The elohim revoke the commitment (`revokes-commitment`) → the channel re-elects the prior head (rung 5 Station 6) → every peer marks v1 `authoring` again and v2 `reading`; v2 cells are **disabled, never uninstalled** (their witnesses are evidence). Nothing on v1 was ever touched. **Records authored on v2 during the window** are re-authored by their authors on v1 (same entry bytes → same entry hash; a native v1 create needs no witness type), with the v2 proof kept in the disabled cell as evidence; until re-authored they are reported `pending`, never lost. Held-carrying them back for an absent author needs v1 to have the witness type (§2.3) — honest absence in the first rehearsal. This is the companion §2 paired revert as code, and it is the remedy escalation reaches for.
5. **Sunset (irreversible, a separate notarized act).** Only after fleet convergence AND a second commitment `sunsets-lineage` do peers `close_chain(Dna(v2))` on v1 and `open_chain(Dna(v1), close_hash)` on v2, carry the CloseChain itself into v2 as a proof, and disable v1. The closed chains stay readable forever. Irreversibility is enforced by us (§3: v1 cell disabled; v2 refuses carried facts after the close). Holochain's remote `ActionAfterChainClose` (Probe B2) rejects the first post-close action and warrants the author — the storage plane reads that warrant as evidence of a violated sunset and files it on the passport; it is not the fence. **The elohim can reverse the crossing at any point before the sunset and at no point after** — which is why the sunset is a distinct commitment with its own soak, not a step in the window.

### 4.1 Constitutional coupling — how the bar is enforced by every agent's own validator

Agent-centricity means nothing exists above the peer: every peer validates with the rules it chose
to install. So "the elohim hold the crossing" is real only if **each peer's own validator** refuses a
crossing the elohim did not hold and accepts one they did, with no controller trusted in between.
The bar for evolution must therefore be something a validator can check in isolation, from bytes it
already has. Three bindings do it; two have a foothold in the tree today.

**Root binding.** Every DNA's properties carry `constitution_root` — the CID of the constitutional
rule-set and its council policy (which change class needs which reach and which threshold) — beside
`lineage`. Properties fold into the DNA hash, so a peer running the DNA has *chosen* that root.
v2's validator accepts a witness or a commitment only under the same root, or under a root reachable
by an **amendment** signed under the prior root (companion §5: the root moves only by the hardest
consensus, across scales). The hApp manifest already carries `progenitor_pubkey` in every role's
properties (`elohim/holochain/dna/elohim/workdir/happ.yaml`) — the single-key training-wheels root
that rung 5's §9 MVP authority uses. This is what it grows into; the rehearsal declares a 1-of-1
roster under that key and says so.

**Quorum binding.** A `migrates-lineage`, `sunsets-lineage` or `amends-constitution` commitment is
valid only when it carries **k-of-n signatures from the current council roster at the reach the
change class requires** (coordinator bytes: low; an integrity crossing: high; the root: global). A
**roster** is an attestation — the set of elohim keys whose earned ceiling meets the bar at that
reach — signed by the *previous* roster, so it hash-chains to the root and a validator verifies the
whole chain with the `verify_signature` it already has (§1). Roster membership IS the earned ceiling
(companion §3): an elohim whose safety or wisdom standing lapses is absent from the next roster; a
bad signature is attested, revocable, and lowers the signer's ceiling. **No key acts alone; no key
is standing.** Plain k-of-n signatures are chosen over a threshold scheme deliberately — verifiable
in validation today with no fork and no new crypto; a threshold signature (FROST) is a later
optimisation of the same binding, not a different one.

**Evidence binding.** The commitment claims, by CID, the evidence classes the bar requires for its
change class — rung-5 soak attestations, the simulation-gate forecast (companion §8), the
deliberation record from the wisdom commons. A validator cannot judge wisdom; it CAN refuse a
commitment missing a required class or the class's signatures. The wisdom is exercised by the
signers, who are accountable for it in the open.

What this preserves and what it couples: at the floor, **agent-centric compute intact** — each agent
validates everything locally and may run a declared branch under a different root (diversity, §4).
At the apex, **the commons' sovereignty** — the commons' notarizations recognise only paths under the
commons root, and the root moves only by the hardest consensus. The "godlike" power is contained by
construction: quorum (no lone key), earned rosters (no standing key), amendment chain (no quiet
root move), local verification (no trusted controller), and the human floor of branch · introspect ·
escalate.

**On blockchain coupling — one narrow concession, no authority moves.** A chain would give one thing:
a global, capture-resistant ordering of which root and which roster are canonical — which also closes
the single hole upstream admits validation cannot close (a closed v1 chain forking into two
successors). The agent-centric answer is that a second successor is **not an attack but a declared
fork** with its own lineage, and the commons is the branch whose path carries the widest-reach
notarization — more honest than "one chain wins", and it is the diversity affordance. The residual
worry is a colluding council rewriting the root's *history*. For that, exactly one coupling:
**optional external timestamp anchoring of the root lineage** (an OpenTimestamps-style witness on a
public chain), floor-protected as auditability (companion §4, §6). It witnesses the ORDER of roots;
it never judges them and holds no authority. Nothing else moves onto a chain.

## 5. The bridge — dual-cell peers, both directions, and the fork watch

- A dual-cell peer's storage runs a **bridge sweep** (the reconcile controller, both cells' signals): new v1 records → held-carry into v2 (§2.2); new v2 records → held-carry into v1 **iff v1 has the witness type** (§2.3; not in the first rehearsal — honest absence, measured at Station 6).
- Laggards (`lag-within-window`) keep authoring on v1 and keep seeing everything, because the household's dual-cell peers mirror. A `declared fork` is a v1 chain that closes toward a DNA not named in any notarized migration commitment (a branch without a bridge map) — the storage plane sees both networks and files it (the out-of-band observer upstream says validation cannot be). `silent staleness` past `sunset` is healed by the ordinary adoption loop.
- Storage's role→cell map becomes `role -> {v1: CellId, v2: CellId, authoring, reading}`; every `call_zome` for the role targets `authoring`; the bridge sweep reads `reading`. The runtime passport publishes it, and `version-matrix --observed` shows the window fleet-wide.

### 5.1 Many versions, many branches — the lineage graph IS the bridge, verification is graded

The operator's question (2026-09-04): *v1 talking to v2 talking to v3 and back — how do we afford
diversity without fragmentation?* The answer is that versions are not islands but nodes of ONE
lineage graph under a root, and that fidelity across an edge is a declared grade, not a yes/no.

**The graph.** Every DNA declares `lineage` (ancestors), `constitution_root`, and the carry
recipes on its edges: a FORWARD recipe (parent-shaped record → own shape, deterministic, in its
own integrity zome) and a REVERSE recipe (own shape → parent shape). A witness carries a fact's
original proof (§2) plus the lineage evidence the receiver needs to accept it: for an ancestor,
nothing more (the hash is in the receiver's `lineage`); for a descendant or a sibling, the roster's
notarized commitment that declared that version under the same root, embedded with its k-of-n
signatures (§4.1) — the same check every validator already runs. So **v3 accepts a v1 fact directly**
(v1 ∈ lineage(v3)) without hopping through v2, and **v1 accepts a v3 fact** it could not have known
about, because the commitment that made v3 a descendant verifies in isolation.

**The grade.** Three levels, declared on the projected record like reach and stakes:

| Grade | The receiver can check | When |
|---|---|---|
| **verified** | original signature + action hash + `transform_recipe(carried_entry) == own_entry` re-run in its own validation | every forward hop along ancestry (the receiver holds the recipe) |
| **authentic** | original signature + action hash + the lineage commitment; the projection into its own shape is the COURIER's attestation (revocable, standing-bearing) | every backward hop and every sibling hop (the receiver cannot hold a recipe written after it was packed) |
| **foreign** | signature only; no shared root | a different `constitution_root` — no automatic bridge; reconciliation is a commitment signed under BOTH roots (a treaty, not a carry) |

A laggard on v1 therefore sees the future's facts as genuinely authored, with the courier answerable
for the shaping — never nothing, never a forgery.

**Routing.** Two versions under one root exchange facts through their **nearest common ancestor**:
reverse recipes up to it, forward recipes down from it. A bespoke branch B (from v2) and the commons
v3 (from v2) meet at v2's shape. The lineage graph with recipes on its edges is a routing graph for
facts; the cost and the grade of any path are known before it is walked — the common language, §4,
made computable.

**What keeps it from balkanizing** (structural, not exhortation): one root makes every branch family,
and leaving the root is a declared secession — legitimate, outside the bridge; the commons head is the
branch whose path carries the widest-reach notarization, so branches compete by earning, not by
stranding; **bridge stewardship** is a commitment the elohim can name (`replicates-content`'s shape:
enough dual-cell peers per reach that no live version lacks a courier); silent staleness past a sunset
is gated out of participation roles, never cut off from reading; and every DNA carries the witness
type, `lineage` and `constitution_root` from its next change onward, so no version is born mute again.

**Holochain note.** If 0.8 ships chain continuation (§9), versions of ONE network stop needing carry —
validation dispatches per action. Branches still do: a branch is a different network hash by
definition. The graded bridge stays necessary for diversity in the best upstream outcome.

**Seam for the story** (story-maintainer form): chain happ-lineage-migration / between Station 6
(one hop, forward, honest about direction) → the epic's "any two versions under one root can talk"
/ missing node: Stations 11–13 — v1 accepts a v3 fact as *authentic* via the embedded descent
commitment; a sibling branch and the commons exchange through their common ancestor; a foreign-root
fact is refused as `RootMismatch` and reconciled only by a two-root commitment / current state:
designed here, not yet in the feature (the feature is READY at Stations 1–10; these are added after
Station 6 is green, not before).

## 6. What changes, by seam (bounded, named)

| Seam | Change | Class |
|---|---|---|
| DNA (rehearsal: node_registry) | `NotarizationWitness` entry type + `lineage` and `constitution_root` properties + validation (§2, §4.1 root binding); coordinator `carry_from` (bounded) + witness index | integrity (hash-moving — the rehearsal IS this) |
| mishpat coordinator | `migrates-lineage`, `sunsets-lineage`, `amends-constitution` action arms in `validate_commitment_payload` — roster chain + k-of-n signature check (§4.1) | coordinator (rung-5 hot-swap) |
| elohim DNA content | `kind: council-roster` content (metadata_json valve, like the release manifest) | DNA-hash-neutral |
| release manifest schema | `artifactClass: happ-lineage`; `appliesTo.roles[*].migrateFrom` + `.lineage` (`migrateFrom ∈ lineage` enforced in Rust); `adoptionDiscipline.path.commitmentCid`; a root `if artifactClass == happ-lineage` requiring all three (fragments: Codex E §4; homes `release-manifest.schema.json:34-42, 166-195, 295-321`) | schema |
| release_adoption verify | `verify_envelope` (`verify.rs:465-489`): `happ-lineage` positively accepts `migrateFrom == installed` with `dnaHash` as target, every other class keeps the hard mismatch; a pure `verify_path` after the envelope + L2 checks and before threshold/artifact (`verify.rs:700-735`), fed a caller-fetched `Answer<PathEvidence>` because the module does no I/O — `Absent` → `PathNotNotarized`, unreadable is its own refusal (C4), `revoked_at` set → `PathRevoked`, signatures below k or off-roster → `QuorumUnmet`, root differs → `RootMismatch` | storage |
| release_adoption apply | `HappLineageVehicle` (`handles: [happ-lineage]`): install `elohim@<hash12>` under the existing key (`agent_key: Some`), enable, authorize both cells, drive `carry_from` to cursor end, attest digest; on sunset: close/open; on revert: disable v2 | storage |
| happ_manager / hc_client — **ADDITIVE (Codex D2, 2026-09-04): `HcClient` itself is untouched** | five touch points: (1) `install_lineage` beside `ensure_happ_installed` — never the destructive reinstall path; `install_fresh(agent_key: Option)` (`happ_manager.rs:740-764`); (2) a `role -> app_id` resolver consulted by `HcClientRegistry::connect_role` / `connect_role_forever` (`hc_client_registry.rs:186-206, 253-280`) — a second client for `elohim@<hash12>` needs zero `HcClient` changes (`HcClientConfig` takes app_id/role per instance, `hc_client.rs:80-91`); (3) `NodeRegistryApi` becomes registry-backed and snapshots the current authoring client (`node_registry_api.rs:40-69, 82-98`) — call sites never learn app ids; (4) lineage-aware signal projection: subscribers today discard the outer `cell_id` (`hc_client.rs:935-948`) and `stewarded_nodes` hardcodes `h_app_id = "elohim"` (`db/stewarded_nodes.rs:253-271`) — a dual subscriber keeps `{app_id, dna_hash}`; (5) dual-cell passport/diagnostics (`runtime_passport.rs:105-136`, `/health` `http.rs:2487-2507`) — today v2 would be invisible, not disruptive. Guard: `ensure_happ_installed` must never be pointed at the one-role side app (it would report four roles missing, `happ_manager.rs:663-690`). | storage |
| runtime passport / diagnostics | per-role dual-cell view | storage |
| storage projection | `notarized_*` columns + `carried_by`; bridge sweep | storage |
| a2o | `features/delivery/happ-lineage-migration.feature` Stations 1–9 + steps composing the rung-5 drivers | verification |

## 7. Concern-canon answers (birth rule, C0–C14)

C0 plane: DHT (witness, commitment) + storage controller (vehicle, sweep) — placed. C1 anti-self-election: the channel's election, unchanged. C2 monotonic authority: `authoring` flips only on a verified head change or a revocation; never both cells. C3 liveness: bounded `carry_from` batches; the sweep is a ticker, not a loop over an uncancellable call. C4 honest absence: `carried < v1 count` is reported as a number, never as done; a v1 without the witness type reports `backward-carry: unavailable`. C5 evidence-not-authority: a manifest's `migrateFrom` is a claim verified against installed reality; a commitment is read through the peer's own conductor. C6a bounded work / C6b idempotent: `carry_from` is cursor-driven; it is NOT yet idempotent by entry hash — a retried page re-creates entries (same entry hash, new action) and commits a second witness. Filed follow-up station: query the chain for the entry hash before `create_entry`. C7 advertise/serve: the passport advertises exactly the cells that serve. C8 observability: `CarryReceipt` per batch + attestation. C9 identity-lineage: same agent key (no re-key); DNA lineage in properties; chain lineage via Close/Open at sunset. C10 contract evolution: `artifactClass` is an additive enum; old controllers refuse unknown classes (already true). C11 backpressure: batch size + the io/ram guards. C12 consent/authorization: the elohim's notarized migration commitment authorizes the path — k-of-n of an earned roster under the declared root (§4.1); no per-node consent exists by design. C13 graduated authority is ALSO §4.1: reach and threshold rise with the change class. C13 graduated authority: canary → promote, rung 5. C14 witnessed residual: the closed v1 chain and disabled v2 cells are kept, never deleted.

## 8. The rehearsal (household mesh, 0.7, this week's shape)

v1 = node_registry as installed. v2 = node_registry + `NotarizationWitness` + `lineage: [v1]`.
Three peers; james is the canary; jessica stays on v1 until Station 6. The story is
`genesis/a2o/features/delivery/happ-lineage-migration.feature` (Stations 1–9); its runnable check is
the habit `elohim/holochain/.epr-meta/happ-lineage-migration.habit.md`. Order of implementation is
the order of stations; each station is a measured red before it is green.

## 9. Lane B — the conductor fork: keep ONE DHT across an integrity change

We ship a forked conductor already (nine patches, a re-carried relay fallback). Upstream's
*chain continuation* draft (`docs/design/dna_migration_chain_continuation.md`, design-only, no
`ContinueChain` in any shipped tree) names the deeper mechanism: split the network identity
(kitsune space) from the integrity identity, put an `IntegrityHash` on the action header, and
dispatch app validation per-op to the integrity version the op names. Under that mechanism an
integrity change does **not** create a new network: no re-authoring, no witnesses, no carry —
the same actions in the same DHT, validated by the rules in force when they were authored.
Old peers hold ops that name an integrity version they do not have as *pending-unknown* until
they upgrade — mixed-version peers keep talking, which is this program's north star.

That is the prize; the cost is deep (kitsune space keying, DhtOp integration, sys-validation,
the `ActionHeader` preimage moves once). So Lane B is not the first move — it is a **bounded
spike with a measured question**, run in the fork after Station 4 of Lane A is green:

- **B1 (smallest fork slice, ≈ days):** a validation host fn `must_get_record_from_lineage(dna_hash, action_hash)` that dereferences a *locally installed* lineage cell's store when both cells are on the conductor. Removes proof embedding for dual-cell peers; a peer without the v1 cell still needs Lane A's witness. Measures: can the fork make a foreign lineage visible to validation at all?
- **B2 (the real question, ≈ weeks, decide after B1):** integrity-version dispatch — `IntegrityHash` in the header, validation dispatched to the matching integrity wasm, network space keyed by a stable `NetworkHash`. Measures on the mesh: two integrity versions, one DHT, mixed peers gossiping, no re-authoring.
- **Decision rule:** Lane A ships regardless (it is the fallback every DNA carries, §2.3, and it is what a peer without a fork can run). Lane B graduates from spike to program only if B2 measures green on the mesh AND the patch is upstreamable (0.8 roadmap continues DNA migration; a contribution beats a permanent fork — the relay fallback precedent, 0.7 guide Lane H).

## 10. Definition of done (answers the backlog home's DoD)

Stations 1–10 of the story: 10 is the coupling's negative control — a path signed below the bar, or
by keys outside the roster, or under a different root, is not a path (`QuorumUnmet` / `RootMismatch`
on every peer's own verification).


proposal → the elohim's notarized `migrates-lineage` commitment → per-peer window (install beside,
carry with witnesses, attest) → laggards mirrored, forks filed → free revert by re-election → the elohim's notarized `sunsets-lineage` → per-peer Close/Open → closed chains readable. Stations 1–10 green on
the mesh with the node_registry rehearsal; §9 spike verdict recorded; §4 posture accepted.

## 11. Progress — from idea to validated delivery (the hub; every follow-up starts here)

This section is the epic's running ledger. Each row is a dated fact with its evidence; nothing here
is intention. The habit atom (`elohim/holochain/.epr-meta/happ-lineage-migration.habit.md`) carries the
runnable check and its DELTA lines; this table carries the epic-level picture the operator reads first.

### 11.1 De-risk probes (decided 2026-09-04 — cheapest first, each retires a named unknown)

| Probe | Retires | Owner | State | Evidence |
|---|---|---|---|---|
| **A** sweettest: v1 + v2 (node_registry + witness) cells under ONE agent in one conductor; a v1 record carried into v2 as a witness, accepted; tampered signature and foreign DNA hash refused | the core claim: `verify_signature` in `validate`, action round-trip, entry-hash equality, the integrity-change build, two cells one key | Opus (rust-architect) | **PASS 2026-09-04 on 0.7** (independently re-run by the chief: 2 passed, EXIT=0) | `sweettest/src/tests/happ_lineage_migration.rs`; v1 `uhC0kyvKwO2J…` (default = pristine), v2 `uhC0kbccGCEQ…`; entry hash identical across the line; accept + two typed refusals |
| **B** sweettest: late `open_chain` (v2 authored many actions, then v1 `close_chain` → v2 `open_chain(close_hash)`) | Station 8's only unmeasured Holochain rule | Opus, after A | **PASS 2026-09-04 with a FINDING** | late open_chain ACCEPTED; post-close create ACCEPTED by the author and by the single-conductor authority (`rejected_activity=[]`, warrants 0) — CloseChain is not a fence; §3/§4 rewritten; **B2** (two conductors) opened |
| **B2** sweettest, TWO conductors: does the remote agent-activity authority refuse post-close activity (`ActionAfterChainClose`) and warrant it? | whether Holochain adds anything to our own sunset fence | Opus | **PASS 2026-09-04** (re-run by the chief) | refuses ONLY seq close+1, warrants 1, tail valid again, record still fetchable — the warrant is evidence, our fence stands; coordinator gained `agent_activity_of` / `get_record_at` (hash-neutral, verified) |
| **G** feature-gate `lineage-witness` so the default pack is byte-identical to baseline; v1/v2 from one tree | landing the witness without a CI hash move | Sonnet | **DONE 2026-09-04**, kernel committed `e233bb4f7` | default pack = pristine `uhC0kyvKwO2J…` (verified against a git-HEAD control build and re-hashed by the chief); v2 `uhC0kEKiIscI…`; 4 cargo-check configs green; probes 2 passed after gating; `just build-witness` |
| **C** mesh: install a second app under an EXISTING agent key by admin websocket (`elohim@probe`), watch the storage client | whether the dual-cell change is ADDITIVE (second client per app id) rather than a hot-path refactor | chief (this session) | **subsumed 2026-09-04**: answered by D2's read (invisible, not disruptive); the live confirmation is Station 3 on the 0.7 mesh, not a separate probe | D2 |
| **D** local `hc dna pack` of node_registry with the witness type added | the local DNA build; fallback = CI DNA pipeline artifact | folded into Probe A (Opus builds v2) | in flight | — |
| **D2** read-only: the additive dual-client design in `hc_client_registry.rs` / `hc_client.rs` (what breaks with two app ids, what does not) | collapses the 2-day refactor to a switch, or proves it cannot | Codex | **DONE 2026-09-04 — ADDITIVE** | `HcClient` unchanged; five touch points recorded in §6; no code asserts one app; the side app is invisible to `/version` and `/health` today, not disruptive |
| **E** read-only: the mishpat `migrates-lineage` / `sunsets-lineage` arms and the manifest schema delta as a proposal | shapes the coordinator + schema lanes; finds contradictions between the epic and the code | Codex | **DONE 2026-09-04** | proposal adopted (payload shape, `signing_payload_cid`, schema fragments, `verify_path` placement); THREE contradictions found and resolved in §3: author-side vs peer-side enforcement, `{}` projected bounds, null properties today |

### 11.2 Stations (the story's finish lines; RED until measured on the household mesh)

| Station | State | Evidence |
|---|---|---|
| 1 admissibility (verify's positive lineage branch) | red, not started | — |
| 2 notarized path (`PathNotNotarized`) | red | — |
| 3 install beside, same key | red | — |
| 4 self-carry, notarization re-verified | red on the mesh; the KERNEL is green in sweettest (Probe A) | `happ_lineage_migration.rs` |
| 5 held-carry | red | — |
| 6 bridge direction | red | — |
| 7 revert before sunset | red | — |
| 8 sunset | red; REWRITTEN 2026-09-04 (the fence is ours, not the conductor's) | Probe B |
| 9 forged witness refused | red on the mesh; the kernel's two refusals are green in sweettest (Probe A) | `happ_lineage_migration.rs` |
| 10 quorum / root refused | red | — |
| 11–13 many versions (§5.1) | designed, not in the story yet | — |

### 11.3 Operator decisions

- **2026-09-05 — WIP slot: HELD.** The habit `happ-lineage-migration` stays declared, red and `active: false`; the fence keeps dataplane-convergence and runtime-death-witnessed. Execution proceeds under the plan without a slot; the slot question returns when a station is ready to flip.
- **2026-09-05 — §4 posture ACCEPTED; epic graduated Draft→Active.** The crossing is the elohim's notarized path, no per-node consent, the sunset irreversible and ours to enforce.
- **2026-09-05 — execution: subagent-driven** (one fresh implementer per task, task review after each; ledger `.superpowers/sdd/2026-09-04-holochain-evolution-epic-mvp-plan/progress.md`).
- Lane B (§9) timing: after Station 4 by default (Task 15).
- **Integrator (push ordering) — OPERATOR ITEM, 2026-09-05:** neither overnight session holds GitHub credentials (measured by the peer session: `git push --dry-run` refused on both remotes), so nothing was pushed. The morning integrator pushes **`elohim/rakia` commit `4947469` first** (`git -C elohim/rakia push origin main`), **then `dev`** (58+ commits ahead of `origin/dev`, both sessions' work) — the superproject's gitlink `9b0f7acee` references that rakia SHA and CI's checkout dies on a dangling gitlink.

### 11.4 Ledger (newest first)

- 2026-09-05 — **Task 11 part 2a landed** (`f508e75c9`): mishpat `create_lineage_commitment` — the CALLING agent signs `signing_payload_cid` with its own key inside the zome (`sign_raw`, the raw-bytes counterpart of the validator's `verify_signature_raw`; in 0.7 `AgentInfo` carries only `agent_initial_pubkey`, which is the lineage's one key by design), appends `{agent, signature}` in exactly the validator's rendering, refuses a double signature, then takes the ordinary create path; it accepts no key and no bytes to sign, ever. Sweettest 16/16 — the same unsigned payload through plain `create_commitment` is refused, so the validator, not the extern, is what was satisfied. Mishpat DNA hash unchanged (coordinator-only → reaches the mesh by hot-swap). This closes the signing gap Task 10 found: the a2o harness could not sign raw bytes with the agent's real key from `@holochain/client`.
- 2026-09-05 — **Task 9 landed** (`fc7bc8dbe` + hardening `85a9c49ef`): v2 `carry_from` pulls ONE bounded page from the v1 cell across cells (`call(OtherCell)` — measured on 0.7.0: no cap grant needed for the same agent), re-creates the agent's own records natively with the SAME entry hash (guarded: a re-created entry whose `hash_entry` differs from the carried action's is refused as schema drift, never held), held-carries other authors, commits ONE witness per page, and returns `carried / self_carried / next_cursor / v1_digest / v1_total / witness_hash`; the default `node-registry.dna` hash is byte-identical (`uhC0kyvKw…`), v2 is `uhC0kEKiI…`; sweettest 5/5 (the new test asserts same entry hash + different action hash in v2, one witness link, cursor end, `carried == v1_total`). CI packs and archives `node-registry-v2.dna` without touching the hash baseline (verified by the integrator's next `[build:dna]`). Corrections to this document: the coordinator interface (§2) and C6b (§7) — the carry is NOT yet idempotent by entry hash; the follow-up station is filed. **Task 10 part 2 in flight** (`/admin/lineage/reset`, `hc-mesh.sh lineage-reset`, fixture before/after convergence).
- 2026-09-05 — **Task 8 landed** (`245d94517`): the passport shows the dual cell — `GET /version` lists every `elohim@…` side app and, per role, `lineage { readingAppId, authoringAppId, readingDnaHash, authoringDnaHash, closed }` only while a window is open or after a sunset (an empty window renders byte-identically to before, tested); the passport reads a `LineageRoles` snapshot, never the handle. `/admin/adoption` was already naming `path_not_notarized` / `path_revoked` / `quorum_unmet` / `root_mismatch` / `dna_lineage_mismatch` since Task 4 — Task 8 pins them to the wire field with a test. Review approved. **Task 9 in flight** (v2 `carry_from` across cells with one witness per page; the page receipt gains `v1_total` so the Station 3 equality is falsifiable). Mesh: the peer's rung-5 shift closed with two consecutive 9/9 passes; conductors restarted in place (~1 GB/h growth each under ceremonies); the peer is restoring the commons/epic-t2 channel registrations a reaped run overwrote — the epic's live stations wait for that.
- 2026-09-05 — **Task 7 landed** (`b8d14d638` + fix `a506e6d9d`): `HappLineageVehicle` — install beside under the same key (`install_lineage`), bounded carry through the v2 role's `carry_from` (32 per page, stuck-cursor and page-ceiling guards), then `open_window`; any failure leaves the side app installed and the window CLOSED, and a manifest with more than one crossing is refused before any install (one crossing per release at MVP; §5.1 multi-role needs revert-on-failure later). The path-evidence FETCH now lives in `path_evidence.rs`: mishpat `get_commitment` through the peer's OWN conductor, payload parsed against `validate_migrates_lineage`'s real shape, lifecycle from the projection row — and every read failure (bridge down, pool busy) is `Unreachable → conductor_unavailable`, never a fabricated `proposed`/`path_not_notarized` (the reviewer caught the fail-open-into-a-fact; that substitution is exactly what this rung's taxonomy forbids). `install_lineage` now reconciles the enable half on retry. `LineageCarryReceipt.v1_count` is `Option`, sourced only from an additive `v1_total` the v2 zome will emit (Task 9) — never from `carried`, so the Station 3 equality is falsifiable. Review OPUS → fix round 1 → re-review clean. Host note: the peer's Station 9 went GREEN on the mesh; heavy cargo held for its two stability passes.
- 2026-09-05 — **Task 6 landed** (`37638fd77`): `LineageRoles` — the per-role window (reading app / authoring app / closed) that `HcRegistryInputs` reads for every supervised role, plus the `node_registry` registry slot and a registry-backed `NodeRegistryApi`; with no window open the connect path is byte-identical (`app_id_for` falls back to the base app for any role, known or not). Review approved with two deferred minors (duplicated `HcClientConfig` literal, `import_api`'s own client outside the window). Task 5's all-targets clippy fix (`c1e086a77`) re-reviewed clean; batch gate `cargo clippy --all-targets -D warnings` EXIT=0. **Task 7 in flight** (the `HappLineageVehicle` — install beside, carry, attest — and the path-evidence fetch Task 4 left Absent; naming ruling: storage `LineageCarryReceipt`, zome `CarryReceipt`).
- 2026-09-05 — **Task 5 landed** (`c5716539d`): `install_lineage` + `lineage_app_id` — a second app installed beside the base under the SAME agent key with the lineage property, idempotent, never uninstalling; proven live on matthew (`list_apps`: `elohim@test-lineage` and `elohim` under one key `uhCAkcTb…`, side app removed after). Box memory had blocked every build: the three stock 0.7 conductors had grown to ~14 GB after a night of runs; this session restarted them in place under their arks (292 MB each, committed 20 → 7.5 GB) after the peer session's measure run ended, the peer restarted storage to heal the role bridges. Traps: the mesh ADMIN websockets are 4444/4454/4464 (4445… are APP ports — `just mesh status` prints the app ports as CONDUCTOR_URLS); stock 0.7 conductors need a periodic restart on a long-lived mesh (the fork's jemalloc is why alpha does not).

- 2026-09-05 — **Task 11 part 1 landed** (`4e859d61d`, `59cf5c264`): the story's 72 step patterns bound (128/128 steps, 0 undefined in dry-run), Background + Stations 1–2 implemented against real surfaces, Stations 3–10 pending with the story text intact; connect rail timeout-wrapped. Two Thens are red by design until the path-evidence fetch (Task 7's caller) and the self-signing commitment extern (Task 11 part 2) land — commented inline. No live run yet: the mesh's storage binary predates Task 4's verify and the peer session holds the mesh; Tasks 5–8 are queued on its cargo window.

- 2026-09-05 — **Task 10 part 1 landed** (`ad5144d24`): the a2o lineage fixture's candidate + commitment helpers with 18 unit tests (307/307). Finding: nothing in `@holochain/client` or the mishpat zome signs raw bytes under the agent's key, so a commitment's quorum signature cannot be produced from outside — **ruling:** Task 11 adds a mishpat coordinator extern that signs ONLY its own commitment payload with the calling agent's key (never a sign-arbitrary-bytes extern). Storage tasks (5, 6, 7, 8) are queued behind a peer-session freeze of the storage crate (its artifact_pull/watch.rs work; tree must compile and cargo be free).

- 2026-09-05 — **Task 4 landed** (`b76a71a2f`, one compiling commit): `ArtifactClass::HappLineage`, the four path refusals, `PathEvidence`, `verify_path` (after `verify_lineage`, before `verify_threshold`), the positive DNA-line branch, `migrateFrom ∈ lineage` in Rust; 92/92 lib tests, clippy clean. Rebased on the peer rung-5 session's `a73897f77` (already-current BY BYTES — a peer running the release's target coordinator wasm is CURRENT, not a lineage mismatch) and `35f0746ad` (peer-pull artifact source). The two sessions sequenced the shared module by message: patches staged in the SDD workspace, applied on the other's SHA, one commit so every HEAD compiles. Task 5 (`install_lineage`) applying now.

- 2026-09-05 — **Task 3 landed** (`9b0f7acee` schema + `f3d5ae034` packager; the schema lives in the `elohim/rakia` submodule → submodule commit `4947469` + gitlink bump). `happ-lineage` artifact class, `roleBinding.migrateFrom`/`.lineage`, `adoptionDiscipline.path.commitmentCid`, root `if/then`; packager `--migrate-from role=hash` / `--lineage` / `--path-commitment`; 289/289 unit tests. Built on the peer rung-5 session's `9e1ac7a7b` (discipline declared or inherited, never defaulted — the household threshold rule is now structural) and `ab3bf32bc` + `438443cf0` (publish admissible over an earned head — the seam Task 2 filed is CURED; one channel can take a second candidate). **Integrator note:** push the rakia submodule commit BEFORE the superproject gitlink bump, or CI's checkout dies on a dangling gitlink.

- 2026-09-05 — **Task 2 delivered 3/3** on the fresh channel `runtime:coordinators:elohim:epic-t2-20260904035403`: james walked the full canary → attest → promote order; matthew/jessica wedged on the packager's default `attestationThreshold: 2` (a 3-peer household has one attester archetype) and were recovered by a threshold-0 revert of the same candidate (controller-run, 05:10:58Z); `/version` ×3: mishpat coordinator `uhCokfEmz…→uhCokomHU…`, node_registry still on Task 1's `uhCok1d81…`, all other roles byte-identical. **Rule (carried into Tasks 10/11):** household manifests are packaged with `--attestation-threshold 1`; the peer rung-5 session is curing the silent default. Review approved; fix round 3 adds the quorum validator's refusal-path tests and the sunset arm's DNA-hash checks. Also this session: the shared checkout was merged with `origin/dev` by the peer rung-5 session (809635645, 26 ahead / 0 behind); the epic's artifacts and the rung-5 SEAM survived the merge.

- 2026-09-05 — Task 2 code landed (`b34027fad`: `migrates-lineage` / `sunsets-lineage` arms, signature quorum over `signing_payload_cid` with `verify_signature_raw`, quorum on lineage-target revocation; 5 sweettests, hash-neutral). Delivery found a **rung-5 seam**: the driver refuses `publish` on a channel with an earned head (adopt-before-author, no override), so a long-lived channel cannot take a second candidate; the rung-5 fixture mints a channel per run. Filed as a SEAM on the rung-5 habit atom; Task 2's delivery uses a fresh channel (fix round 1).

- 2026-09-05 — **Task 1 complete** (`ad62e5a8e`, `31268cf77`): v1 `export_records` (bounded, cursor-resumable, whole-chain digest) — hash-neutral, sweettest 4/4 + node_registry 3/3, measured record count 5. **First live rung-5 delivery on the 0.7 mesh after the cutover**: the commons channel had been wedged by a promote-before-canary and was recovered with a threshold-0 revert (rung 5's fc090d901 lesson); the candidate then applied on 3/3 peers in 49 s, node_registry coordinator wasm `uhCok825E…→uhCok1d81…`, every other role byte-identical (arc-doc cycle-time row). Trap learned and recorded: `hc dna hash <packed .dna>` is the hash BEFORE the hApp manifest's role modifiers; the installed hash folds them — never compare the two (diagnostic `happ_manager::tests::probe_bundle_dna_hashes`, `b73b8c95c`). Review: approved; the threshold-0 delivery was ruled acceptable (rung 5's canary→attest→promote is its own proven habit); Task 2's delivery must exercise the full order.

- 2026-09-05 — operator: no WIP slot for the habit yet; §4 posture accepted → **epic Active**. Plan execution started subagent-driven; Task 1 (v1 `export_records` + first rung-5 delivery on the 0.7 mesh) dispatched.

- 2026-09-04 — **Probe B2 PASS** (two conductors, 0.7; re-run by the chief): the remote authority refuses exactly the first post-close action and issues a warrant; the tail validates again; the record stays fetchable. Design consequence: §3 (iii) and §4 step 5 now say what Holochain adds — a warrant we read as evidence — and keep the fence ours. Test committed with two hash-neutral coordinator externs. All three probes together: 3 passed, 229 s. The de-risk pass is complete: every named unknown (core claim · late open · close fence · remote authority · additive storage · local pack · hash neutrality) is measured.

- 2026-09-04 — **MVP plan minted**: `genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md` — 15 tasks, one checkbox each → 15 OPEN gap items → the flow's commitments (`epr flow project`); sealed plan→epic. Tasks 1–11 = the MVP (Stations 1–5, 9, 10), 12–14 = Stations 6–8, 15 = the Lane B fork spike. **0.7 mesh baseline is UP**: three stock 0.7.0 conductors under the ark, iroh-relay 1.0.3 on :3340, storage rebuilt from this tree into the dev slot (3 min, incremental), bundle = the 0.7-packed workdir whose node_registry default hash equals this tree's (`uhC0kyvKwO2J…`). Probe B2 (two conductors) still running.

- 2026-09-04 — **kernel committed** (`e233bb4f7`): the witness type rides `--features lineage-witness`; default pack byte-identical; probes A+B in the tree, skipping loudly when the v2 bundle is absent (CI's DNA pipeline does not build the feature variant yet — a plan task). **Hash discipline learned (Probe G):** `hdk_entry_types` / `hdk_link_types` ignore `#[cfg]` on a variant (use two whole cfg-exclusive enums), and ANY line shift in an integrity source moves the wasm — `file!()`/`line!()` constants ride in panic paths — so a gated block must be appended after the original last line with zero inserted lines above. Probe B2 (two conductors, remote authority after close) dispatched. The 0.7 storage binary for the mesh is building into the dev slot (the mesh still runs the 0.6-line binary until `storage-restart`).

- 2026-09-04 — story re-read after the Station 8 rewrite: r5 REVISE (sealed undefined, harness undefined, close vs reading posture), r6 REVISE (passport "sealed" not a field; harness roles), r7 **READY** with two definitional majors applied (ADMISSIBLE/ADOPTABLE; release bridge map vs DNA lineage as two checks). Deferred minors, operator's to pick up or leave: jessica's "wrong courier" concern is motivation, not adjudicated; "held" is overloaded (held-carry vs a held release); `pending` undefined; the revert-horizon far edge (revocation after the horizon, before sunset) is never exercised; a laggard past the sunset (STALE's own case) is not dramatized — both belong to the many-versions stations (§5.1).

- 2026-09-04 — **Probes A and B PASS on 0.7** (Opus; re-run by the chief). The core claim is proven on the fleet's line in 51 s of test. Finding: `close_chain` is not self-enforcing — the sunset's fence moves to our layer (v1 cell disabled + v2 refuses carried facts after the close), Probe B2 opened for the remote authority. Correction to the previous entry: the dev sweettest slot was ALREADY warm on 0.7 (no recompile; `--no-run` in 5.6 s). Landing rule set: the witness rides a `lineage-witness` cargo feature so the default pack stays on baseline (Probe G, Sonnet).

- 2026-09-04 — Codex D2 + E returned (read-only, on the rebased 0.7 tree). D2: the dual-cell change is ADDITIVE (five touch points, `HcClient` untouched) — the largest single risk in the MVP estimate retires; the storage lane drops from ~2 days of hot-path refactor to a resolver + a registry-backed `NodeRegistryApi`. E: the mishpat arm + schema proposal adopted; three contradictions between the epic and the code found and resolved in §3 (author-side coordinator validation → consumers verify the embedded proof; mishpat's own integrity enforcement is mishpat's first lineage crossing; `{}` projected bounds → read `payload_json`; properties are null today → the rehearsal sets `lineage` + `constitution_root` explicitly on v2).

- 2026-09-04 — **baselined on 0.7 at the operator's call** ("avoid rework"): this checkout was 45 commits behind `origin/dev` (the 0.7 push came from another worktree); rebased (6 commits on top, only `habits.yaml` overlapped). The 0.6 probe was stopped before it touched a file and re-dispatched on the 0.7 pins (sweettest holochain 0.7.0 / hdk 0.7 / hdi 0.8). One holochain-0.7 sweettest compile is unavoidable (no compiled 0.7 slot survived) — it is the baseline, not rework. 0.7 binaries persisted at `/projects/.claude-config/tools/{hc-0.7,iroh-relay-1.0.3}` (the other session's scratchpad is volatile). Note for every local proof: locally packed DNA hashes ≠ CI hashes (the local-only `--import-undefined` RUSTFLAGS moves integrity bytes) — the mesh runs local hashes, `dna-hashes.baseline` is CI's; never compare them. The running mesh is still stock 0.6.0 with a 0.6-line storage binary; the 0.7 mesh baseline (storage rebuilt from this tree, `HOLOCHAIN_BIN`=tools/hc-0.7, `MESH_RELAY_BIN`=tools/iroh-relay) is sequenced AFTER the sweettest compile (one cargo at a time under the RAM guard).
- 2026-09-04 — valueflow minted: `epr flow seal` story → epic (cite-seal) and habit → story (`test:happ-lineage-migration`); projected. Downstream commitments appear once the implementation plan's gap items exist — the plan is the next artifact; a2o reports then `epr flow fulfill` them. `serves:` anchors added to this spec.

- 2026-09-04 — probes A/B/D/D2 dispatched (Opus, Codex); C taken by the chief on the running mesh (note: this workspace's mesh runs **stock 0.6.0**, not the 0.7 line — probe C's finding is about the storage client, which is line-independent; A/B run in sweettest on the dev tree's pins).
- 2026-09-04 — §5.1 bridge across many versions and branches added (graded verification; routing via nearest common ancestor; Stations 11–13 seam).
- 2026-09-03 — epic designed from four grounded readers; story Stations 1–10 READY (blind-reader r4); habit born red; renamed from "rung 6" to the Holochain Evolution Epic (spec-level, not manifesto-tier) by the operator.
