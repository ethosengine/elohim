---
title: "A governed space is a content tree behind a declared boundary — collective memory as its first instance"
id: governed-space-share-boundary-collective-memory-design
status: Draft
class: process-meta
process_subdomain: memory
date: 2026-11-26
steward: human:matthew
context-tier: disclosed
topic:
  - collective-memory
  - governed-space
  - share-boundary
  - epr-meta
  - devices
  - replication
  - holder-set
  - tending-attestation
  - memory-ceremony
  - commons-pool
refines: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md
  - genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md
serves: [acts-attributed-to-participants, memory-tended-once, devices-share-one-berth, workspace-device-arrival, agents-arrive-briefed]
graduation-trigger: >-
  a memory ceremony run once on one of the steward's three devices is accepted by the
  other two without a re-run, and an entry contributed on one device appears attributed
  on the other two with no git operation carrying it
cites:
  - "memory-search-scale-three-seams-design | the spec this refines: its content-synced-index-never invariant, the household as smallest collective, and the register row that names contribution acts across devices as an open seam | sha256:5397da36e2c29635 | path: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md"
  - "google-drive-application-design | the steward's experience note: a folder as an EPR with reach grants, and revoked reach leaving the holder's copy in place; the experience kept, its mechanism not adopted | sha256:08736bfde8e350f4 | path: genesis/docs/content/elohim-protocol/architecture/applications/google-drive-application-design.md"
  - "commons-pool-as-collective-party-design | the collective as holder scope: member draw by membership and reach, the eleven declarations a collective's space owes, and the ruling that a steward's devices are a different authority scope | sha256:803fa69bf3735eb9 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "workspaces-cooperate-as-devices-design | the peer link and channel the first instance rides: roster admission, signed heads, the fold, and the crossing allow-list this design adds a memory kind to | sha256:28296e002122f501 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - "artifact-store-sprint-2026-11-07-design | the chunk-verified blob fetch and serving scope between a steward's devices that station 5 holds contributions under | sha256:5f6d453c9944f391 | path: genesis/docs/superpowers/sprints/2026-11-07-artifact-store/design.md"
  - "verdict-is-an-attestation-on-an-artifact-design | the precedent for doing work once and letting sibling devices reuse it: closure key, sibling claim, counted in no approval | sha256:b4b0ee6c8912fcb2 | path: genesis/docs/superpowers/specs/2026-11-24-verdict-is-an-attestation-on-an-artifact-design.md"
  - "unified-memory-loop-design | the rule that competing contributions do not resolve by last writer, and that local review is not peer attestation | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "agents-arrive-briefed-plan | the approved plan that records git as today's carrier for collective memory and graduation as the transport for a lesson; the habit the collective's space serves | sha256:26a1214a7c89914a | path: genesis/docs/superpowers/plans/2026-11-22-agents-arrive-briefed.md"
  - "private-thought-governed-fruit | the constitutional line this design keeps: what was said or done may cross, thought and transcripts do not, outcomes cross as attestations | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "native-dev-plane-one-derivation-earned-reach-design | rulings D4 and D5 reused in the gradient reading: independence counts people not devices, and widening reach is a separate governed act | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
  - "verification-as-memoized-derivation-guidestar | the three axes that compose and are not merged: audience, strength of evidence, outcome | sha256:b4a0a2e087e67c12 | path: genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md"
  - genesis/data/timeline/backlog/prepush-memory-ceremony-reruns-eprfs-recall-tests.md
  - elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
---

# A governed space is a content tree behind a declared boundary

In brief: this document proposes one storage shape, the governed space, and builds its first instance for the memory that agents keep on the steward's three devices. Today that memory reaches each device by git and has to be tended separately on each one. The proposal is that a ceremony runs once, the other devices accept its result or say why not, and git stops being the carrier. Section 0 records the steward's own words; the design starts at section 1.


## 0. Provenance

The steward's words, recorded 2026-11-26:

On the general shape: "I think the shape of this problem is an Elohim protocol discipline that's going to get replicated for a lot of other use cases.. this is where the elohim protcol needs to mature, to show the way."

On draining: "when I say \"drain\" the idea is that memory.. held at that collective level, has a shared control surface at epr-meta, so something like memory-ceremony actually can act once over that surface, but it automatically syncs to each peer"

On the scopes: "as for \"matthew's devices\" yes it's shem, ethosengine and thinkpad for right now.. but \"ethosengine as collective\" is our nacient collective org that extends the simulacra/test-fixture to adam so we can model and exercise our disciplines"

On git: "yeah I think git first was a concession."

On the feel: "in my mind i was thinking this would \"feel\" kind of like google drive, where there's say one folder, and it's share boundaries.. that the  epr-meta node.. but that's where the memory ceremony would execute, but it would automatically update to all three workspaces that used it."

On the mechanism: "that's an experience note.. not necesarily how exactly it has to work note.. I'm leaning on your expertise to help us translate what would be that elohim native experience with shared governed storage spaces over our primitives."

On governance as a spectrum: "note that even bytes on a hosted drive are governed space, it's just a really low bar of governance... the governed space we're talking about 1st is a p2p-hosted drive in essence, an epr-meta layer shared between three or more stewarded devices.. it doesn't have huge governance requirements right now, but thoes reach obligations will grow, when those governed spaces go from just my governed devices to, the collective.. many of those more private memories could graduate there and be shared with any ethosengine collective developer (ie. adam the test fixture), that would want that love map of the elohim protocol to drive independently everything adam's agent invocation wolud need to participate as a dev."

On graduation as judgment: "\"never to be published\" probaly should have triggered our absolutes guard epr-meta hook, we're trying to balance intimacy/privacy and pro-social aspects of information.. what SHOULD graduate is a judgement call a sufficientyl attuned elohim agent can hopefully help with."

On the open decisions and on acts: "Can you work on each of those decisions.. and take a working posture towards each.. I would think memory acts and tending attestations might not \"cross\" but I'd imagine they'd graduate to climb the reach ladder agreements to cover more, as appropriate."

On standing: "I Matthew am the Steward of record." This was recorded the same day as an `affirmsStewardOfRecord` clause in the session's covenant, in those words.

On why now: "the reason I started talking about collective memory.. is because with the blob stores and artifact stores.. collective memory stores as a commons-like storage shape.. one of which is that collective memory store is why we're revisiting this topic. Because git being the surface is a interiem concession I think, and not necessarily the destination"

The rest of this document is the orchestrating agent's design (Claude, Fable 5.2, session 96b46344) composed from existing specs, and is a proposal until the steward accepts it.


## 1. What this document says and does not restate

A governed space is a content tree held in each holder's own payload store. Each holder signs a head naming the root it holds, and holders converge on the same root. The top `.epr-meta` node of the tree declares who holds the space, what may enter it and how it is tended. Collective memory between the steward's devices is the first instance, and the same shape is meant to serve installables, developer artifacts and content blobs.

This document does not restate the recall stages and the "content is synced, the index never is" invariant from the three-seams spec; the pool as a Collective that is party to commitments from the commons-pool spec; the device channel, roster admission and crossing allow-list from the workspaces-cooperate-as-devices spec; the artifact tier and chunked fetch from the artifact-store design; sibling reuse of a verdict from the verdict-is-an-attestation spec; or the thought/fruit line from the private-thought-governed-fruit document. It adds four things: the governed space as one primitive, the boundary node, the tending attestation, and the two holder scopes.


## 1a. Terms

- **`.epr-meta`**: the repository's existing governance file. A directory may carry one; it declares rules, measures and habits for that directory and everything beneath it. This document reuses the name for the node at the top of a governed space.
- **CID**: a content identifier, an address computed from an object's bytes, so that the same bytes have the same address everywhere.
- **Item**: one content-addressed object in a space: a contribution, an act about a contribution, or a version of the boundary node.
- **Head**: a small signed statement by one device naming the root of the tree it holds, with a sequence number that increases.
- **Peer link and channel**: the peer link is the encrypted connection between a steward's admitted devices, run by the `epr peer serve` daemon. The channel is what travels on it today: each device's signed head and a short list of permitted record kinds.
- **Crossing allow-list**: the list of record kinds permitted on the channel, `berth.channel.crosses` in `genesis/agentic/pool-policy.json`. It exists today, and a kind that is not listed does not cross.
- **Slot**: a short labelled string on a flow event, such as `measure:gate-verdict@1`. The channel reads slots to decide what a record is.
- **Holder and principal**: a holder is a device that holds a space. A principal is a person with standing. Several holders can belong to one principal.
- **Fold**: to read a sibling's new records, verify each against its address, and append them to this device's own store, marked with the device they came through. A device refuses a record whose kind its own policy does not list and answers with a negative acknowledgement ("nack").
- **Roster and chain root**: the roster is the signed list of one person's devices. The chain root is the first device in that list, which the others were authorised from.
- **Reach and ring**: reach says how far something may travel. The protocol's reach has eight rings, from narrowest to widest: `Private`, `SelfScope`, `Intimate`, `Trusted`, `Familiar`, `Community`, `Public`, `Commons`.
- **Locality**: the placement tier memory contributions carry today (`private`, `workspace`, `repository`). It says where a contribution may be placed inside one repository and is not a network reach. The scope marker of section 7 is new and sits beside it.
- **Closure and closure key**: the closure of a piece of work is everything it read that could change its result. A closure key is one address computed over all of that, so two devices holding the same inputs compute the same key. The push gate already uses one to decide whether a check has to run again.
- **Scope marker**: a field proposed here on each memory entry that records where the entry sits (section 7).
- **Crosses**: the channel spec's and the policy file's own word for a record being carried between a steward's devices. This document keeps it only when describing that existing mechanism. For what a record of memory does, it says "graduates" (section 8).
- **Tending**: the upkeep a memory ceremony performs: compacting entries, linking what supersedes what, and re-rendering the index.
- **Bin-store**: the device's store of installed tool releases (`epr bin`), which already follows a published release and reports the release it has pinned against the one it has landed.
- **Three-seams invariants** cited here: invariant 1, "The content is synced; the index never is"; invariant 6, "Demotion, never deletion; `as_of` preserved".
- **The commons-pool spec's eleven declarations**: purpose and lens; contribution and draw basis; a condition members can read; at least three consequence rungs with notice first; who may inspect draws; admission and withdrawal; explicit holdings; an amendment path; a challenge arena; a recognising parent; a ceiling on the deciding body.

## 2. The experience, and governance as a bar that rises with reach

What the person should experience amounts to four promises. There is one place for the thing. That place has a visible boundary that says who it is shared with. Tending it happens once. Each holder inside the boundary receives the result without acting, when its device is reachable.

A hosted drive and a governed space are the same kind of thing at different bars of governance. A hosted drive is governed — by its provider's permission list, its terms of service, its background jobs. This design keeps the same experience and raises the bar only as far as the space's reach requires:

| Aspect | The lowest bar (a hosted drive) | What this design raises it to |
|--------|--------------------------------|-------------------------------|
| Where it is held | One copy on the provider's servers; sharing grants access to it | Each holder holds it in their own store; sharing means becoming a holder |
| Leaving | Access ends and the content is gone for that person | The boundary stops admitting that holder's fetches and their copy remains, as the Google Drive archetype already states for revoked reach |
| Change | The latest write replaces the content, or editors merge live | Proposed: a change is a new version, and where two versions compete a person's verdict selects the one that applies |
| The sharing setting | A permission list kept by the provider | Proposed: the boundary is itself an item in the space, versioned and governed like the rest |
| Upkeep | The provider's background jobs | Tending is an act by a holder, attested, which other holders accept or refuse with a stated reason |
| Updates | The provider pushes the current state | Each device follows the head and may observe a result before applying it; a device with nothing else reachable still works |
| What the host can see | Everything stored | What enters a space is limited to what was said or done; the private-path refusal in `import.rs` and the default-deny crossing allow-list in `allow.rs` refuse private thought, transcripts and working notes |

A hosted drive is a governed space whose only governance is a permission list held by its host; this design keeps the experience and raises the bar only as far as the space's reach requires.


## 3. The problem, with measured facts

The steward works across three workspaces that are his devices: shem, ethosengine and thinkpad. On 2026-11-26 all three were exchanging on the peer link. This device's daemon showed both siblings' heads 16 to 18 seconds old, each acknowledging this device's head, with no forged, unadmitted or rollback heads and one failed fold.

A memory ceremony must currently be run on each device. The tracked outputs of a ceremony already reach every device by git. What stays per device, and the consequence of staying there:

| Thing | Where it lives | Consequence |
|-------|---------------|-------------|
| Acts that attribute and review a contribution | Each device's untracked `.eprfs/status/flows.jsonl` | On shem, 269 of 269 shared contributions read as unattributed (source: lvi arrival spec section 6.3, "the record travels, the act does not") |
| The cleanup pressure count and its reset | Derived per device from that device's own hook observations in the same sidecar; the reset is a local observation `cleanup-pressure-reset@1` | Each device believes its own ceremony is still due |
| Any record that a ceremony ran over this surface | None exists | No sibling can accept another's work; `genesis/agentic/pool-policy.json` line 124 withholds reuse for `memory-ceremony` with the reason "no step fingerprint: gate-only inputs, so no closure key can be computed" |
| The control surface | Paths in one checkout (`.epr-meta/collective.json`, `.claude/memory/.epr-meta`, the ceremony package) | Devices cannot say which version they hold |

Entry bytes travel by git. The plan approved by the steward on 2026-11-22 (agents-arrive-briefed section 1.4) records "Collective memory content | yes (attribution does not) | git". The steward said on 2026-11-26 that this was a concession and not necessarily the destination (section 0).


## 4. The discipline

Five rules define a governed space. They are the reusable part of the design and are not specific to memory.

**Rule 1: Immutable items.** Everything in a governed space is content-addressed. A receiving device verifies each fetched record and chunk against its address before storing it, as the peer link does today, so an item that does not match its address is not accepted, and a holder that serves different bytes under an address is caught by the same check. A change is a new item that names what it supersedes. Memory already works this way: a re-import of unchanged bytes is a no-op, a contribution is not re-authored by someone else (the frozen-author check in `attribution.rs` refuses it), and umbrella entries fold members out through `supersedes:`. The collective declaration itself carries a `supersedes` list of CIDs.

**Rule 2: Declared heads, and a judged choice between versions.** Each holder signs a head that names the root it holds, with a sequence that only increases; the peer link already refuses a head whose sequence goes backwards. Where two items compete to replace the same entry, the unified-memory-loop spec rules that the choice "must not resolve by last writer, file mtime or retrieval rank": the applicable version is to be selected by a verdict act, or the judgment recorded as open. That selection is proposed here and nothing performs it between devices today; station 2's probe is where this design must show it.

**Rule 3: Union between holders.** Because items are immutable, two holders converge by set union of items and of the acts about them, and their heads then name the same root. No merge of a mutable tree is needed. Two holders adding items at once is harmless. Two holders tending at once can produce competing replacements, which rule 2 leaves as an open judgment; section 9 uses a lease to avoid that wasted work.

**Rule 4: Acts are fruit and graduate; thought does not.** Contribution, verdict and lineage acts, and attestations of an outcome, are items with a reach of their own. They are born on a device and graduate to a wider space when they carry the agreement that space asks for (section 8). Transcripts, recall sessions and receipts, working notes and private memory are refused by the default-deny crossing allow-list (`elohim/eprfs/eprfs-peer/src/allow.rs`) and the private-path refusal in `import.rs`.

**Rule 5: Everything derived is folded locally.** The index (`MEMORY.md`), the rendered working files, the semantic fold and any pressure count are computed by each holder from what it holds. This design intends that none of them travels; station 5's probe will demonstrate it for the governed space as a whole.

These five rules mean that a blob, an installable and a memory space become the same shape. The boundary names the holders and the holders are the replicas. Sharing with three devices gives three copies; widening the boundary to a collective or to the commons adds holders, which is how something becomes more resilient as more people rely on it. Today those are separate mechanisms: the content blob store with custody commitments targeting 2 holders by default, the artifact tier with no replication count or garbage collection, and the bin-store with its own layout. This document does not design that convergence; it fixes the shape so the convergence has one target.


## 5. The boundary node

The top `.epr-meta` node of a governed space is the share boundary. It declares:

| Declaration | Meaning | Existing vocabulary it reuses |
|------------|---------|-------------------------------|
| Holders | Who may hold and fetch the space; one of two scopes (section 6) | Roster rows under a chain root; affiliations of a collective |
| Surface rule | What may enter; private material is refused at import by the shape of its path | `SurfaceRule` in `elohim/epr-rea/src/index.rs`; the private-path refusal in `elohim/eprfs/epr-cli/src/flow/memory/import.rs` |
| Reach bound | One bound with a sense: a ceiling for a steward's own devices, a floor for a pool | `ReachBound` in the same file; the 8-ring `elohim_epr::Reach` |
| Tending | The ceremony package by CID, the measures and bounds it answers to (for memory: `cleanup-pressure-ceiling@1`, `memory-index-bytes@1`) | The `.epr-meta` signal vocabulary and `.claude/epr-meta/measures.yaml` |
| Tending and acceptance rule | Who may tend, and under what evidence another holder accepts a tending (section 9) | Berth steward-scoped leases; the gate's sibling-reuse fences |
| Retention | Demotion with a timestamp, not deletion (three-seams invariant 6) | Three-seams invariant 6 |

The node is itself an item in the space, so a change to the boundary is a new version with a `supersedes` link. It takes effect on a device when that device holds it and has accepted the tending that introduced it. Nothing checks that today; station 4 must refuse to apply a boundary version that arrived without an accepted tending. Under this proposal each device would report the boundary version it has applied against the newest version it holds, in the way `epr bin` already reports pinned against landed.

Today `.epr-meta` cascades by directory ancestry inside one checkout and `extends:` and `max-cascade-depth` are inert. Naming a boundary by CID is new.


## 6. Two holder scopes

The commons-pool spec already states: "Matthew's admitted personal devices are a different authority scope from that collective's pool." These two scopes are different authorities and must not be collapsed.

**Scope A: one steward's devices.** Holders are the admitted devices in the steward's roster (`.eprfs/status/participants/<handle>.jsonl`, one chain-root lineage per handle; ethosengine is the chain root). The peer link admits exactly this set today and its gossip topic is derived from the chain root. Ring: `SelfScope`. This scope runs on what is built.

**Scope B: a collective.** Holders are the devices of the collective's current members, computed by joining the collective's affiliations (`.eprfs/status/affiliations.jsonl`, written by `epr flow memory affiliate`) with each member's roster. Nothing performs that join today; only `matthew.jsonl` exists as a roster. The existing declaration is `.epr-meta/collective.json`, id `collective:ethosengine/elohim`, parent `org-ethosengine`, registry reach `familiar`, whose charter says it "grants no network membership, authentication or publication authority" and that "widening a contribution needs an approving verdict from a Steward who is not its author". Its affiliations hold two Stewards and 28 Contributors; one Steward is `human:adam` with `standing: Fixture`, sponsored by `human:matthew`. A Fixture Steward is defined for simulation and does not mint real authority, so exercising scope B with Adam is an honest simulacrum and not a second real member.

An earlier draft of this document proposed that the reach ring names the holder set. That is withdrawn. The reach vocabulary spec (`genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md`) defines declared reach as the author's policy on a unit of content, with "no viewer term", "a *commitment*, not a computation", and it places custody, "who holds/replicates for whom", on a separate axis. Rule 2 of section 12a says the same thing from the other side. So the boundary declares two separate things. It names its holders directly, as a roster or as a collective. And it states a reach bound, which limits what may enter the space and how far an item in it may later be widened. For the steward's space the holders are his roster and the bound is a ceiling of `SelfScope`. For the collective's space the holders are the members' devices and the ceiling is the ring the collective declares for itself, which `.epr-meta/collective.json` records as `familiar`. The space, the heads, the acts and the attestation are the same in both scopes; who is admitted, and how much agreement a record needs, differ.

Scope B additionally waits on: admission by membership in the peer gate (the `ReachGate` trait in `elohim/eprfs/eprfs-core/src/reach.rs`; `StewardDevicesGate` in `elohim/eprfs/eprfs-peer/src/gate.rs` is the same-steward implementation), a gossip topic per collective, device-signed acts (section 8), and the commons-pool spec's eleven declarations. That spec's own reading found none of the eleven yet backed by both a declaration and a runnable probe.


## 7. Governance grows with reach

The boundary node carries few declarations when a space is held only by one steward's devices, and more are owed each time its reach widens.

| Space | Holders | What the boundary must declare | State today |
|-------|---------|-------------------------------|-------------|
| A device's own notes | One device | Nothing beyond the surface rule; held outside the repository and copied into a gitignored directory | Exists |
| The steward's space | The steward's admitted devices (three or more) | Holders by roster, the surface rule, the tending and acceptance rule; light governance, in the steward's words "a p2p-hosted drive in essence" | The first instance this document builds (stations 1 to 5) |
| A collective's space | The devices of the collective's current members | Everything above, plus the collective's charter, approval of each widening by a Steward who is not its author, acts signed by the device that made them, membership and activity unobservable from outside, and the commons-pool spec's eleven declarations (none yet backed by a declaration and a probe) | Waits on stations 6 and 7 |
| A commons pool | Custodians under commitments | The commons-pool spec; not designed here | Not designed here |

Each entry carries a scope marker that records where it sits now (this device; the steward's space; the collective's space) and, where the author said something about sharing it, what they said and why. A draft over one device's notes (section 15, decision 3) found that two further states are needed: personal, and stale. The marker describes; it does not forbid.

Whether an entry should graduate is a judgment that weighs intimacy and privacy against the good the entry does when others can draw on it. The steward's stated hope is that a sufficiently attuned elohim agent helps with that judgment.

The division of labour follows the commons-pool spec: the agent may sense that an entry is a candidate, read what its author said about it, assemble the evidence, draft the graduation, and carry the outcome. The verdict belongs to a person: the steward for graduation into his own space, and under the collective's charter a Steward who is not the entry's author for graduation into the collective's space.

Examples of what the judgment has to weigh, without ruling on them: facts that are true of one device only; entries that involve private, identifiable people; entries any developer would need.

The one floor that code enforces today is the surface rule: private material (transcripts, recall receipts, working notes) is refused at import by path shape in `elohim/eprfs/epr-cli/src/flow/memory/import.rs` (its `PRIVATE_COMPONENTS` list names `memory-kit`, `recall-executions`, `recall`, `transcripts`, `continuations`, `scratch` and `traces`), before the file is read. This is a check on naming: a private file saved under a path with none of those names would pass, so the boundary's surface rule has to do more than this before a space holds anything sensitive.

The steward's stated purpose for the collective's space is that a developer joining the collective finds in it everything their agent needs to participate as a developer without being walked through it. Adam, the fixture steward, is the first exercise of this. This ties the design to the existing habit `agents-arrive-briefed` and its plan (`genesis/docs/superpowers/plans/2026-11-22-agents-arrive-briefed.md`). The probe for scope B is that a fresh session for the fixture developer, admitted by membership and holding only the collective's space, arrives briefed and can do a first task, and can fetch nothing from the steward's own space.


## 8. What graduates, and the agreement each rung asks for

An earlier draft treated the records about a contribution as exceptions that "cross" a bar on a channel. The steward's framing (section 0) is the better model and this section follows it. An act or an attestation is an item with its own reach. It is born on a device, beside the thing it describes, and it graduates one rung at a time. Each rung says how much agreement a record must carry before it is taken to cover that rung.

The steward has stated this shape before, for build releases. Asked how many agreeing producers a release needs, he said: "let's start with one for now.. an elohim decision can stand in for a broader consensus, it's a reach ladder concern if that release becomes more broadly used.. it should be expected to start getting more agreeing produces as reach grows." (`genesis/docs/superpowers/specs/2026-11-24-verdict-is-an-attestation-on-an-artifact-design.md`, the rulings of 2026-11-25). The number at each higher rung is not set there, and it is not set here: the boundary of each space declares it (section 12a, missing node 3).

| Rung | What a record needs to cover it | Where that agreement comes from |
|---|---|---|
| The device it was made on | Nothing more: a session made it and its identifier is recorded with it. This is the session's own report and is not verified | Exists today |
| The steward's space | One of the steward's admitted devices made it, under the steward's standing consent for that kind of record | Today only the steward's affirmation as steward of record exists, given once per session. The consent for a kind of record is proposed below and does not exist yet |
| The collective's space | A signature by the device that made it, and agreement from more than one principal as the collective's boundary declares; for a widening, the charter's verdict from a Steward who is not the author | The collective's charter and boundary; with the fixture steward Adam the agreement reads as bootstrap and not as peer validation (section 15, decision 5) |
| A commons pool | More agreeing principals, as the pool declares | Not designed here |

**What the steward's rung covers.** Attribution acts, lineage acts and tending attestations graduate to the steward's space by standing consent, with no decision per record. A verdict made on one of his devices graduates too, and on his other devices it is honoured as the same steward's verdict, so he does not approve the same thing three times. This design requires that it not be counted as a second person's verdict, and station 1's probe has to show that. This corrects an earlier draft, which said a carried approval should not count as an approval at all; the gate precedent it cited in fact lets a sibling's green stand in for a local run while counting it in no quorum, and the same holds here.

**What that rests on today, and its limit.** Attribution is self-report among cooperating participants, and a record carried from a sibling is admitted by which device relayed it, with no signature per record. Among one steward's devices that is accepted: the steward's memory of his own ruling calls the grant "a working agreement, not a widening". It is not enough for the collective's rung, where a record must be signed by the device that made it (station 6).

**The clause that states the consent.** Nothing today lets a steward give standing consent for a kind of record at a rung. Graduation of a memory contribution is decided per item, and so is the graduation verb ruled for build records. The nearest existing shape is the session covenant's `variesRule` clause (`elohim/eprfs/epr-cli/src/covenant.rs`): a human party, that party's own words, the rule and the paths it covers, an expiry, and revocation by a later clause from the same party. This design proposes a clause of that shape in a space's boundary: the party, their words, the kinds of record covered, the rung, an expiry, and revocation by a later clause. "Covering more, as appropriate" is then a new clause, which is a governed act with a person's words on it.

**How it is enforced between devices.** The channel's allow-list (`berth.channel.crosses` in `pool-policy.json`, default deny, today listing `dev-berth-events@1`, `gate-verdict@1`, `dev-landing@1` and `artifact-produce@1`) is, in effect, the first such agreement: it is what one steward's devices carry for one another without asking. It is written by hand in a policy file. Under this design it becomes a projection of the clauses in the boundary, in the way `genesis/manifests/habits.yaml` is a projection of the habit files, so that the statement of consent and its enforcement cannot drift apart. Until station 4 gives the boundary a place to hold clauses, the list is edited by hand under the gate that guards that file.

**What does not graduate.** The policy file says "Recall sessions and receipts, private memory, session transcripts and every other measure never cross", and the channel spec gives the test: "Thoughts are private; only what is said or done crosses." Nothing here changes that. Transcripts, recall sessions and receipts, working notes, the private memory tier, and any index or fold stay on the device that made them. The constitution's provision for private work is that "a signed attestation of the outcome is what crosses", which is what a tending attestation is.

**Device notes.** Each device also holds the agent harness's own notes outside the repository, copied one way into the gitignored `.claude/memory/device/` by `genesis/agentic/bin/device-memory-sync`. The plan the steward approved on 2026-11-22 says "graduation into a tracked, governed surface is the only transport for a lesson." Under this section's model a note is an item born at device reach like any other, and it reaches the steward's other devices by graduating into his space, not by being copied raw. Section 15, decision 3 gives the working posture and the evidence for it.


## 9. Tending once

A ceremony runs on one device. It takes a steward-scoped lease first so that two devices do not tend at once and produce competing replacements. The devices are one principal, but separate agent sessions run on each at the same time and do not know of one another, which is what the lease is for. Steward-scoped leases already exist in berth. The lease saves work and is not a safety condition: if two tendings do overlap, both sets of items remain and the choice between them is an open judgment under rule 2.

The ceremony produces new items (compacted entries, `supersedes` links, a new boundary version if governance changed) and one tending attestation, after which the device's head names the new root.

The attestation is keyed on a closure of the governed space: the root it read, the boundary version, the ceremony package CID and the recall contract CID. This is what `pool-policy.json` says is missing for `memory-ceremony` today.

A sibling accepts the attestation under the gate's existing fences, restated for memory: the signer is an admitted device and not this device; the closure matches what this device holds; any recorded red on that closure, local or sibling, refuses; local items the attestation did not see refuse and the device tends the difference or runs itself; a change to the boundary's governance refuses reuse. Acceptance is labelled as a sibling claim. Station 2's probe must show that an accepted attestation is counted in no approval and is not treated as a witness.

The attestation is the pressure reset. Pressure is counted from records every holder has, so the count agrees between devices at the same head.

This design intends that a device can tend with no sibling reachable: the ceremony reads only items in the device's own store, and a sibling attestation saves work without being required. One existing behaviour works against that and has to be decided. A steward-scoped claim today waits for other devices to acknowledge it (`confirm_wait_s` is 10 seconds in `pool-policy.json`), and a silent device was observed to hold a landing for about 650 seconds. Because the tending lease only saves work, this design's choice is that tending takes the lease without waiting for confirmation and proceeds; station 2's probe includes a tending with both siblings silent. A sibling whose loaded policy does not list the attestation's kind refuses the record and answers with a nack; it then simply has no attestation to accept and tends for itself. That costs duplicated work and is visible as a nack; it does not make either device wrong.

Each device follows its siblings' heads: it sees an announced head, fetches the difference, verifies every item against its address, and lands it into the workspace projection. The bin-store's follow record already has apply, canary and observe modes; the governed space follow should offer the same, so a device can observe a tending result before applying it.


## 10. What exists and what is missing

| Concern | Exists today | Missing |
|---------|-------------|---------|
| N-device admission, signed monotonic heads with acknowledgement vectors, rejection of forged, replayed and rolled-back heads | Exists in `eprfs-peer` | Nothing |
| Crossing a new record kind | Policy plus a writer; no kind registry or fold handler; an event is at most 64 KiB and a fold reads at most 1024 events or 8 MiB | Memory acts carry no `measure:` slot today, so they cannot be listed; the index does not read folded sibling acts |
| Every device loading the same policy | A kind is refused and nacked by a device whose loaded policy lacks it; on 2026-11-26 this device's daemon reported its loaded policy digest differing from the file's | A status line naming the mismatch |
| Chunk-verified blob fetch between admitted devices | Exists as the artifact tier (`Manifest`, `Record`, `Blob` requests). `epr artifact put` is specific to mesh builds, but the call beneath it, `put_local_blob_with` in `elohim/eprfs/eprfs-peer/src/local_blob.rs`, stores arbitrary bytes and records custody | To hold a memory item this way: a method name for hand-contributed items with its record stored, one row in `berth.artifacts.serves`, a producer that is not a build, and a fetch consumer that does not assume a binary to install. Not checked: whether the serving reach decision treats a non-build method as private. The tier has no replication count or garbage collection |
| Serving a governed space's tree between devices | The tier's tree verbs (`Read`, `List`, `Diff`) reach only a device's own channel projection; the channel store keeps the last 16 roots. A root that is neither a channel root nor an indexed artifact is classified `Unknown` and refused (`blob_mount.rs`, `gate.rs`) | A classification for a space root in the serving scope, and serving its diffs |
| One payload store per device | Landed as code in four commits (`45f807408`, `54f065fae`, `3fdd51e6e`, `371b627dc`) after the sprint result of 2026-11-09 was written; that result document is stale. The device's payload store is named by `EPR_PAYLOAD_STORE` (default `$XDG_DATA_HOME/epr/payload`) and is opened by the peer daemon and by `elohim-storage`'s shared tier. The content `/blob` route keeps a separate root, as the artifact-store design says | The `blob-store` feature is off by default in `epr-cli`, so a default build still creates the older directory format; nothing in `elohim-storage` reads the shared tier yet |
| The collective and its members | `.epr-meta/collective.json` and affiliations | The join to rosters; admission by membership |
| Two address forms for one entry | The request address is a raw CID of the bytes and the act's resource is a different body CID form | One canonical address |
| Reach on the record | Contributions carry `Locality` (private, workspace, repository), deliberately kept apart from network reach; no level means "my own devices" | A ring on the record; the lvi arrival spec section 6.2 says "no narrower ring exists on the record yet" |
| Encryption at rest | None in `eprfs-peer`; transport is encrypted; folded events sit as plain text | Required before anything more private than today's tracked memory enters a governed space |
| Retention of folded events | Append-only, no rotation; an open design question | A retention rule |
| Roster source | The daemon reads the roster from its own checkout, so a row landed on dev admits nobody until each checkout carries it | Reading the roster from a published ref |
| Edit-time governance on device notes | The repository's edit-time hooks, including the absolutes guard (`.claude/hooks/absolutes-guard-signal.py`, policy `absolutes-guard@1` in `.epr-meta/manifest.md`), act on writes inside the repository; a write to the harness notes directory on 2026-11-26 reported "Seam guidance coverage gap: path outside repository" | An entry passes the guards when it is contributed into a space, so contribution is where an uncited absolute in a note is caught |


The rows on the artifact tier, the serving scope and the payload store are from a survey of the code made for this document on 2026-11-26; its author did not re-read every cited line.


## 11. Stations

Each station adds one capability. The order is the orchestrating agent's judgment. Stations 1 through 4 are worth landing on their own because station 5 needs all of them.

**Station 1 (plan stations GS-1 and GS-1b): Acts graduate into the steward's space, with scope markers.** Acts carry a measure and kind (attribution and lineage first), the channel's list is extended to carry them between the steward's devices, and the index reads acts carried from siblings. This station would introduce the scope marker (this device; the steward's space; the collective's space; personal; stale) for every new contribution and attribution act. A verdict carried from a sibling is honoured as the same steward's verdict and adds no second principal; this station has to show both halves. Backfill of the existing acts must be batched under the fold limits. Probe: a contribution imported on one device reads as attributed on the other two with no git operation. Depends on: nothing beyond what exists.

What a builder of station 1 can take from the code as it stands, and what is still open:

- A crossing record is a flow event carrying exactly one `measure:<id@version>` slot and one `reason:<kind>` slot (`MEASURE_SLOT` and the kind slot in `elohim/eprfs/eprfs-peer/src/allow.rs`). The allow-list entry is one line under `berth.channel.crosses` of the form measure to list of kinds, as in the existing `"gate-verdict@1": ["verdict"]`.
- Memory acts today are flow events with neither slot, which is why they cannot be listed. The measure id and its kinds are to be named in the station's plan; this document does not fix them.
- The index that must read sibling acts is the one `epr flow memory project --index` builds (`elohim/eprfs/epr-cli/src/flow/memory/index.rs`), which reads acts from the device's `.eprfs/status/flows.jsonl`. Reported by a survey of the code on 2026-11-26; UNVERIFIED by the author of this document.
- A folded record carries a `via:<device>` slot naming the sibling it came through. The proposed use: a verdict whose record has a `via:` slot is shown as carried from that sibling, is honoured as the verdict of the principal that device belongs to, and adds nothing to a count of distinct principals.
- Limits: a record is at most 64 KiB; one fold reads at most 1024 records or 8 MiB. Backfill therefore means several folds, not one. About 270 acts exist now.
- Open for the plan: where the scope marker is written (a field on the contribution or a slot on the act) and where it is read.

**Station 2 (plan station GS-2): The tending attestation.** The tending attestation and the governed space closure key. Probe: a ceremony run on one device at a head is accepted on a second device at the same head and its ceremony does not run; with a local unseen item it refuses and says why. Depends on: station 1.

**Station 3 (plan station GS-3): Shared pressure.** Pressure counted from shared records, with the attestation as the reset. Probe: the pressure line reads the same on three devices at the same head. Depends on: station 2.

**Station 4 (plan station GS-4): The boundary node by CID.** Each device reports the boundary version it has applied against the newest it holds. Probe: a boundary change made on one device is reported as pending on the others until they accept the tending that introduced it. Depends on: nothing beyond what exists.

**Station 5 (plan stations GS-5 and GS-5b): Items are the source.** Contributions held in the device's payload store, fetched between devices, with the governed space root served over the peer link, under a serving scope that refuses a root outside the asking device's boundary and with spaces that share holders mounted under one head (section 12a, missing node 5). Git retires as the carrier for memory when this station's probe passes. Probe: the graduation trigger in the frontmatter. Depends on: stations 1 through 4. Open sub-question: how a hand edit to a projected file becomes a new contribution, since today's hooks treat the file as the source.

**Station 6 (plan station GS-6): Signed acts.** Each act is signed by the device that made it, so a receiving device can check it without trusting the relay. Required before any record graduates into a collective's space. Depends on: station 1.

**Station 7 (plan stations GS-7 and GS-8): Scope B.** Admission by membership, a channel per collective, exercised with the Fixture steward Adam. Probe: a fresh session for the fixture developer, admitted by membership and holding only the collective's space, arrives briefed and can do a first task, and can fetch nothing from the steward's own space. Depends on: station 6 and on the pool declarations.


## 11a. Where this is tracked

This design belongs to the native development plane epic, whose plan is `genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md`. Its stations are claimable in a child of that plan, `genesis/docs/superpowers/plans/2026-11-26-governed-space-collective-memory-plan.md`, under the ids shown in the headings of section 11.

Six things this design needs were already promised elsewhere when it was written, and that plan gives them no second station:

| What this design needs | Where it is already carried | State on 2026-11-26 |
|---|---|---|
| A key over what the memory ceremony read (station 2) | Station CD-no-fingerprint in the epic's plan | open |
| The roster read from a published source (section 10) | Row 77 of the `steward-device-arrival-open-items` backlog | open |
| A status line when devices load different policy (section 10) | Stations Crossing-drop-seen and Daemon-policy-current in the epic's plan | partly landed |
| A method on the artifact path that is not a build (station 5) | Station K11 in `genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md` | open |
| The payload store's current format by default (station 5) | The follow-on the artifact-store sprint named E4b; no plan carried it, so it is now row 19 of the `arch-native-dev-plane-backlog` | open |
| Membership and standing at wider reach (station 7) | Concern rows 13 and 14 and the F6 stations in the epic's plan | not started |

Five concerns this design raised were carried nowhere and were added as backlog rows on 2026-11-26: the bin-store beside the payload store, a record of who carries a root, a failure domain on each roster row, and the standing of acts made through a revoked device (rows 20 to 23 of the `arch-native-dev-plane-backlog`); and encryption at rest (row 16 of the `arch-confidentiality-plane-backlog`).


## 12. Design gate

This section applies the repository's P2P design gate, a checklist every new stored thing passes before it is designed in detail: how it is classified, how it is addressed, what it costs the network to keep, and what the worst possible holder of it could see. Run 2026-11-26. No new DHT entry type and no DNA change; all four entities live on the developer plane in eprfs and the peer channel. Head-plane cost is none. Item volume is about 270 acts now and low thousands a year, with no per-item network heads. All addresses are content-derived. Every entity must behave at every declared network stage. Local-first reading: one device with nothing else reachable can contribute, tend and render, and a sibling saves work but is not required.

**Memory act.** An attribute of an already content-addressed contribution. It graduates (section 8); between a steward's devices it is carried as a channel record until station 5 makes it an item in the tree. Classification nearest Linked.

**Tending attestation.** An outcome proof over private ceremony judgment. Classification Attested-Private; the judgment stays on the device.

**Boundary node.** Content-derived. Versions linked by `supersedes`. The applicable version is the one a device has accepted through a tending (a declared dependency, not the latest).

**Governed space head.** One per holder, signed by that device, sequence strictly increasing per device, as the channel head is today. Holders are converged when their heads name the same root.

**Worst-holder test.** The gate asks what a holder seeking to dominate, and followers disposed to enforce for them, could see, join and compel (it names this the SDO and RWA test, for social dominance orientation and right-wing authoritarianism). Here: an admitted device sees who contributed what and when across the steward's devices, bounded by roster admission at scope A. At scope B, membership and activity must be unobservable from outside the collective, which the public-membership gap in the commons-pool spec does not yet give. This design intends that nothing more private than today's tracked memory enters a governed space until items are encrypted at rest. The only enforcement that exists is the import refusal of private path shapes; the boundary's surface rule has to carry the rest from station 5.


## 12a. Gradient reading

Run 2026-11-26 with the repository's gradient-reading method, beside the design gate. The method asks, for each part of an object, how far it may travel, who carries it, how stale a read may be, what holding it reveals, and who pays; and for each costly path, whether the cost is one that an existing relationship of trust should already have removed. An earlier reading of this kind was made for the native development plane and the artifact store. Its working files are in a gitignored directory that is not on this device, so this reading takes from the spec it produced (`genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md`) the two rulings that bear here, and uses the tag vocabulary of `genesis/data/timeline/CONVENTIONS.md`, which names the relationship between the two ends of a path (for example `trustful-self` for one steward's own devices and `trustful-declared` for distinct parties in a standing declared relationship) and the kind of cost paid on it.

Three words recur. A cost is **compressible** when both ends already trust one another and the cost is re-deriving something, waiting, or not being able to see: one carried proof can replace the repeated work. A path is **fused** when one part of an object is made to wait on, or pay for, another part's carrier. A **missing node** is a step that the design needs between two things that already have names and that nothing provides yet; the six found here are listed at the end of the section.

### How the dimensions compose

The five things declared below are not five independent settings. Each is fixed, in part, by the others. The order of dependence is: reach comes first and bounds custody; custody then fixes who bears the cost, what holding reveals, and how strong any evidence from the holders can be; freshness of the admission record then decides how far the bytes can in fact still travel. Two words are used strictly from here on: a **holder** is a device, and a **principal** is a person with standing. Several holders can belong to one principal.

1. **A space has several parts, and the reach of one part is set relative to another.** The bytes of an entry, the act that says who wrote it, the fact that a device is a holder, and a device's head are different things with different audiences. The method's own example is that an intimate reference to commons bytes is two reaches, not one. Under this proposal the act naming an entry's author may travel less far than the entry, so widening an entry forces a second decision about its act; it does not carry the act along. Today an act does not travel at all.
2. **Reach bounds custody; it does not produce it.** The reach bound sets a ceiling on which devices may hold a space; it lists no one. The boundary names the holders within that ceiling (section 6). Which of the permitted devices do hold it is a smaller set again, chosen to meet the space's need for resilience, and holding is opted into. So a wider ring raises the ceiling on copies and leaves the number of copies where it was. Reading reach as custody is how a thing comes to be called resilient because many may hold it while one does.
3. **The dials of custody fix one another.** There are four: how many hold (n), how many must be reachable to read (k), how independent the holders are, and how many principals must agree before a tending is accepted or an entry is widened. The method warns that any one holder sufficing to recover is any one sufficing to take over. In the steward's space k is 1, so a single device can serve everything and can sign as a holder. That is why the fourth dial cannot also be counted in devices: it is counted in principals. A low read threshold is affordable only because the act threshold is set on a different measure.
4. **Independence caps the strength of evidence.** Ruling D4 of the native development plane spec says "independence counts principals with standing, never keys or devices", and that "Device, hardware, toolchain lineage and infrastructure are declared dimensions. They explain a `Mismatch` and inform liquidity. They are never counted." (A `Mismatch`, in that spec, is two records of the same work that disagree.) Three devices of one steward are one principal, so nothing those three devices say to one another rises above a claim. Adding holders adds copies; it adds no evidence. That spec records the mistake this prevents: a count that fell back to keys, under which "one human's devices and agents would read as k = 3".
5. **Evidence and outcome gate a move in audience.** The verification guidestar (`genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md`) names three axes: who may see a thing (reach), how independently it was confirmed (claimed, witnessed, confirmed), and how the check came out (permit, refuse, refer, or not measured). They are not merged into one grade, and they are not free of one another either: a record may move outward along the audience axis only at a stated strength of evidence and with a permit. Graduation is that move. Ruling D5 says "Reach on the record itself only ever narrows, and builders cannot promote their own work", and the collective's charter sets the evidence a widening needs: a verdict from a Steward who is not the author. A tending attestation between a steward's devices sits at `SelfScope`, claimed, permit, and by rule 4 it cannot be used to widen anything.
6. **The freshness of the admission record sets the real reach of the bytes.** Bytes are addressed by their content, so a copy cannot be stale, and a stale head only names a smaller set of items. The admission record is different. A device that was revoked, or a member who withdrew, goes on being served by any holder whose copy of that record is old. So the bytes stay reachable by a departed holder until the slowest holder's admission record is current. The more holders custody chooses, the more copies of the admission record there are to bring current, and the longer that tail.
7. **What a space reveals grows with its custody.** Holding shows that a device is a holder. Serving shows the serving holder which device asked for which item. Announcing a head shows the other holders when a device is active. Each of these is seen by holders, so every holder added for resilience is another party that sees them. This is the same reason the method prefers independence to headcount: two holders in two failure domains reveal less, and cost less, than five in one.
8. **Cost follows custody; benefit follows reach.** The holders pay to carry a space. Those the ring admits benefit. Because rule 2 separates the two sets, a widening that names no new holder has widened the benefit and left the cost with the same devices.

### The declaration, part by part

Each part is declared in the order of dependence: its reach; the custody that reach permits and the space's need selects; what follows from that custody for evidence, exposure and cost; and the freshness that applies.

**Bytes** (contributions and boundary versions), for the steward's space.
- Reach: `SelfScope`. The devices that may hold are the three on the steward's roster.
- Custody, chosen within that reach: the need is to survive the loss of any one device and the loss of the cluster. The channel spec describes the two Che workspaces as "pods in one cluster", and the third device is a laptop, so only a set that includes the laptop reaches a second failure domain. All three hold: n is 3, in at most two domains. k is 1, which is safe for reading because bytes are addressed by their content, so any one copy is correct. Nothing on a roster row declares a failure domain (the row's fields are `authorizedBy`, `chainRoot`, `cid`, `controller`, `handle`, `kind`, `recordCid`, `row`, `sigA`, `signature`, `signer`; read from the first row only, so UNVERIFIED for other row kinds), so the two domains are known from prose and not from a record.
- What follows for evidence: the three holders are one principal, so by rule 4 no agreement among them is more than a claim, and by rule 3 the act threshold is one person, the steward.
- What follows for exposure: every holder is the steward, so holding reveals nothing outside him. A fetch shows one holder what another asked for.
- What follows for cost: three disks carry it; every session on those three devices benefits.
- Freshness: none to manage, for the same reason k can be 1: a device serves what it holds and a receiver checks it against its address.

**The admission record** (the roster, the affiliations and the boundary node), which every other part depends on through rule 6.
- Reach: the roster and the affiliations are tracked files today, readable by anyone who has the repository. For the steward's space that is wider than the reach of the bytes they admit to. This is accepted for the first instance because the roster carries device keys and no content. It cannot stand for the collective's space, where the design gate requires that membership not be observable from outside (missing node 6).
- Custody: carried by git today, which is the fused path read below.
- Freshness: must be current, and by rule 6 it is this part's freshness that decides how long a revoked device keeps its access to the bytes. It is the highest-stakes read in the design and is not one to cheapen at any declared network stage.
- What follows for cost: the steward bears enrolment; the stewards who review bear each widening.

**Heads** (each holder's signed head, and the choice between competing replacements).
- Reach: holders only; the channel takes only admitted devices as gossip neighbours.
- Custody: each holder keeps its own, because a head is a statement about what one device holds and no other device can make it.
- What follows for exposure: by rule 7 the cadence of a device's head shows the other holders when it is active, and shows traffic timing to a relay where one carries the connection. On 2026-11-22 the third device was reached only through a relay.
- What follows for cost: every device announces its head every 30 seconds and after each change. If each space had its own head, this would multiply by the number of spaces (missing node 5).
- Freshness: a stale head is safe to read. The choice between competing replacements is a judgment with higher stakes, since a wrong choice briefs agents from the wrong version.

**References** (acts of attribution, lineage and review; `supersedes` links).
- Reach: set relative to the bytes they describe (rule 1), and may be narrower.
- Custody: on one device only today, which is why 269 of 269 contributions read as unattributed on shem. As channel records at station 1; as items in the tree at station 5, where they take the custody of the bytes.
- What follows for exposure: an act names the device and session that made a contribution. Together the acts are a ledger of activity. The design gate says such a ledger is held by the group it describes; rule 7 says each added holder is another reader of it.
- Freshness: a missing act reads as unattributed, which is incomplete and not wrong.

**Custody as a record** (which device holds which root).
- Read from the head table: holders have converged when their heads name the same root. `freshness_s` in `pool-policy.json` is 90 seconds, the age after which a device with no fresh head reads as silent. It rides the head, so its reach, exposure and cost are the head's.

**Projections** (the working files, `MEMORY.md`, the semantic fold, the pressure count).
- Reach: the device that computed them; they do not travel. Custody is therefore one device, exposure is nil, and the cost is that device's compute once per fold and one embedding model per device.
- Freshness: lags the items by one fold. The stake is an agent briefed from a stale index.

**Attention** (recall sessions and receipts; the record of which device fetched which item).
- Reach: private to the session or to the serving device. Recall paths are refused at import by path shape. The artifact-store design records each serve as a `serve-blob` use event that does not cross. This is the part that would show what a person or an agent read, so rule 7 is answered here by giving it no holders beyond the one that made it.

**Value** (records of carrying and drawing).
- The artifact tier records a use event when it serves. Nothing records that a device carries a space, so rule 8's cost bearer cannot be read back today (missing node 4).

**Notary**: none. This design writes no DHT entry at any rung it covers.

### One entry, followed upward

A worked case shows the dimensions moving together. Take a note written on shem that records how the steward wants push authority handled. It is true for him on every device and would help any developer.

| | On the device | After graduation into the steward's space | After graduation into the collective's space |
|---|---|---|---|
| Who gave the verdict | none needed | the steward | a Steward who is not the author |
| Reach of the bytes | one device | `SelfScope` | the collective's ring |
| Devices that may hold | 1 | 3 | the members' devices |
| Devices that do hold | 1 | 3, when each follows the head | those members who opt in; possibly still 3 |
| Failure domains | 1 | at most 2 | more only if a member's device is elsewhere |
| Strength of evidence | a claim by one session | still a claim: three copies, one principal | witnessed only if a second real person reviewed; with the fixture steward Adam it stays a claim |
| Reach of the authorship act | the device | `SelfScope` | a separate choice under this proposal; it may stay at `SelfScope`, so members see the entry without the session that wrote it |
| What holding reveals | nothing | nothing outside the steward | to other members, who is a member and when each contributes |
| Who bears the cost | one disk | three disks | each device that opts in |
| Tail after a revocation | none | until three rosters are current | until every holder's admission record is current |

Three things did not move at either step: the address of the bytes, the private material around the note (its session's transcript and recall receipts stay on shem), and the projections, which each new holder computes for itself. Two things that might be expected to move together did not: the number of copies did not rise because the reach did (rule 2), and the strength of evidence did not rise because the number of copies did (rule 4).

### How the declaration changes as reach widens

| | A device's notes | The steward's space | The collective's space |
|---|---|---|---|
| Relationship between the two ends | none | one steward's own devices; nothing is left to earn (the vocabulary's tag for this is `trustful-self`) | distinct people in a standing declared relationship, by charter and sponsored affiliation (tag `trustful-declared`) |
| Ceremony owed on a graduated record | none | re-derive the closure; admission by roster; no signature per record, accepted | a signature by the device that made each act; a verdict from a Steward who is not the author for each widening |
| Holders, read threshold, independence | 1, 1, one domain | 3, 1, at most two domains, one principal | the members' devices, 1, and as many principals as there are real members; with the fixture steward Adam that is still one |
| Act threshold | the steward | the steward | two people for a widening |
| What holding reveals | nothing | nothing outside the steward; traffic timing to a relay | membership and activity to other members; this must not be observable from outside |
| Who bears the cost | the device | the steward's three devices | each member's devices, to the extent that member commits |

A commons pool sits beyond this table. Its two ends are people whose standing was earned on the commons, or strangers, and the full ceremony belongs there.

### The measured paths, read

The method keeps a measured ranking as measured and does not re-rank by cost. The only figures below are those already recorded elsewhere.

| Path | The two ends | Kind of cost | What is paying | Multiplies by | Reading |
|---|---|---|---|---|---|
| 1. The ceremony is re-run on each device | one steward's devices | re-deriving a result one end already holds | projection and reference work | once per device | Compressible: carry one attestation per closure (station 2) |
| 2. Contributions read as unattributed on a sibling, 269 of 269 | one steward's devices | cost that cannot be attributed | the reference part has no carrier | per item | Compressible twice. Station 1 carries acts one by one. At station 5 the acts are items in the tree, so one signed head carries all of them: per item becomes once per root |
| 3. A steward-scoped claim waits for acknowledgement (10 seconds configured; about 650 seconds observed with a silent device) | one steward's devices | waiting for agreement | authority | per claim | Compressible: the tending lease does not wait (section 9) |
| 4. The roster and the boundary are read from a checkout | one steward's devices | waiting | authority riding the carrier of bytes | once per change, per device | Fused: an admission decision waits on which branch a checkout is on. Separate them (a published ref; station 4) |
| 5. A memory entry pays the code landing gate to reach a sibling | one steward's devices | re-verification and waiting | small self-certifying bytes charged the code landing ceremony | per push | Fused. The backlog entry `prepush-memory-ceremony-reruns-eprfs-recall-tests` records one push in which the memory-ceremony gate was still re-running recall tests 35 minutes after the eprfs gate had passed them in 5114 seconds. Station 5 removes memory from that path |
| 6. A device whose loaded policy differs refuses a kind | any | plain engineering | the contract between devices | per record | Owed whatever the relationship. A status line; not a trust question |
| 7. The pressure count differs per device | one steward's devices | cost that cannot be attributed | a projection computed from observations local to one device | per device | Compressible: count from shared records (station 3) |
| 8. A fold fails | any | plain engineering | the channel | per fold | Owed: count it and surface it |

Paths 1, 2, 3, 4, 5 and 7 are compressible because both ends are the same steward and the cost is re-derivation, waiting or blindness. Paths 6 and 8 are owed. How long one ceremony takes on one device is not recorded in anything this document read, so the saving from station 2 reads unmeasured until its probe records it; the native development plane spec states the standard: "A rung with no measured saving reads **unmeasured**, never \"working\"."

### Guards applied

- **Ceremony is compressed, not skipped.** No station weakens a signature, an admission or a reach check to go faster. Where the relationship changes from one steward's devices to distinct people, the ceremony is added back: a signature per act and a second person's verdict.
- **Resilience is independent failure domains.** The steward's space is three copies in at most two domains under one person. That is the honest figure for the first instance.
- **Serving has to enforce scope.** The native development plane spec required that an admitted device asking for an object outside its grant be refused. A space's root must be classified by the peer link's serving scope in the same way, with the same test, before station 5.
- **Separation has a price.** Keeping an act's reach narrower than its entry's means some holders see an entry without its author. Keeping projections local costs a fold on every device. Encrypting items at rest, when it comes, adds the handling of keys.
- **No new register.** This reading adds no tag to any source and no index; it lives in this section.

### Missing nodes

Each is stated as the chain it belongs to, the two named points it sits between, the node, its probe and its present state.

1. Chain: a space's resilience. Between "a roster row admits a device" and "the space counts independent failure domains". Missing node: each device declares the failure domain it runs in. Probe: status for a space prints its holders and the number of distinct domains. Present state: no such field on the roster row read.
2. Chain: the ring on the record. Between "a contribution's placement tier" and "the set of devices that may fetch it". Missing node: a ring on the contribution. Probe: an item marked for the steward's space is refused to the device of a collective member. Present state: absent; the lvi arrival spec says "no narrower ring exists on the record yet".
3. Chain: the act threshold. Between "a widening is signed" and "a holder applies it". Missing node: the boundary declares how many distinct people must agree. Probe: a widening carrying only its author's verdict is not applied on any holder. Present state: the charter says it in prose; nothing checks it between devices.
4. Chain: the cost of carrying. Between "a device holds a space" and "the steward can read who carried what". Missing node: a record that a device carries a space, written beside the use record the artifact tier already writes when it serves. Probe: a status command for a space lists, for each holder, the bytes carried and since when. Present state: absent.
5. Chain: one head for many spaces. Today each device signs one head over one tree, its channel tree, and announces it every 30 seconds. If every space had its own head, three devices sharing five spaces would announce fifteen heads in each interval where they now announce three. Between "a set of holders shares several spaces" and "the number of announcements stays one per device". Missing node: the device's one head names a root under which each space it shares is a subtree, as `channel/events` is a subtree of that root today. Probe: create a second space for the same three devices; each device still announces one head per interval. Present state: the channel tree has no place for a space.
6. Chain: the reach of the admission record. Between "a collective lists its members" and "membership is not observable from outside the collective". Missing node: the affiliations and rosters of a collective's space are held inside that space, at its ring, and not in a tracked file. Probe: a reader with the repository and no membership cannot list the collective's members. Present state: `.eprfs/status/affiliations.jsonl` and the roster are tracked files.

## 13. Concern canon

| Class | State | Note |
|-------|-------|------|
| Plane location | answered | Developer plane, the peer channel and the device payload store; not the DHT, not the deployment manifests |
| Anti-self-election | answered by copy | A device does not treat its own attestation as independent (station 2's acceptance fences refuse self-attestation); an author does not approve their own widening (charter rule) |
| Monotonic authority | partial | Heads refuse rollback by sequence only (the peer link's head-rollback refusal); no content-level check on roots. Owed at station 5 |
| Liveness | partial | Intended: a device tends with no sibling reachable. Today a steward-scoped claim waits on acknowledgement and a silent device held a landing for about 650 seconds; section 9 chooses a tending lease that does not wait, to be shown by station 2's probe |
| Honest absence | partial | With no per-row signature check, unverifiable reads the same as accepted; absent, silent, refused and unverifiable need distinct readings. Owed at station 2 for attestations and station 6 for acts |
| Evidence not authority | partial | A carried verdict is honoured as the same principal's and adds no principal (section 8, station 1); it cannot be checked without trusting the relay until station 6. Answered for the attestation, whose closure the receiver re-derives |
| Bounded work | partial | Fold limits exist; the first backfill needs batching (station 1) |
| Idempotent effect | answered | Append by address; re-import is a no-op |
| Advertise equals serve | partial | Policy must match on every device; mismatch observed live. A status line is owed at station 1 |
| Observability per decision | partial | The channel counts drops and nacks; acceptance and refusal of an attestation need a labelled reason line like the gate's (station 2) |
| Identity lineage | unbound | Nothing says what becomes of attributions made through a device later revoked. No station answers it yet; it must be answered before station 7 |
| Contract evolution | answered | An unknown kind is refused and nacked by the peer link |
| Backpressure | n-a | The 64 KiB event cap means item bytes ride the blob path, not events |
| Consent and authorization | partial | The steward's standing consent covers acts and attestations at his own rung (section 8); the clause that states it has no home until station 4; graduation of notes is a judgment (section 7) |
| Graduated authority | answered | Git is named as a scaffold with station 5 as its successor |
| Witnessed residual | partial | A failed fold must be counted and surfaced; one was showing on 2026-11-26 (station 1) |


## 14. Habit

Habit `memory-tended-once` was declared on 2026-11-26 on the steward's choice, in `.epr-meta/memory-tended-once.habit.md`. It is born `unwired` and inactive: the register is committed to it and has no way yet to observe whether it is kept. Its invariant is that a memory space shared by several devices is tended once per head, that each other holder accepts that tending or says why it did not, and that the count which says a tending is due reads the same on every holder. Its planned check is an a2o scenario (the repository's acceptance scenarios) under the tag `@concern:memory-tended-once`, built from the probes of stations 2 and 3. It retires when the governed-space shape serves a second kind of content and the invariant is restated for spaces in general.

The other stations serve habits that already existed: `acts-attributed-to-participants` (stations 1 and 6), `devices-share-one-berth` (stations 4, 5b and the two-root test), `workspace-device-arrival` (station 5) and `agents-arrive-briefed` (the scope marker and the collective's space). The register allows two habits to be active at once, and this one is not one of them.


## 15. Decisions, each with a working posture

The steward asked for each open decision to be worked and given a working posture. A posture is what the stations build to unless he corrects it. None of them is his ruling.

**1. How acts and attestations reach other devices.** Posture: they graduate (section 8). In the steward's space, attribution, lineage and tending attestations graduate by his standing consent, and a verdict is honoured as his own on his other devices. Basis: his framing of 2026-11-26; his reach-ladder ruling of 2026-11-25 for releases; the session covenant's clause shape. What it still needs from him: the clause in his own words when the boundary can hold one, since a clause records a person's statement and not an agent's.

**2. Whether the reach ring names the holder set.** Posture: no; withdrawn (section 6). The boundary names its holders directly and states its reach bound separately. Basis: the reach vocabulary spec, which gives declared reach no viewer term and puts custody on its own axis. A consequence: the collective's space has a ceiling of `familiar`, because that is what its declaration says. The withdrawal is the document author's, on reading that spec. Left for the steward: to say so if he did want rings to stand for sets of people, which would mean amending the reach vocabulary and not only this design.

**3. Whether device notes reach the steward's other devices.** Posture: they do not travel raw; they graduate by judgment, the agent drafting and the steward giving the verdict (sections 7 and 8). To test that posture, a reader drafted a scope for every note on this device on 2026-11-26. It is a draft for a person to rule on, and the reader noted where it was torn.

| Proposed scope | Notes | Meaning |
|---|---|---|
| The collective | 57 | would help any developer's agent take part: design rulings, vocabulary, engineering traps, how the tooling behaves |
| The steward | 36 | true for him on any of his devices: how he wants agents to work with him, his standing rulings |
| This device | 23 | true of this workspace or of one moment on it |
| Stale | 13 | a dated grant or sprint whose window has passed |
| Personal | 6 | about the steward's own life, or identifiable private people |
| Total | 135 | |

What the draft shows:

- 93 of the 135 notes on one device would be of use on the steward's other devices or to another developer, and none of them leaves this device today. Each of his other devices holds a different set.
- The note's existing `type` field does not predict its scope. If each type is given the scope most of its notes received (collective for `feedback`, 49 of 90; stale for `project`, 12 of 37; this device for `reference`, 2 of 3; personal for `user`, 3 of 5), the default is right for 66 of 135 notes. Notes of type `feedback` split three ways (49 collective, 28 steward, 12 device). So scope cannot be defaulted from type; it has to be marked when a note is written and judged when it graduates.
- Three scopes are not enough. The marker of section 7 needs two further states: personal, which stays with the steward unless he says otherwise, and stale, which is retired and not graduated.
- At least fifteen notes were marked as a device or steward note with a general lesson inside it. For those, graduation is not moving the note: it is writing a new entry that carries the lesson, linked to its source, which is rule 1's "a change is a new item".
- A mechanical match for "always", "never", "must never" and "guaranteed" in each note's description and its how-to-apply text finds 53 notes. Read by hand, about 45 of those name nothing that enforces the rule; the reader did not check each of the 53 one by one, so 45 is an estimate. These were written outside the repository, where the absolutes guard does not run (section 10), so contribution into a space is the first point at which that guard can see them.
- No note says anything about its own scope, apart from one that the steward's remark of 2026-11-26 caused to be reworded.

What it still needs from him: his verdict on the draft, note by note or by group, when station 1 gives the marker a place to live. The draft table itself is held in the session that produced it and is not part of this document, because it names personal notes by their titles.

**4. Whether the developer plane and the application plane share one primitive.** Posture: one definition, two carriers for now. The content tree, its boundary node and the reach gate are defined in `eprfs-core`, which both planes may already depend on. `elohim/elohim-storage/src/boundary.rs` states the rule: "This crate may depend on `eprfs-core` and `eprfs-serve` ... never on `eprfs-storage` or `eprfs-local`", and the eprfs adapter crates "never depend on `elohim-storage`". The steward's devices carry spaces over the peer link. The dataplane takes them up when a boundary widens past a household, through the adapter `elohim-storage` already has for eprfs roots, which is written and not yet wired to any route. Basis: the code survey of 2026-11-26. A consequence for station 5: the first instance holds memory items on the existing artifact path, which needs four small additions (section 10) and no change to the tier. Left for the steward: nothing has to be decided now. The choice returns when the first space widens past a household, because that is when the dataplane has to carry one.

**5. Whether Adam's fixture standing is how the collective's rung is exercised.** Posture: yes, and its results are read as bootstrap. Basis, from the code: a fixture person can have a roster and devices of their own by the ordinary enrolment commands and a second device key; a fixture Steward's verdict does satisfy "a Steward who is not the author", and is labelled "bootstrap (fixture co-steward)" with the text "this is not peer validation"; a fixture cannot found a collective, sponsor a standing Steward, or stand as steward of record; and the repository's rule on simulacra says Adam's "simulated memberships and grants remain facts of that world". Three things to do first: no test today runs daemons under two chain roots, so that harness is the first step; Adam's affiliation line is unsigned and is pinned to an earlier version of the collective's declaration, which should be checked before it is relied on; and the charter grants no network membership, so a gate that admits by membership has nothing to read until the collective's boundary declares it. The first two are engineering work. Left for the steward: whether Adam, as a fixture, is to have devices enrolled in his name on real workspaces, and the collective's boundary declaration, which is the members' to make and not an agent's.

**6. What was declined on 2026-10-01 as "a git coordination branch".** Closed by the steward's words of 2026-11-26: git as the carrier was a concession and not necessarily the destination.

**7. This session's purpose.** Posture: none can be taken for him. The session's covenant holds the steward's affirmation and asks one further question: "What is this session's purpose, and which paths may it write?" A covenant clause records the party's own words, so an agent cannot answer it. What the session has in fact done is design this governed space and write this one document, which is offered as a description for him to accept or replace.


## 16. Non-goals

This document does not design: convergence of the content blob store, artifact tier and bin-store; custody temperatures and tier movement; erasure coding; encryption design; search between holders; the pool's economics; live co-editing; anything on the DHT.
