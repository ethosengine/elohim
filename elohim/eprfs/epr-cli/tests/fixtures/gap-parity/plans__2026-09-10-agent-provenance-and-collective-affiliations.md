---
title: Agent provenance and collective affiliations — signed personas, identity chains, plural stewards
id: agent-provenance-collective-affiliations
status: proposed
class: devflow
serves: dev-system-equilibrium
date: 2026-09-10
cites:
  - "unified-memory-loop-design | Unified Memory Loop | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "actor-plane-inflight-identity-claims-design | Actor Plane | sha256:6a6dee8249ae76ef | path: genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md"
  - "did-bridge-identity-resolution | DID Bridge | sha256:5769f6cd4c7163ca | path: genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md"
  - "reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md"
  - "holons-are-spaces-how-we-use-holochain | Holons, Spaces, and Holochain | sha256:ac1de36d2423be82 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "stewardship-over-sovereignty | stewardship-over-sovereignty | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "qahal-collective-membership-dht-design | 2026-05-19-qahal-collective-membership-dht-design | sha256:8d7b9704f7aa9ca0 | path: genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md"
---

# Agent provenance and collective affiliations

Follow-on to the 2026-09-09 collective-memory integration (task-3 report reviewed
2026-09-10, changes requested and closed the same day). That slice proved governed
contributions with attribution to a session persona. This slice makes the attribution
verifiable, makes the persona an identity chain rather than a label, and replaces the
declaration's single `steward` field with plural affiliations shaped as the pre-image of
the notarized Qahal Membership. Everything stays local-first: no conductor, no network
head, no new EPR kind, no new DHT entry type.

The mechanism this plan relies on: an EPR head is a small dag-cbor object whose CID is
its identity, whose canonical bytes exclude `cid`, `proof` and `supersededBy`, and whose
coupling legs (knowledge, value, governance) are CIDs that stay pins offline and
dereference to notarized entries online. Files in the tree are pins or caches of heads,
never records. Records live in the append-only `.eprfs/status/*.jsonl` sidecars.

## Why agents are identity chains

A human's body gives continuity; the protocol grounds human identity in imago dei plus
community attestation, never in a key. An agent has no body and its weights are shared
by every instance, so the weights are a kind, not an individual. What individuates an
agent is what is layered on the kind and the lineage of witnessed acts it leaves in
governed memory. "The agent" in any act is therefore the head of an **identity chain**:
an ordered list of layer heads, each a small dag-cbor record that pins the layer beneath
it. The set of layers is open. Today's known layers, hardest first:

| Layer | What it pins | Who may add it (whose proof) | Already exists as |
|---|---|---|---|
| hard | model descriptor: vendor, model id, version, from a controlled vocabulary | the steward declaring it, later the lab | `berth moor --model --lab` fields, normalized |
| tuning | a system prompt or adapter over the base, if any | whoever tuned it | none yet |
| given | the persona definition as an EPR, the subagent package bytes | the package steward | actor-plane "definition address" (sha256 of `.epr-meta/elohim/packages/agents/<role>.json`) |
| contextual | acting as representative of a person or collective | the represented party, as a Delegation on the governance leg | none yet |
| ephemeral | the session: runtime envelope, effort, context budget, task, later compaction events | the session's own key | actor claim + berth moor record |

Each layer head is `{version, layer: <name>, basedOn: Option<Cid>, pin: FileRef | Cid,
proof: Option<Signature>}`. The layer name is an open string so a new layer needs no
schema change; the record shape is closed. Adding a layer costs one sidecar line. A layer
shared by many sessions costs nothing more, because the same bytes mint the same CID. A
session adds exactly one ephemeral head. The four operations the chain must support, and
how each is cheap:

- **find**: `epr actor current --json` returns the chain from ephemeral down to hard; every
  contribution carries the ephemeral head CID, so any act leads to its full chain.
- **inspect**: a layer's pin dereferences locally to a sidecar line or a file in the tree,
  online to a notarized entry; the pin is a CID either way.
- **verify**: a layer's proof is checked against the key of whoever may add that layer, per
  the table; a layer without proof is reported `proof: absent`, never assumed.
- **follow the chain**: walk `basedOn`; a broken or cyclic link is a refusal naming the CID.

**The working boundary.** Identity is the chain head *excluding* the ephemeral layer. That
head is what an affiliation names as a member, what a DID document takes as subject, and
what memory counts as an agent. The ephemeral head pins it and is carried on contributions
as provenance, so a reader can recover which run acted, but attribution, standing and
value resolve through the identity beneath. Measured 2026-09-10 from `.eprfs/status/`: 28
claim lines, 17 sessions, 21 distinct role-at-model strings and 9 model spellings for
about four model families, so the free-text claim string cannot be the hard layer or
spelling drift becomes identity drift. The model descriptor is a controlled vocabulary
normalized from the berth moor record's separate model and lab fields; the claim string
becomes display only.

**Feedback with the right blast radius.** The existing `FeedbackSignal` EPR kind (squelch,
correction, retraction, quarantine, graduated standing impact) targets a CID. Feedback on
a bad run targets the ephemeral head and touches nothing else. Feedback on a bad system
prompt targets the tuning head, and every session chained on it inherits the standing
impact when its chain is walked. Feedback on a package targets the given head. This is how
something seriously wrong reaches the layer that caused it and no further, without any
per-run identity being counted.

Conditions of an act (effort, temperature, remaining context, a compaction) ride on the
ephemeral head or the observation, never on an identity layer. Qualities of an agent (a
Sophia discovery profile) follow the attested-private pattern: raw evaluations private to
the steward, an attested profile notarized against the identity head. Sameness of agent is
a question about which layer, answered by walking the chain, never by collapsing it.

Standing is always mediated. An ElohimAgent member acts under a steward's delegation; the
`stewardship-over-sovereignty` framing applies and every tier in this plan is named by its
community grounding, never by key possession. The DID bridge is the outward projection of
an identity chain: subject is the identity head, controllers are the stewards and the
represented party of any contextual layer, deactivation is what an ended session becomes.
`did:key` derives offline from any ed25519 public key, so a signed persona gets a
standards-legible identity for free.

## Gate classification (p2p-design-gate, run 2026-09-10)

| Entity | Class | Source of truth, local | Source of truth, online | Address |
|---|---|---|---|---|
| signed persona claim | Private (B) | `.eprfs/status/actors.jsonl` claim line | the agent's private source chain | content-derived CID of the claim record |
| identity layer head, any layer | Private (B), reconstructible from its pin | `.eprfs/status/actors.jsonl` layer line | the agent's private source chain; the identity head later the Agent EPR | content-derived CID (dag-cbor) |
| collective declaration | Notarized (A), existing `qahal::Collective` type | `.epr-meta/collective.json` as a pinned FileRef | Collective entry in the imagodei DNA | FileRef locally; entry hash online |
| affiliation | Linked (A2), existing `qahal::Membership` type | `.eprfs/status/affiliations.jsonl` | Membership link on the Collective entry | agent-scoped composite: member, collective, role |
| contribution share | Ephemeral (C), never stored | computed from `.eprfs/status/flows.jsonl` | computed from economic events | none |

Every sidecar line is a `{cid, record}` pair whose CID is re-verified on read; a mutated
line invalidates itself. The local sidecars are the pre-image the crossing mints from,
never a second authority once the notarized entry exists. No DNA hash moves: nothing in
this slice touches an integrity zome. Head-plane cost online is zero new heads, because
memberships are links on the collective entry. Network stakes: all four stages; the
counter-evidence path on `graduate` refusal is floor-protected.

## Delivery stations

- [ ] A persona claim can carry an ed25519 public key and a detached signature over its canonical claim bytes; `epr actor current --json` reports the key, whether the signature verifies, and the claim's `did:key`; an unsigned claim remains a valid honor-system claim and nothing at this floor blocks on a signature.

Native owner: extend the actor store in `elohim/eprfs/epr-cli/src/actor.rs` and the
claim record it appends to `.eprfs/status/actors.jsonl`. Reuse `elohim_epr::proof::AgentKeypair`
and `elohim_epr::signature::Signature` (`signer`, `algorithm = "ed25519"`, 64 bytes).
Key material lives outside the tree: `epr actor keygen --session ID` writes a 32-byte seed
to `$EPR_ACTOR_KEY_DIR/<session>.key` (default `~/.config/elohim/actor-keys/`, mode 0600)
and prints the multibase public key; `epr actor claim --sign` reads it. The signed bytes
are the canonical dag-cbor of the claim record without `proof`, exactly as `Envelope::canonical_bytes`
excludes `proof`. Add `did_bridge` as a path dependency of `elohim-epr-cli`
(`bridges/did/did-bridge`, native workspace, `RUSTFLAGS=""`) and derive `did:key` through its
codec so the string matches the `did-tests` vectors byte for byte. The actor-plane spec's
contract holds unchanged: a claim never blocks, claims stack per session, history is never
rewritten, a tampered line reads as unclaimed. Tests: signed claim round-trips and verifies;
unsigned claim still resolves as current; a claim whose signature fails verification is
reported `signature: invalid` and still counts as unclaimed for governance, never as the
tamperer's identity; key file missing is a refusal naming the keygen command.

- [ ] A claim references an identity chain of layer heads; hard, given and ephemeral are minted from what the session can pin, tuning and contextual are accepted when supplied and otherwise absent; the memory contribution's `author` is the identity head (chain head excluding the ephemeral layer) and its `provenance` is the ephemeral head; `epr actor current --json` walks the chain and reports every layer's pin and proof state, never defaulting an absent layer; the model descriptor is normalized from a controlled vocabulary so two spellings of one model mint one hard head.

Native owner: add `LayerHead { version: u32, layer: String, based_on: Option<Cid>, pin: LayerPin, proof: Option<Signature> }`
with `LayerPin = FileRef | Cid` to `elohim/eprfs/eprfs-agent/src/memory.rs` beside
`Collective` (`deny_unknown_fields`; `layer` is an open string, the shape is closed). Add
`ModelDescriptor { vendor, model, version }` with a normalization table in
`elohim/eprfs/epr-cli/src/actor.rs` that maps the spellings observed in
`.eprfs/status/actors.jsonl` and `flows.jsonl` (`opus-5`, `claude-opus-5`, `gpt-6`,
`gpt-6-astra`, and the rest of the nine) onto one descriptor each; an unmapped spelling is
refused naming the table, never silently minted as a new hard head. Mint the chain in
`epr actor claim`, hardest first: `hard` pins the normalized descriptor; `given` pins the
definition address the actor-plane spec already computes, `basedOn` the hard head;
`tuning` and `contextual` are added only via `--layer tuning=<path-in-tree>` and
`--layer contextual=<delegation-cid>`, in that order between hard and given; `ephemeral`
pins the berth moor record for this session when `berth who` returns one (pin its CID from
the ledger line bytes, raw codec `bafkrei…`) together with the session id and the steward
slot from git author, `basedOn` the identity head. Each head is a `{cid, record}` line in
`.eprfs/status/actors.jsonl` with `kind: "layer"`; the claim line references the ephemeral
head by CID. Memory contributions (`flow/memory/validation.rs`) set `author` to the
identity head CID, add `provenance` as the ephemeral head CID, and keep the current
`agent:<role>@<model>` string as `display`. `epr actor current --json` emits
`chain: [{layer, cid, basedOn, pin, proof: valid | absent | invalid}]` from ephemeral down
to hard and refuses a chain whose `basedOn` walk breaks or cycles, naming the CID. Tests:
two sessions with the same normalized model, package and tuning share every non-ephemeral
head CID and differ only in the ephemeral head; `opus-5` and `claude-opus-5` mint the same
hard head; a re-moor with a changed task mints a new ephemeral head, no new identity head,
and supersedes the claim without rewriting the earlier line; a `--layer tuning=` file
changes the identity head and every later contribution's `author`; an unknown layer name
is accepted and rendered, an unknown record field is refused; a contribution's `author`
chain contains no `ephemeral` layer and its `provenance` chain does; a `FeedbackSignal`
recorded against a tuning head is found by walking any session chained on it, and one
recorded against an ephemeral head is not found from a sibling session.

- [ ] `.epr-meta/collective.json` declares no `steward`; stewards derive from affiliation records shaped one to one as the Qahal Membership pre-image; a directory may declare a child collective whose `parent` pins the enclosing one; a session's actor claim binds its collective of record; source rules speak locality, not reach; `graduate` requires an approving verdict from a Steward affiliate who is not the contribution's author.

Native owner: in `elohim/eprfs/eprfs-agent/src/memory.rs` remove `Collective.steward`,
add `parent: Option<FileRef>`, rename `Reach { Private, Workspace, Repository }` to
`Locality` and `SourceRule.max_reach` to `max_locality`, per the reach/locality split spec
("reach" is the schema-8 audience vocabulary; placement tiers are locality). Add
`Affiliation { version, collective: FileRef, member: String, member_kind: Person | Collective | ElohimAgent,
role: Steward | Contributor | Observer, sponsor: Option<String>, acts_for: Option<String>,
since: String, withdrawn: Option<String> }`, mirroring `imagodei_integrity::qahal::Membership`
field for field so the crossing is a mint. Affiliations are `{cid, record}` lines in a new
`.eprfs/status/affiliations.jsonl` sidecar (source of truth locally: this sidecar, a local
pre-image of the notarized Membership; online: the Membership link on the Collective
entry), not files per directory. In
`elohim/eprfs/epr-cli/src/flow/memory/validation.rs`: resolve the collective of record by
the nearest `.epr-meta/collective.json` walking up from the contribution's source path
(replacing the `COLLECTIVE_PATH` constant) and require the claim's bound collective to
equal it; stewards are affiliations with `role: Steward` and at least one is required;
`REPO_AGENT` stops being the steward test and becomes the default `acts_for` of the root
collective's affiliations; `graduate` additionally requires the approving verdict's
provider to hold a Steward affiliation in the collective and to differ from the
contribution's author (concern C1, anti-self-election). Register the root repository
collective in `genesis/data/collectives/collectives.json` vocabulary terms
(`constitutionalParentId`, `governanceLayer`, `reach`) by adding a `registry` pin to the
declaration; do not copy fixture collectives into `.epr-meta`. Seed
`.eprfs/status/affiliations.jsonl` with one Steward affiliation for the git author as
Person and one Contributor affiliation per agent package as ElohimAgent with
`acts_for` the git author. Update `.claude/scripts/memory-kit/recall-ceremony.py`,
`genesis/a2o/steps/devflow/collective-memory.steps.ts` and the memory-kit and
memory-ceremony packages for the renamed fields, package-first, then
`just codegen agents write` and verify 1919/0. Tests: declaration with `steward` is
refused as an unknown field; zero Steward affiliations refused; child collective in a
subdirectory resolves for a path beneath it and the parent resolves for a sibling path;
a claim bound to a different collective than the source path's is refused naming both;
graduate approved by the author's own persona is refused; graduate approved by a
Contributor is refused; graduate approved by a distinct Steward passes; `maxReach` in a
declaration is refused with the rename named in the message.

- [ ] The crossing story is written and held: a signed local contribution graduates into a wider space with its CID unchanged and its holders changed, the elohim witness is recorded, and the DID document for the contributing identity head resolves with its steward as controller; the equilibrium habit carries one evidence delta and remains RED.

Integration owner: `genesis/a2o/features/devflow/agent-provenance.feature` (executable,
profile `agent-provenance`, steps `genesis/a2o/steps/devflow/agent-provenance.steps.ts`)
covers stations one through three against the built `EPR_BIN`, mirroring the
`collective-memory` profile's isolated temp-repository fixture. `genesis/a2o/features/devflow/collective-crossing.feature`
carries `@requires:household-nodes` and `@wip @design` and is not wired to a profile
until the mesh lane can run it; its steps are the spec for the network slice, not this
one. Both features enter the blind-reader revision loop `genesis/a2o/.epr-meta` requires.
Add both feature paths and the new sidecar to the `memory-ceremony` project in
`genesis/build-manifest.json`; add the profile to `genesis/a2o/cucumber.mjs` and the
`_gate-memory-ceremony` recipe in the root `justfile`. Append one DELTA line to
`.epr-meta/dev-system-equilibrium.habit.md` naming test counts, scenario counts, package
check count and the frozen binary digest, then `python3 .claude/scripts/habits-project.py`.
The habit stays RED: this slice proves verifiable local provenance and plural local
stewardship; it proves nothing about network enforcement, settlement or cheaper ceremonies.

## Explicitly outside this slice

Allocation and settlement (contribution shares moderated by an agreement) wait for the
crossing story to run: the gate refuses unsigned persona-to-person bindings for economic
attribution, and station one signs the persona, not the binding. A contribution-share
lens over flow events may be added as a report only, never stored, and only after station
three lands. The tuning layer's owner Delegation, Sophia-attested agent profiles, DID
document assembly for identity heads (`did:elohim` needs an `ElohimIdentityStore`), and any
integrity-zome change are later slices.

## Execution boundaries

At most two implementation seats; scoped write sets: native seat owns `elohim/eprfs/**`
and `.eprfs/status/**`; integration seat owns `genesis/a2o/**`, `.claude/scripts/memory-kit/**`,
the two skill packages and their projections, `genesis/build-manifest.json`, `justfile`,
and the habit atom. No commits or pushes in this sprint; existing dirty edits are
preserved. Native gate: `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace`
plus fmt and clippy `-D warnings`, `EXIT=$?` echoed on its own line, cargo berth claimed
first. Integrated gate: `just gate memory-ceremony`. Independent technical review of each
report, then one fresh-agent observation with context reset for station three. Reports
land in `genesis/docs/superpowers/plans/agent-provenance-collective-affiliations/task-N-report.md`
linked through the native lifecycle (`epr flow claim` / `fulfill` / `note --kind verdict`).
