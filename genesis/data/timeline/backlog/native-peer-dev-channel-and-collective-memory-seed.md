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

**The landing plane, closure-keyed (2026-10-29).** The jobs half of this seed now has a design that
names its rungs: [the closure-keyed landing-plane design](epr:closure-keyed-landing-plane-design)
turns the steward space's A8 (sibling verdict = the build cache), A9 (the landing intent as a
`FlowRecord::Intent` atom; `dev-integration` held only for a 600 s CAS window) and A10 (Jenkins as
witness, attestation written on the device) into sprint slices, and adds the carry (S3a: a gate
neither device paid runs on the device whose own guard lines admit it, over eprfs-peer). The
operator's ruling the same day: master peer utilization of compute before adding hardware; enlisted
machines enter as enrolled devices running `epr peer serve`, never as Jenkins agents; the onboarding
story is "fork an lvi workspace, join, and your peer carries integration jobs while it is on".

## Field evidence, 2026-11-22: one evening's coordination ran on the Claude bridge, none on the mesh

Recorded by claude-opus-5-5 (laptop device z6MkgHju…xMbu, session f0b335e0) at the steward's request ("note frictions or improvements for collaboration over the mesh as compared to your own native channel"). Marks: [M] measured or observed this session.

### What happened (2026-11-21/22)

Three agent sessions on three of the steward's devices coordinated one brit release (0.3.2), one
monorepo landing order and one write-set check: this laptop (session f0b335e0), ethosengine (session
elohim-b0, "Input classes for build steps") and shem (reached only through ethosengine). Every
message travelled over the Claude Code cross-session bridge (`SendMessage` to
`bridge:session_…`). The Elohim mesh carried none of it. The steward asked for both channels to be
reliable, so this entry records what each one did.

### The bridge (Claude Code cross-session messaging)

What worked [M]: ethosengine found this session by name and messaged first; replies arrived within
the session and were acted on (ownership of the release settled, a scope error corrected, a
write-set overlap withdrawn, a landing order agreed, a design finding relayed to shem).

Frictions [M]:

1. **No read receipt, and declining confidence.** Every send returned "not confirmed read". From the
   fourth message on it also said "delivery is not confirmed: no recent listing here shows that it
   can receive cross-session messages". Only the next inbound reply proved receipt.
2. **Unsigned and unrecorded.** Messages carry no device or covenant signature and leave nothing in
   the repository; the coordination (who owns the release, who moves the pin, the landing order)
   exists only in two transcripts.
3. **Discovery by chat title.** `ListAgents` listed 138 peer sessions by conversation title, most
   offline. This session could not have found ethosengine's session by role or device; ethosengine
   found it.
4. **No direct path to the third device.** Shem was reachable only as a relay through ethosengine,
   which doubled latency and made ethosengine the editor of shem's words ("relayed verbatim in
   substance").
5. **Authority framing is manual.** Each inbound message is framed as untrusted teammate text; there
   is no attested link from a message to the steward's standing or to a covenant clause, so each
   agent re-derives whether a claim ("Matthew asked me…") is real.

### The mesh (berth, epr peer)

Frictions [M]:

1. **This device was never admitted.** The laptop's roster rows (`binding`, `peerEndpoint`) existed
   only in local commit `c6a5f57fc` from 2026-11-16. SessionStart reported "degraded roster bound …
   local only (in a local commit), not on origin/dev" every session for six days; nothing turned that
   line into a landing. They now ride the integration branch `fix/dialogue-stasis-askuser-20261121`.
2. **Admission depends on each sibling's checked-out branch.** Each peer daemon reads the roster from
   its `--root` working tree (`read_roster`: `root.join(PARTICIPANTS_REL)`), not from a published
   ref. ethosengine's root `/projects/elohim` is on `memory/ceremony-2026-11-20` with 4 rows and no
   row for this laptop, so landing the row on dev does not admit the device there. Whether the daemon
   re-reads the roster or needs a restart is unverified. (Handed to shem for
   `steward-device-arrival-open-items.md`, its write set, as a missing node: "the admitting daemon's
   roster source carries the row".)
3. **`berth` is not on PATH** on this host; it runs only as `genesis/agentic/bin/berth` from a
   checkout. `berth status` shows each sibling as "SILENT: no head folded yet" without saying why
   (unadmitted, unreachable or idle look the same).
4. **`epr peer status` answers by directory.** Run inside the `elohim/brit` submodule it refused "this
   device … is on no participant roster in this checkout"; from the superproject worktree it named
   the device and its head. A refusal that depends on the cwd reads like a device fault.
5. **The mesh cannot bootstrap itself.** Arranging the laptop's admission needed a working channel
   between the devices, and the only one was the bridge.
6. **No cross-device session view.** `berth status` is "this workspace as one blade"; it cannot list
   the sessions moored on a sibling, so an agent cannot find whom to `berth say --to` across devices.

### Improvements (each with its probe)

1. Land roster rows through the normal gate as soon as they are written: when SessionStart reports
   "roster row … local only", `just status device` names the exact landing command. Probe: a new
   device's row reaches origin/dev within the session that wrote it.
2. Read the roster from a published ref (or a declared roster ref) rather than the daemon's working
   tree; at minimum `epr peer status` says "roster source is <branch>, which lacks <did> that
   origin/dev has". Probe: a sibling admits a device whose row is only on origin/dev, with no checkout
   switched by hand.
3. `berth` on PATH through the bin store like brit and epr; `berth status` distinguishes unadmitted,
   unreachable and idle siblings.
4. `epr peer status` resolves the superproject from inside a submodule, or names the checkout it read.
5. A cross-device session listing (moored sessions per admitted device, with role and task), so
   `berth say --to` has an address book; then direct notes to shem need no relay.
6. Delivery acknowledgement on the mesh that the sender can see (today seen-state stays on the
   receiver by design); a signed ack note is enough.
7. Mirror decisions made over the bridge into the repository: a coordination decision (owner of a
   release, pin mover, landing order) becomes a dated line in the owning backlog entry or a covenant
   note, so it outlives the transcripts. This session recorded the release ownership and the steward
   ruling in `brit-rebase-cost-gitlinks-and-stranding.md`; the landing order lives only in transcripts.

### Close when

The three devices exchange a `berth say` note in both directions with acknowledgement, and a
coordination like this one can run on the mesh alone.

### Evidence

- Bridge send results: msg ids `eef9e77b…`, `5749f600…`, `11e6c190…`, `228cc82f…`, `f994eef2…`
  (session f0b335e0 transcript); the last two carry the "no recent listing" warning.
- `epr peer status` from the integration worktree (2026-11-22): this device at head seq 15885, both
  siblings "no head folded yet"; laptop daemon `epr peer serve --root /home/matthew/git/elohim
  --handle matthew`.
- ethosengine's report of its daemon root and roster (4 rows, branch `memory/ceremony-2026-11-20`).
