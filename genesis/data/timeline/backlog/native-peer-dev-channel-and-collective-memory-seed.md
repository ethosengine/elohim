---
id: "backlog-native-peer-dev-channel-and-collective-memory-seed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Campaign seed: a private, reach-governed native channel between the operator's own devices, carrying jobs, conversation and collective agent memory"
slug: "native-peer-dev-channel-and-collective-memory-seed"
written: "2026-10-01"
author: "claude-opus-5-5 (recording the operator's direction; not a design)"
status: "open"
priority: "high"
area: "devspace peers / rakia / ark / berth / brit / agent memory"
domain: "protocol"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:reach-enforced-everywhere"
cites:
  - genesis/docs/superpowers/sprints/2026-10-01-campaign-1.4-closing-definition.md
  - genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md
  - genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md
  - genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md
tags: [seed, design-session-input, devspace, peer-channel, jobs, collective-memory, reach, device-binding, trustful-self]
---

# Seed: the native peer dev channel

This records what the operator asked for on 2026-10-01, in the order they gave it. It is input to a
design session. It proposes no entry types, routes or protocols; those go through the P2P design
gate first.

## The need

There are now two Che workspaces: one on ethosengine, one on shem (running on the x86-64-v2 image
flavor). Today the operator carries every message between them by hand.

> "right now, I'm going to have to facilitate all cross communication between workspaces, we need
> a matthew (me) private reach-governed elohim native peer dev communication channels.. pretty
> quickly on top of these device registration epr-authorization grants"

## What it rests on

Device registration and EPR authorization grants, as settled by campaign 1.4. A channel private to
the operator is a channel whose members are devices bound to the real Matthew. The channel is the
first real consumer of that work, not a separate identity system.

## What rides it

The operator's direction is that this is one concern, not several:

- **Conversation** between agent sessions on their devices.
- **Jobs.** Revisit with rakia, ark and berth: "we should be able to post those jobs, alongside the
  communication channel".
- **Brit.** Pull it in: the tree with root integrity anywhere on the network is what the peers are
  collaborating over.
- **The native agent memory harness.** "this would be a 'collective' memory graduation": memory
  that today is local to one workspace becomes shared between the operator's devices.

## Sequence the operator set

1. Campaign 1.4 closes: household mechanism proof, one push, the real Che device on the fleet.
2. This campaign is designed and built.
3. The shem workspace updates to head and launches a conductor under a witness-authorized grant.
4. Collaboration between the two workspace peers drives the shakeout.

The shem workspace is deliberately not a second device in campaign 1.4.

## Constraints already on record, to carry into the design

- **Fixtures and agents are an attribution boundary** (reach solutioning brief, §6a, 2026-10-01).
  Messages and job results between the operator's devices are agentic labor accountable to their
  stewardship. Each should carry which device and which agent session produced it, so it never
  reads as a second human.
- **Thoughts are private; only what is said or done is judged** (constitutional ruling,
  2026-09-10). Private stores are never imported. A collective memory graduation has to say what
  crosses between devices and what stays private to a session.
- **Secrets never cross.** The credential contract already forbids copying keys, keystores or
  signing credentials between devices.
- **Device revocation.** Withdrawing a device must end its place in the channel without touching
  the human or the other devices, as the credential contract requires for publishing.
- **shem is CPU-constrained and shared.** Its 24 older threads also host fleet conductors that
  request an eighth of a core each. Jobs posted to the shem workspace can starve them; the
  workspace devfile is capped at CPU limit 8 and request 2 for that reason.

## What exists to compose, not yet audited for fit

- The peer-executed stage (`just measure <feature> --on <peer>`): one peer sends work to another
  and receives the completion.
- Berth leases and moorings, ark envelopes, rakia manifests.
- Private reach in the reach vocabulary.
- The governed recall session and memory import path in `epr flow memory`.
- The lvi devspace peer-runtime spec.

A quick search of the zomes on 2026-10-01 found no existing peer-signalling or message entry
type. That was a search, not an audit.

## Open questions for the design session

1. What is a message, a job and a memory entry in the P2P classification: notarized, linked,
   private, attested-private or ephemeral? They need not share one answer.
2. How does a receiving device prove that the sender is a device bound to the same human, and at
   what cost per message?
3. What is the head-plane cost of a channel that carries agent conversation at working volume?
4. What does "collective" memory mean between devices of one human, as against between humans in
   a collective, and does the first generalize to the second?
5. What replaces the operator as courier in the meantime, if anything? He declined a git
   coordination branch.

## The operator's observation: convergence is already a rudimentary channel

> "we get the ability to perform atomic updates to the peer network that converge in this next
> push.. that.. is a rudementary communicatino channel itself."

A root the operator authors, updated on one device and converged on the other, is a message: it
is signed by a device bound to them, it carries its own history, and delegation decides who else
may write to it. Once campaign 1.4's push lands, this answers question 5.

Its limits, so the rudimentary form is not mistaken for the finished one:

- **Not private.** Content is readable by the peers that hold it; the confidentiality plane is
  unbuilt, and HTTP reads through a doorway currently return content to any authenticated caller
  regardless of reach (`http-reach-enforcement-gap.md`). Suitable for coordination that could be
  public; not for anything sensitive.
- **Slow.** The convergence deadline is 75 seconds per update.
- **Every message is a head.** Volume is a head-plane cost.

This reframes the campaign: it may be mostly about what to add on top of converging roots
(privacy, latency, volume) and little about inventing a transport.

## The operator's direction, 2026-10-26: a dev release channel whose candidate heads earn reach

Prompted by the canonical household's install bundle looking lost: no bundle on disk seemed to
reproduce its five cells, so `join-peer` could not cast a new peer onto them. It was not lost. The
comparison hashed each bundled `.dna` with `hc dna hash`, which ignores the hApp role's modifiers
(here `properties: {progenitor_pubkey: null}`). Repacked with those modifiers, a bundle left in an
unrelated worktree (`land-self-heal`) matched all five; the household's own `environment.sh` names a
different, non-matching one. The point stands: the network could not say which bundle it runs, and
finding it took a hand search across worktrees.

> "I feel like we should be able to... see the bundles FROM the network, and get the bundles' we're
> deployed on.. when we're building/updating, we're building snapshot bundles that become options
> for our local device-level dev channel, and become visible to the dev, peer-device channel, since
> ethosengine and shem are matthew's stewarded peer devices.. something like that."

> "building.. and earning reach for those channels.. (up the trust curve), give's the opportunity
> for the devs in different workspaces to elect candidate heads to coordinate on..."

How it composes with what exists. This is a mapping, not a design; the P2P design gate runs first.

- **The channel is already specified.** `2026-09-01-runtime-artifacts-elected-content-design.md`
  §3: `runtime:<class>:<network>:<channel>`, reach-scoped from birth, a staging-to-earned election,
  and promotion up the reach ladder as a ceremony act. A self-reach `happ` (and coordinator,
  binary) channel whose members are the operator's stewarded devices is the ladder's bottom rung,
  not a new system.
- **Every build is a candidate.** Each local or CI build publishes its bundle as a content-addressed
  blob plus a release manifest, as a staging version. Each install records the channel version it
  used, so a running peer's DNA hashes resolve to the exact bundle. A network then cannot lose the
  bundle it is running.
- **Workspaces elect heads by attestation.** Another device adopts the candidate in
  `release_adoption`'s `canary` mode, soaks it (gates, household stories, live use) and signs a
  SoakAttestation. A finding is a contest with evidence. Attestations from distinct devices that
  meet the threshold promote the head to earned: a head the workspaces coordinate on. The R3
  follow-ups of 2026-10-26 ran this loop by hand: shem verified R3 live, reported four findings over
  a bridge message, and ethosengine fixed and announced them.
- **Gate evidence becomes shareable.** Gate reuse (`GATE_EVIDENCE=reuse`) refuses another signer's
  attestation, so each workspace pays for the same closure. A device's green gate attestation,
  counted as a soak attestation on the candidate, would let one closure pay once across the
  operator's devices.
- **The bootstrap caveat may dissolve here.** The spec's §6 caveat says a fresh joiner cannot
  resolve the channel that supplies its DNA. Carried on the eprfs peer channel between devices bound
  to one steward, which needs no shared DNA, it can.
- **Limits kept.** Adoption is consent, and a head never widens its own reach. Promotion beyond the
  steward's devices is a ceremony act. No single device is terminal authority; contest and revert
  stay open to any member.

Recovered 2026-10-26: the matching bundle is kept beside the household at
`household-dowell/artifacts/installed-20261007/elohim.happ` (sha256 b4f86c90…). The conductor's own
wasm store is SQLCipher-encrypted, so reading code back from a running peer needs the conductor.

**The channel is an EPR, and it governs stories (operator, 2026-10-26).**

> "and these channels.. are they eprs? because they become the governing artifacts for the
> "stories" of features that are being goverened by those releases."

They are. The spec makes a channel a content identity under the existing `Content` entry type, with
releases as its versions and a declared canonical head. The operator adds that a release manifest
cites the stories, the a2o features and their `@concern` tags, whose behaviour it governs. Attesting a
candidate head then means those stories passing on that exact head, and reach is earned by story
evidence. Slice M's serving receipt is the hand-run form: pre-push refuses the slice until its two
dataplane stories pass on its exact source. Open question for the design gate: whether the release
cites stories, the stories cite the release, or a habit's `checks:` joins them.

**When the design gate runs (operator, 2026-10-26).** After R2 of the restart-to-connected sprint,
on one entity. R2 adds no release head: "published" is git ancestry read from the device manifest's
`publishedRef` (`genesis/orchestrator/manifests/runtime/workspace-device.manifest.json`, checked by
`require_published` in `elohim/eprfs/device-runtime/src/bin_store/source.rs`), and the channel later
replaces that one field. The 2026-09-01 spec's §5 already answers classification, entry type,
head-plane cost and identity for the channel itself. The gate's slice is what is new here: the edge
by which a release cites the stories it governs, and whether a device's green gate attestation
counts as a soak attestation on a candidate. If R2 turns out to need its own notion of a release
head, the gate runs before R2 instead. The conversation, jobs and collective-memory half serves no
declared habit yet and waits for one.

**P2P design gate, first pass: the room (2026-11-18).** Run on the operator's approved plan, with
the `p2p-design-gate` skill, before any design. It proposes no table, route, entry type or zome
change. The `gradient-reading` and `local-first` companions were not loaded in full; their headline
questions are answered under Design constraints and a full pass is owed before a spec.

*The first social consumer (operator, 2026-11-18).* Another developer's onboarding is an act of
collective sensemaking: what do their sessions inherit when they initialise the workspace? The
scenarios are in `genesis/a2o/features/devflow/collective-memory.feature` (a second developer can
tell whose each inherited memory is; a newcomer's correction is declared as theirs when written;
graduation by another steward's verdict; two stewards' directions disagreeing; no inherited private
picture of another person).

*Where this story leads, as a breadcrumb only (operator, 2026-11-18).* The developer scenarios
are one early flavour of a longer path: what onboarding with the protocol is like for a person.
Today it is a developer starting an lvi workspace, and every participant is in effect "matthew",
because there is one. The path it graduates into is human participation in the network's own
evolution: the protocol getting to know someone; their becoming a contributor and becoming known;
their contributor presence, where one already exists, being reconciled to them; and their coming
to understand their relation to the network and to their communities. Establishing a person's
identity durably on the network and reconciling them to themselves and their communities is a
long piece of work and is deliberately not pursued now. What stands in for it is the minimum: an
agent-written memory entry carries the steward of record of the session that wrote it, read from
that session's envelope on the device. When the longer path is taken up, that stand-in is the
first thing it replaces.

Entity: **a memory entry that was said and crosses between one person's devices**
- Classification: Private (B). It belongs to one person and no peer validates it. It is held today
  as a file in the repository and would be held as a leaf in the authoring device's channel tree,
  which is eprfs and not a Holochain private source chain; the gate's category fits by its test
  and not by its storage.
- Head-plane cost: none. It is never a head, a Kad record or an election candidate.
- Network stakes: behaves the same at every stage. Floor-protected: a correction reaches the agent
  it corrects and is not filtered by any stage (the CounterEvidence floor).
- Address: content-derived. The channel already names each leaf by the CID of its canonical
  dag-cbor bytes.
- Transport affinity: iroh, the tier the device peer already serves.
- Source of truth: the entry as its author wrote it. The index and the semantic fold are
  projections rebuilt on each device.
- Integrity zome and DNA-hash class: none; DNA-hash-neutral. Coordinator function: none.
- Projections: the `MEMORY.md` index and the per-device semantic fold. No SQLite table, no
  Automerge document. HTTP route: none.
- SDO/RWA test: the worst holder is whoever holds one device's channel tree. They see the
  person's direction to their agents. Boundary 1 bounds it (transcripts and recall receipts do not
  cross), with boundary 5 (a device is bound to a person by an enrolment act, never inferred).

Entity: **the room** (a shared session; its roster is its participants)
- Classification: no class fits cleanly, and that is a finding. Each thing said belongs to its
  author (Private, B) and is jointly held by the roster. Nothing in the room is witnessed by the
  community. What leaves the room is Attested-Private (B2) in shape: the room's record stays
  private and a graduated entry is the outcome others may judge. Whether a room is a membrane or a
  namespace is the reach brief's tension 6 and is not settled here.
- Head-plane cost: none for a room or its record. If graduated entries were ever notarized, a
  collective holds on the order of a thousand within a year, which is past the bundling line: one
  composite root per collective, never a head per entry.
- Network stakes: every stage. Floor-protected: constitutional records of who assented to a
  graduation.
- Address: content-derived, the CID of the room's opening declaration (roster, purpose, opened
  at), as a session envelope already has. No slug.
- Source of truth: the participants' own copies. No server holds the room.
- Integrity zome, coordinator function, route: none proposed.
- SDO/RWA test: the worst holder is an outside observer of the channel or a doorway a hosted
  participant sits behind. Boundary 6 is the one at risk: that a room exists, who is in it and
  when they speak. Removal needs a key epoch (confidentiality cluster row 14). A design that lets
  membership be read from outside is refused.

Entity: **the graduation** (an entry leaves a room or a steward's pile for the collective)
- Classification: the verbs exist locally (`contribute`, `feedback`, `graduate`, a Steward's
  verdict in the flow plane). Its network form is unbound (unified memory loop C13) and no type is
  proposed here. It is the one act in this set a community witnesses.

Back-fill check: no coordinator function returns anything and no route accepts an id, because
nothing here creates a DHT entry. No integrity zome is touched and no DNA hash moves. At one year:
about a thousand entries per collective and an unbounded count of things said in rooms, neither of
which may become a per-item head.

Concern canon, preliminary (the canon rows were not read; no code exists, so no registry row was
made): plane location, answered (the device channel; never the DHT). Anti-self-election, partial
(at the first rung the author's own act suffices; from the second person on, a Steward who is not
the author). Monotonic authority, unbound (which of two devices' versions of an entry applies).
Liveness, partial (a silent sibling must not delay a session's start). Honest absence, partial (an
entry that has not arrived must read as not yet folded, never as absent). Evidence-not-authority,
answered (a folded entry is the sibling's custody, not local acceptance). Bounded work and
idempotent effect, partial (fold by CID is idempotent; volume is unpriced). Advertise and serve
symmetry, unbound. Observability per decision, unbound (the join record has no home). Identity
lineage, partial (device revocation exists; a re-keyed device's past entries are unaddressed).
Contract evolution, partial (the crossing list is versioned by measure). Backpressure, unbound.
Consent, partial (roster admission; per-room assent is not built). Graduated authority, partial.
Witnessed residual, unbound.

Design constraints discovered:
- A room is the general case and a person's two devices are a room of one. Design the crossing
  kind for the room and let the first rung be its simplest instance.
- Alone on one device, memory works whole: write, recall, index. A sibling adds and is never
  required. Memory crossing must not make a session wait, unlike a steward-scoped lease.
- The reach of the bytes, of the index and of the join differ: text may cross, the index is
  rebuilt per device, the join stays with the person.
- Before a spec: decide which version of an entry applies when two devices hold different ones,
  and where the record of a join lives.

**The memory half gets a first-rung shape (2026-11-18).** Minted from the [Memorilla and dreaming survey](epr:memorilla-claude-dreaming-cross-pollination-2026-11-18), §4.5, with the
operator's direction of that day. Nothing here is designed; the P2P design gate still runs first.

*The first rung already exists.* The steward channel has the four parts every rung needs: admission
by roster, a default-deny crossing list, folding a sibling's records as the sibling's with an
acknowledgement vector, and the rule that thoughts stay. Memory is not on it yet.

*What crosses.* An entry says what it applies to: the device (stays), the steward (crosses the
channel, device to device) or the repository (git, as now). Transcripts never cross; each device
notices over its own and only the resulting entry crosses. A sibling's entries fold as the
sibling's, shown with their origin. Text crosses and each device rebuilds its own index with the
pinned model; vectors never cross. This answers open question 4 in part: between one person's
devices the author's own deliberate act is the whole of the assent, and the charter's "a Steward
who is not its author" begins at the first rung that holds a second person.

*Graduation with intention and care (operator, 2026-11-18).* When one peer works with another, a
conversation that should be shared or a memory that should be kept collectively graduates
deliberately, with care at its creation and at its graduation. Requirements to carry into the
design: an entry declares its audience when it is written; each party may graduate what they
themselves said, and a joint record needs every participant's assent; a graduating act says why,
for whom, what context is stripped, which memory class the result takes and how it would be
withdrawn; a scheduled dream may propose a graduation and never performs one.

*A shared session is a room (operator, 2026-11-18).* A chat or session that several parties are in
has a rough analogue in the old "private chat room". Read on this ground, a room is a small rung of
its own: its roster is the participants, what is said in it is said to that roster, and its reach
is the room. That sharpens the rule above. A solo session's transcript is thought and stays; a
shared session's transcript is already said, at the room's reach, and is jointly made. Taking
anything out of the room is the graduation that needs every participant's assent. What a room adds
to the six things a rung supplies: what a late joiner may see of earlier history, and what happens
to keys and holdings when someone leaves or is removed. The confidentiality cluster's row 14 (key
epochs on removal, membership unobservable from relays) and the reach brief's tension 6 (holons
versus spaces) are where those two are already being asked. The difference from the web 2.0 room
is the holder: there a server held the room and its log; here the participants do.

*A candidate habit, not yet declared.* "A correction given on one device is in force on the sibling
before that sibling's next session." It is observable with two devices and one entry and no git
pull. Declaring it would give this half the habit it has been waiting for. The WIP fence holds two
active habits already, so it would be declared inactive.

*Composition: what earlier deliveries should leave open.* Whatever ships first on this channel, on
the artifact store or in the memory tooling should not close these:

- The crossing list stays `{measure: [kinds]}`, default deny, so a memory kind is one added row.
- Every folded record keeps its origin device, so an entry can fold as theirs.
- Memory frontmatter grows by adding fields, and the index projection tolerates and can print them.
- A payload held for a sibling is treated as opaque bytes. Nothing assumes the holder can read it,
  so encrypted custody can begin at the household rung without a rewrite.
- A session covenant can grow a declaration of the joins a session may make, beside its write set.
- Recall receipts stay private. Where the record of a join lives is undecided.
- None of this enters the DHT.

*What a rung supplies.* Roster, crossing list, fold, graduation, custody and join: the survey's §4.5
table gives the first rung's answers and the household rung's expected ones.

**The landing plane, closure-keyed (2026-10-29).** The jobs half of this seed now has a design that
names its rungs: [the closure-keyed landing-plane design](epr:closure-keyed-landing-plane-design)
turns the steward space's A8 (sibling verdict = the build cache), A9 (the landing intent as a
`FlowRecord::Intent` atom; `dev-integration` held only for a 600 s CAS window) and A10 (Jenkins as
witness, attestation written on the device) into sprint slices, and adds the carry (S3a: a gate
neither device paid runs on the device whose own guard lines admit it, over eprfs-peer). The
operator's ruling the same day: master peer utilization of compute before adding hardware; enlisted
machines enter as enrolled devices running `epr peer serve`, never as Jenkins agents; the onboarding
story is "fork an lvi workspace, join, and your peer carries integration jobs while it is on".

