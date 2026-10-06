---
title: Ink & Switch — what they are building and what it means here
id: ink-and-switch-local-first-reading-2026-10-04
status: noted
class: research
date: 2026-10-04
cites:
  - "working-version-sdk-standard-design | The working-version standard | sha256:b323e209c972831d | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments | sha256:d171beb3a628997e | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "version-dag-lives-at-l2-not-in-crdt-doc | History/ADR: The version DAG lives at L2 (the DHT), not inside the Automerge doc | sha256:3fd5a2fffdc19377 | path: genesis/docs/content/elohim-protocol/history/2026-08-07-version-dag-lives-at-l2-not-in-the-crdt-doc.md"
---

# Ink & Switch: what they are building and what it means here

Research read 2026-10-04. Two passes read primary text raw: the lab's site (sitemap of 185 pages;
Keyhive notebook 00-06, Patchwork notebook 01-11 and the 2026 notes, the local-first and
malleable-software essays, Cambria, Backchannel, PushPin, Upwelling, Livelymerge 01-08, several
dispatches) and the GitHub organizations `inkandswitch` and `automerge` (READMEs, design docs and
source through the GitHub API). Quotes are verbatim from those reads. Not read: the Subduction
wire-protocol docs, the patchwork bootloader source, Embark, Potluck, Peritext beyond a skim.

## The map

One stack on Automerge:

| Thread | What it is | Status |
|---|---|---|
| Local-first (2019) | The premise: seven ideals | essay |
| Patchwork | Their daily working environment: tools and datatypes loaded at runtime over shared documents, with drafts, history, comments, presence | active; they "live and work inside Patchwork every day" |
| Keyhive | Access control and end-to-end encryption for merged documents | pre-alpha, unaudited |
| Subduction | Sync of encrypted, partitioned data between peers (WebSocket, long-poll, iroh/QUIC); replaces Beelay | early preview |
| Cambria (2020) | Mixed-version peers through bidirectional lenses | no production version |
| Livelymerge (2026) | A program heap in one document; the best text on what does not belong in the document | notebook |
| Onomancy | "a local-first name system" | early |
| GAIOS | Funded work derived from Patchwork | active |

The seven ideals, exact headings: "No spinners: your work at your fingertips"; "Your work is not
trapped on one device"; "The network is optional"; "Seamless collaboration with your colleagues";
"The Long Now"; "Security and privacy by default"; "You retain ultimate ownership and control".

## Mechanisms worth knowing

### Access control (Keyhive)

- "a capability system whose state is a CRDT. Every replica that has seen the same set of
  delegations and revocations computes the same answer."
- Individual: "A bare key pair with no membership state." Group: "An agent with a mutable
  membership." Document: "A group that additionally owns encrypted content." A person is a group
  of device keys; rotating a key is adding one member and removing another.
- Levels: "`Relay < Read < Edit < Admin`. Totally ordered." Relay: "May sync and forward ciphertext
  but not decrypt it. The level held by sync servers."
- Delegated authority is "clamped to the weakest link". "Keyhive never restricts sub-delegation
  because doing so pushes users toward sharing raw keys instead."
- Operations by someone later revoked "are retained (to preserve causality) but excluded from
  materialisation."
- What the relay still learns: it "sees operation hashes, sizes, timing, and the membership graph."
  "Accepted. Relays must see membership to evaluate capabilities."
- Non-goals: "Keys are the only principals"; no identity, no key recovery, no trusted time, no
  forward secrecy.
- Unsolved in their own words: concurrent mutual revocation by two admins; honest edits that depend
  on content later found malicious; back-dated operations. On the cure for back-dating: "gain
  consensus on which operations happened up to a certain point... This is fairly counter to the
  local-first ethos... We currently consider it a last resort."

### Presence and passing state

- automerge-repo: `handle.broadcast(message)` is "no guarantee of delivery… not persisted". A
  `Presence` helper adds a heartbeat (15s default) and an expiry (45s). The transport's peer id "is
  *not* a useful user identifier".
- Subduction: "Authenticated, fire-and-forget pub/sub for transient signals — cursor positions,
  selections, presence."
- PushPin: "persistent state that is replicated, and ephemeral state that exists only locally",
  the second on "an additional messaging channel, adjacent to the CRDT".
- Livelymerge: per-device state is marked and never enters the document ("None of this belongs in
  the document"). During a drag the position passes as ephemeral messages and the document gets one
  write at the end, because "Automerge wasn't built for the kind of workload" of drag-rate writes.
- Patchwork tasks: heartbeats only hint at who is active; the record in the document decides. Each
  task "will eventually run, but does not guarantee that it will run only once".

### What the person is shown

- `sync-indicator` names the kind of peer (shared worker, service worker, sync server) and a state
  per peer: "synced", "behind" or "unknown".
- `doc-presence` shows a face per person, de-duplicated across their devices, faded when their
  window is unfocused.
- The essay still lists "how to communicate online and offline states" when "everyone is a peer"
  as open.

### Drafts, history, agents

- A branch is cheap, unnamed by default, deleted on merge. "There's no branching from branches."
  Upwelling found dependent drafts "made the model more difficult for users to understand".
- Merging a branch as one history item "encourages us to use branches as a unit to encapsulate
  work". People wanted to "revert to any prior version", not only saved ones.
- A bot "puts changes on a branch" and appears "as a user in the document history". AI summaries of
  edits were "more successful than any of our other diff visualizations so far".
- Their repos ship agent skills and `AGENTS.md`. Agents are tool authors and document processes;
  no identity or authority model for an agent was found.

### Mixed versions (Cambria)

- "Data translations in decentralized systems should be performed on read, not on write."
- Writes are tagged with the writer's schema; lenses travel with the document, so "An older client
  can read newer versions without upgrading its code".
- "there are limits to interoperability": consistency, conservation and predictability cannot all
  hold.

### Tools over apps (Patchwork packaging)

- A tool declares `{ id, type: "patchwork:tool", supportedDatatypes, name, … }`; a datatype
  declares `init`, `getTitle`. A document names what opens it. A `modules.json` manifest lists
  tool directories and a shell loads them at runtime.
- "these tools should never assume the existence of other tools."

### Where merging stops

- Livelymerge: "Convergence is not enough". Merges replay writes, not intents, so rules that span
  several objects break. "We don't have a solution yet".

## Reading it against the Elohim Protocol

| Their position | Ours | Consequence |
|---|---|---|
| Keys are the only principals; identity is a later layer | Standing belongs to a person or collective, never a key | Their group-of-device-keys shape is a usable model for one person's devices. It is not a model for standing |
| Consensus on "what happened up to a point" is a last resort | A notary floor exists for exactly that | The live copy can stay coordination-free because the settled record is where order is agreed. Their open problems (back-dating, mutual revocation) are live-copy problems that settle at the notary |
| A relay must see the membership graph | Participation is itself sensitive; membership is unobservable from outside the holon | A relay inside the household may see it. One outside may not. This needs a design, not an import |
| Revocation is a key event ordered by seniority | Removal is a governed act with redress | Excluding a removed member's later edits from what is shown, while keeping them, fits. Who decides does not |
| Sync servers are interchangeable and free | Someone carries the always-on peer and that is accounted | Same mechanism, with an economic event beside it |
| History cannot be purged | What may be forgotten is a governance question | Open here too |
| Merge whatever arrives | A change can be refused | Rules that span objects are checked where the record settles, never assumed of the live copy |
| Translate on read; lenses travel with the data | Mixed versions must talk; no big-bang rolls | Directly relevant to upgrade propagation. Worth its own pass |
| Tools and datatypes loaded from a manifest over shared documents | The app-manifest seam | Outside the local-first skill; belongs to that seam |

Everything in their stack is marked pre-alpha, alpha or prototype. It is a design reference, not a
dependency.
