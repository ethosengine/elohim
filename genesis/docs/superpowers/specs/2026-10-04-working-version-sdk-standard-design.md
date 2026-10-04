---
title: The working-version standard — how an app keeps a thing that is still being changed, and how it becomes a published version
id: working-version-sdk-standard-design
status: Draft
class: architecture
serves: dataplane-convergence
date: 2026-10-04
context-tier: disclosed
steward: human:matthew
graduation-trigger: one content type declares a working version in its manifest, the SDK verbs in §5 exist in storage-client-ts, and the shared-sheet story's offline-edit scenario passes on the household mesh with the edit circle enforced on the sync path
cites:
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
  - "version-dag-lives-at-l2-not-in-crdt-doc | History/ADR: The version DAG lives at L2 (the DHT), not inside the Automerge doc | sha256:3fd5a2fffdc19377 | path: genesis/docs/content/elohim-protocol/history/2026-08-07-version-dag-lives-at-l2-not-in-the-crdt-doc.md"
  - "sync-state-contract | The sync-state contract | sha256:cea95b17140d2fd6 | path: genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md"
  - "app-manifest-staged-intents-design | 2026-05-28-app-manifest-staged-intents-design | sha256:98e0a6576d9a197a | path: genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md"
  - "cell-qualified-projection-and-sync-contract | Cell-qualified projection and sync | sha256:54ff98e31c7f793a | path: genesis/docs/superpowers/specs/2026-09-08-cell-qualified-projection-and-sync-contract.md"
  - "platform-one-sdk-many-apis-design | THE ELOHIM PLATFORM MODEL | sha256:a15b10c68787a460 | path: genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md"
  - "swarm-curve-and-blind-custody-design | The swarm curve and blind custody | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
  - "private-thought-governed-fruit | private-thought-governed-fruit | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  - elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
  - elohim/sdk/storage-client-ts/src/sync.ts
  - elohim/elohim-storage/src/sync/doc_store.rs
  - elohim/elohim-storage/src/p2p/sync_state.rs
  - elohim/elohim-storage/src/p2p/reach_authorization.rs
---

# The working-version standard

This is a design, not a build order. It says what an app, the SDK and a storage peer each owe when
a person is still changing a thing. Nothing here is scheduled. It fixes the contract; exact types
and wire shapes belong to the build.

Terms this leans on: the *head* is the declared current version of a thing, chosen by an election
the DHT runs among candidate versions (called the notary here, because it witnesses and orders
and stores little); a *doorway* is a web gateway that hosts people who run no node and holds their
keys for them; *reach* is how far a thing may travel, from `private` through `self`, `intimate`,
`trusted`, `familiar`, `community` and `public` to `commons` (`glossary.md`); the entity classes
and the numbered hard boundaries are in the `p2p-design-gate` skill and
`private-thought-governed-fruit.md` §4.

## 1. The gap

The protocol has a settled form for a thing at rest: an addressed version, held by any device that
has its bytes, checkable offline. It has no agreed form for a thing in progress. Today:

- The TypeScript SDK can load and merge an Automerge document (`AutomergeSync`,
  `elohim/sdk/storage-client-ts/src/sync.ts`). The one consumer only reads
  (`content-doc-sync.service.ts`); it never changes the document and keeps it in memory only.
- Storage projects published content into Automerge documents (`node:{id}`,
  `elohim-storage/src/sync/projector.rs`). Nothing writes a person's own documents; a `personal:`
  document prefix is recognized (`sync/doc_store.rs:303`) and unused. This standard gives it its
  use (§3).
- A new version is declared through `declare_content_head` (`POST /db/content/{id}/head`).
  Nothing connects a changed document to that call.
- The browser has no place to keep an unfinished change across a reload or a lost network.

Two earlier rulings fix the ground this stands on:

- **The merging layer carries values, not authority.** "The CRDT plane converges VALUES; the DHT
  carries VERSIONS and elects the canonical head" (history 2026-08-07). What peers merge is
  unauthenticated input until a version is declared and elected.
- **Three moments.** A person opens a thing to change it, changes it with whoever may, and asks for
  it to be republished (spec 2026-07-01 §5.6). That section is headed "a new design space" and
  leaves open who agrees to republish (OD-7). This standard settles that per content type (§4).

## 2. What a working version is

A working version is a changeable copy of a thing, kept on the merging layer, that carries the
values a person is working on and nothing else. It is not itself a version: it has no place in
the election, no anchor on the DHT and no standing. It becomes real to the network only when a version made from it is declared and the notary's election
chooses it.

Not every content type has one. A vote, a consent, a payment is one act: there is nothing in
between to keep. A document, a sheet, a plan or a long form has one. The content type says which.

Four things vary independently, and the standard treats each on its own:

| What varies | The cases |
|---|---|
| Who holds the person's key | the device; a doorway, for them; nobody (a visitor) |
| Who else is present | nobody; the person's other devices; other people who may edit; an always-on copy |
| How far the thing may travel | its reach, from private outward |
| Whether the type has a working version | none; the author's alone; shared by an edit circle |

A design is read along each row separately. Adding someone or something along one row never
becomes a condition for what worked without it.

## 3. Design gate

### Entity: working version

- **Classification:** none of the gate's five fits, and that is a finding. A solo draft is close to
  Private (B), which the gate defines as a private source-chain entry and gives "draft content" as
  an example. A source-chain entry per change is the wrong cost, and a shared working version
  belongs to several agents. It is not Ephemeral (C): an unpublished change cannot be rebuilt from
  anything notarized.
  - *Missing node:* chain `content change` / between `Private (B) draft` → `Notarized (A) content
    version` / missing node: circle-held merging state that is durable on its holders and has no
    notary footprint; probe: delete every holder's copy and the unpublished change is gone, by
    design / current state: exists only as a projection of already-published content.
- **Decision:** a working version is an Automerge document in the holder's local document store,
  and it is never the document the projector writes published values into. That document
  (`node:{id}`) receives publication writes (`sync/projector.rs:267`); unpublished values placed
  there would meet them. A working version is its own document: under the `personal:` prefix
  while it is the author's alone, and under a separate id of its own when it is shared. The
  prefix is only a label today: document listing and serving make no exception for it
  (`p2p/mod.rs:8130`). The standard therefore requires that a document held for its author alone
  is excluded from serving, from listing and from automatic sync, on every path. That exclusion
  does not exist and is listed in §9. Automerge is what the merging
  layer already runs; this standard does not reopen that choice.
  A draft that must travel with the person's chain (for example across a device migration) is
  additionally checkpointed as a Private (B) entry at a moment the person chooses, never per
  change: a source-chain entry is permanent and costs a conductor write each time, which is the
  wrong price for a keystroke.
- **Head-plane cost:** none. A working version adds no head, no provider record and no conductor
  round-trip. Republishing adds one version to a head that already exists. This is the point of
  the layer: change is free of the notary until it is published.
- **Network stakes:** behaves the same under all four declared network stages (Simulacra,
  Bootstrap, Coordinated, Enforced; `elohim-storage/src/trust/stage.rs`). It touches no
  floor-protected cost. Republish inherits the head path's pricing.
- **Address:** an agent-scoped composite: the agent who opened it, the content id it was opened
  from (or the content type, for a thing never yet published), and a local name chosen at
  creation. It records the published head it was opened from. Justification for the local name:
  no content exists to hash yet. Document ids become cell-qualified under spec 2026-09-08.
- **Source of truth:** the holders' local document stores, until republish. After republish, the
  DHT, as for any content.
- **Integrity zome and DNA-hash class:** no change. DNA-hash-neutral.
- **Coordinator:** existing functions only, and there are three paths, which the 2026-08-07
  ruling keeps apart.
  - *The root author, or a device they delegated.* A new version is an update on the root author's
    own chain and becomes the head of that chain. `declare_content_head`
    (`content_store/src/lib.rs:5586`) only re-affirms the head or re-declares an older version the
    root author wrote (`:5647`); it is not how a new version is submitted.
  - *An earned declaration.* The root author, a device carrying their signed delegation, or the
    bootstrap steward may declare a version as the earned canonical head
    (`declare_earned_canonical_head`, `:6196`; reached through `POST /db/content/{id}/head` with a
    delegation).
  - *Anyone else.* Their version is written on their own chain and offered through a staging
    canonical-head declaration (`declare_canonical_content_head`;
    `POST /db/content/{id}/canonical-head`).

  Every declaration is a candidate in the election (`select_canonical_winner`, `:3154`): earned
  declarations outrank staging ones, then the declaration's time, then its link hash. A staging
  declaration therefore loses to any earned one.
- **Projections:** the existing ones. The projector already writes published values into the
  document; the reverse direction stays as ruled: values in a document never reach an authoritative
  column except through a conductor-verified path.
- **Routes:** the existing document routes under `/sync/v1` and the existing head declaration. No
  new route is required: the SDK composes a content write and a head declaration (§5). If a single
  call is later wanted, it is declared in storage's `build_manifest()`.
- **Count at one year:** working versions are bounded by what people have open, on their own
  devices. They cost quiesce nothing.

### Per-plane shape (gradient reading)

- **Reach of the bytes:** the published origin's reach is a ceiling. The working version's own
  circle starts at the author alone.
- **Custody:** the author's device. Durability beyond one device needs a second holder: another of
  the person's devices, or an always-on copy.
- **Freshness:** every copy states its sync state (§6).
- **Linkability:** everyone in the circle sees who changes what. Any relay on the path sees that a
  document is being changed, when and how much.
- **Cost bearer:** the holders.

### SDO and RWA test

- **A solo draft is a thought.** It stays on the device. It is never sent to a doorway, and never
  enters a notarized plane (boundary 1). For a hosted person this means the browser, not the
  doorway, holds it.
- **A shared working version through a doorway is readable by that doorway today.** The sync routes
  carry plaintext and the doorway proxies them. Until changes can be sealed to the circle, a shared
  working version may pass through a doorway only for a thing whose reach already admits that
  doorway as a reader: `public` and `commons`, and `community` where the doorway serves that
  community. A thing at `familiar` or narrower is shared between the circle's own nodes or not at
  all. A hosted person can still keep and edit their own draft of a narrow thing in their browser;
  what is refused is sending it through the doorway to others. This is a refusal, not a mitigation
  to schedule.

  The test named here (for social dominance and authoritarian following) asks what the few who
  seek to dominate could see, join and compel if they held this store. Boundary 1: thoughts never
  enter a notarized plane. Boundary 6: taking part is itself sensitive, and unobservable from
  outside the circle at narrow reach.
- **Participation:** who is in a circle is visible inside it and to any relay that must route for
  it. That is acceptable inside the household and not outside it (boundary 6).

### Concern canon, for the one new decision

The new decision is "may this peer join this working version". Today it is unbound: no sync path
asks it (`reach_authorization.rs` is called from the EPR write path and provider reconcile only).
C12 (consent and authorization) and C5 (evidence, not authority) are the classes it must answer.
It is registered when it is built, not here.

## 4. What a content type declares

A `workingVersion` block on `ContentTypeDeclaration` in the app-manifest schema, beside `coupling`
and `quiltPolicy`:

| Field | Values | Meaning |
|---|---|---|
| `kept` | `none`, `author`, `circle` | whether a working version exists, and whether others may join it |
| `republish` | `author`, `circle-agreement`, `governance-act` | who must agree before a version is submitted |
| `passing` | a list of names | what unstored, in-the-moment state the type has (presence, cursor, selection) |

Absent block means `kept: none`. `republish` makes OD-7 a per-type declaration; its three values
are the three answers that open decision lists (the author alone, the people editing together, an
act of the governing body), and a type picks one because a shared recipe and a community charter
do not want the same rule. `passing` is a list of names only: what passing state means is the
app's to define, and the standard needs to know just that it exists and is never stored. Only `author`
(with the existing device delegation) can be honoured by today's zome gate; the other two are
declared values with no enforcement yet.

This is a sibling of `stagedIntents`, which also holds something before it is canonical and
replays it at a later moment. The two are kept apart: a staged intent waits on a person's
identity; a working version waits on a person's decision.

## 5. The verbs

In `@elohim/storage-client` beside `AutomergeSync`, mirrored in `crates/elohim-sdk`.

| Verb | What it does | Alone, offline |
|---|---|---|
| `open(contentId)` | makes a working version from the published head the device holds | works |
| `start(contentType)` | makes a working version of a thing never yet published | works |
| `edit(change)` | applies a change function to the local copy (the shape Automerge's own `change` takes); merges with others when present | works |
| `widen(to)` | admits named people, or a reach no wider than the origin's, to the circle | prepared and signed by the person; takes effect when they are reachable. The admission is a signed record of its own, never a field in the working version: what peers merge is unauthenticated and cannot carry authority (§9) |
| `fix()` | freezes the current values as a version and computes its content address, the same address the published version will have | works on any device |
| `republish()` | signs the fixed version, writes it, and submits it by the caller's path (§3) | needs the key: see below |
| `state()` | per other copy: caught up, behind, unknown; and the republish outcome | works |
| `history()`, `revert(to)` | the changes so far; return to any earlier point | works |
| `discard()` | drops the working version | works |

**What republish returns.** For the root author the new version is the head of their chain once
written. For anyone else it is a candidate, and a candidate can lose. Either way the head others
see is whatever the election currently says, and that can change when a later declaration
arrives. So `republish()` reports an observed state, never a final one: `pending` (written, not
yet seen in the election), `current` (the election's present answer), `not-current` (another
candidate is), or `refused`. The existing route answers with a head view or an error; reporting
these states is new work in the SDK, built on reading the head back. A version that is not
current stays as a working version the person can merge forward and submit again.

`fix()` freezes the values on the device that calls it. Two people who fix the same values get the
same content address, and still make two declarations: candidates are told apart by their
declaration, not by their content.

`refused` is a gate saying no before any election: the type's `republish` rule is not met, or the
caller asked for an earned declaration without being the root author, their delegate or the
bootstrap steward. A caller with no such standing is not refused a staging declaration; it is
simply outranked.

**Refusals each verb owes.** `open` on a thing the device does not hold: refused, naming that it
must be fetched first. `republish` without `fix`: fixes first. `widen` past the origin's reach:
refused. `edit` never fails on conflict; conflicting changes both survive in the document and the
app shows them. Every refusal names its reason.

**Why `fix` is separate from `republish`.** Fixing needs no key and no network; republishing
needs both. Keeping them apart is what lets a hosted person, or anyone offline, finish their part.

**By who holds the key:**

| The key is | Kept in | `fix()` | `republish()` |
|---|---|---|---|
| on the device | the local document store | works offline | signs offline; declared when the conductor can publish |
| held by a doorway | the browser's IndexedDB, beside the existing content cache | works offline | waits for the doorway to sign; the person sees "not yet saved to your account" |
| nobody's (a visitor) | memory | works | not offered; the person is told the work is not kept |

**Acts that need another party.** Where an act cannot complete alone (a consent someone else must
receive, an agreement two people sign), the person's side is prepared, fixed and kept alone, and
completes when the other party arrives.

## 6. What each copy says about itself

The sync-state contract already defines the vocabulary: position, declared, and whether the first
contains the second (`SyncStreamState`, `p2p/sync_state.rs`; for Automerge documents, position is
the local heads and declared is the remote heads). The words to show are the ones the SDK already
uses for other streams: `caught-up`, `behind`, `not-computable` (`StreamSyncState` in
`api/dataplane.ts`). No new words. The reading itself is new work: today that type covers
replication, pull and projection, and a document sync returns only the document, whether it
changed, and its heads (`sync.ts:31`). A per-document, per-copy state has to be built.

The person is shown, at least: what is only on this device; which other copies are caught up,
behind or unknown; whether a version is pending, elected or lost. A wait on the network says that
it is waiting.

## 7. What passes and what stays

- **Shown here only** (a selection, a scroll position): never sent, never in the document.
- **Passing** (who is present, a cursor, a drag in progress): messages on a separate channel,
  authenticated to their sender so that presence cannot be forged, sent without guarantee, expiring on a heartbeat, never stored. The document gets one
  change when a gesture ends. A decision that matters is never read from passing state.
- The transport for passing state is not chosen here.

## 8. Mixed versions

This section states a requirement; the mechanism is not designed and is listed in §9. A change
carries the schema version of the client that wrote it. A reader translates what it reads into its
own shape, because a writer cannot know every reader that will exist. Nothing requires every device to upgrade before any device can write. Where a
translation would lose something an older client cannot express, the older client reads and does
not write that field.

## 9. What this depends on that does not exist

Each is a missing node, named and not designed here.

| Gap | Where | Consequence until it lands |
|---|---|---|
| The sync routes ask neither who is calling nor what reach admits them. The write route is marked as needing a signed-in caller, which is a declaration the doorway does not yet enforce. The peer-to-peer handler serves and merges documents directly | `http.rs:6974,7046,16303`; `p2p/mod.rs:7930`; backlog `http-reach-enforcement-gap` | a circle cannot be enforced |
| A document held for its author alone is listed, served and synced like any other | `p2p/mod.rs:8130`; `sync/doc_store.rs:303` | a solo draft is not private until this exclusion exists; it must not be written to a store that serves |
| No per-document, per-copy sync state | `api/dataplane.ts:63`; `sync.ts:31` | §6 cannot be shown |
| No signed record admits a person to a circle | none exists | `kept: circle` is declared and cannot be honoured; today a working version is the author's alone |
| The reach check is not called on any sync path | `p2p/reach_authorization.rs` | same |
| Changes cannot be sealed to a circle | spec 2026-08-23 (blind custody), not built | the doorway refusal in §3 |
| Document ids are not cell-qualified | spec 2026-09-08, not built | a working version cannot say which cell it belongs to |
| The browser keeps no document across a reload | `content-doc-sync.service.ts` | no working version for a hosted person |
| How a schema version is carried on a change, and how a reader translates | not designed | §8 is a requirement without a mechanism |
| `circle-agreement` and `governance-act` republish | zome gate is author or delegated device | declared, not enforced |

## 10. The story it is measured by

`genesis/a2o/features/federation/local-first-shared-sheet.feature` (a vision feature, every step
undefined) and its one runnable seam, `dataplane/hub-carries-edit.feature`
(`@concern:hub-carries-edit`, a check of the `dataplane-convergence` habit, born red). The vision
feature's offline-edit and closed-laptop scenarios are this standard's acceptance.
