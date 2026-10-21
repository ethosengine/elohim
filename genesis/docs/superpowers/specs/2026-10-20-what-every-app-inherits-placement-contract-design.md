---
title: What every app inherits — the placement contract, and the bytes layer beneath it
id: what-every-app-inherits-placement-contract-design
status: Draft
class: architecture
serves: materialize-what-is-read
date: 2026-10-20
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), with eight Sonnet plane readers, at the operator's direction"
tags: [placement, sdk, pwa, local-first, eprfs, residency, tevah, berth, brit, rakia, lvi, diversity-of-peers, spaces]
graduation-trigger: >-
  On the household mesh a fresh reader that names a tree root and a view of N paths under it holds
  afterwards exactly those paths' bytes plus the tree nodes on their spine, fetched through one
  storage interface whether the bytes came from its own store, a household peer or a hub, each
  object verified against its address and a requester outside the root's reach refused by name;
  and one app, declared once, opens unchanged for one person offline and for a household, with the
  answer to every plane in §5 readable back from a probe rather than assumed.
external:
  - "https://www.anthropic.com/engineering/redesigning-source-control"
  - "https://about.gitlab.com/early-access-preview/next-gen-source-control/"
cites:
  - "device-footprint-residency-carrying-capacity-design | follow/reside/carry, residency states, the closure requirement and the fetch order this design keeps; eprfs holds no pin | sha256:99ab8f8330cc3f24 | path: genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md"
  - "compute-envelope-tevah | ark envelope, berth as blade offer, berth offer as REA intent, ArtifactRef and the exec floor that the compute layer and install rest on | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "workspace-berth-carrying-capacity-design | the dev-workspace berth lease store, one of the three things named berth | sha256:f7f53bb9e7686d19 | path: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md"
  - "live-session-plane-design | the template: section 9 what every app inherits, generalized here from one plane to all | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "working-version-sdk-standard-design | kept/republish/passing and the solo-draft-never-to-a-doorway rule; overlays are working versions | sha256:72f6e63fde2fb953 | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "seamless-groupware-ux-addendum | install ladder, open-with and runtime offers that the experience-depth and compute planes complete | sha256:a9698f7152f60430 | path: genesis/docs/superpowers/specs/2026-09-08-seamless-groupware-ux-addendum.md"
  - "package-composition-at-the-holochain-seam | A7 placement is not authority; apps are bundles not DNAs; D8 notification kinds | sha256:e417e740ce2212ce | path: genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md"
  - "commons-pool-as-collective-party-design | what a pool may carry and its duties; compute row records no berth offer and no matching | sha256:095276e5ccec79b3 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "runtime-artifacts-elected-content | elected runtime heads and channels; native binaries deferred; projections carry a retention floor | sha256:eaa2716381075140 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "render-delivery-manifest-adapter-design | the absent manifest delivery section and zero-doorway rendering for the publication plane | sha256:9e22fd71be24a5ee | path: genesis/docs/superpowers/specs/2026-07-30-render-delivery-manifest-adapter-design.md"
  - "commons-reach-spaces-physical-holon-design | spaces, the D6 test, leaving rules and the absent peer-side admission validation | sha256:480d0c57ab16449b | path: genesis/docs/superpowers/specs/2026-10-16-commons-reach-spaces-physical-holon-design.md"
  - "graduate-onto-own-node-key-design | leaving the hosted stage onto ones own key; the key-custody plane | sha256:327698391aef591e | path: genesis/docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md"
  - "mutual-storage-replication-dwelling-hub-design | hub roles by discernment, donut bands and replicates-dwelling for the data-holders plane | sha256:3ef54b7a3bec9f17 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
  - "holons-are-spaces-how-we-use-holochain | section 9.1 rungs: household, collective three to seven households, commons seven hubs | sha256:8307626aa810f8a2 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "ai-stewarded-commons-reimplementation-plan | 10.3 and 10.6 declare runtime needs separately; no parallel app store manifest | sha256:0d5f1300b5d615dc | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "elohim-seam-map-concern-routing | tracks T1-T4, device rungs L0-L5 and native-binary-only footprint tuning | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "tiered-quilt-stewardship-design | temperature verbs; quiltPolicy is temperature, not copy count | sha256:14ee4836af492ff1 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md"
  - "storage-physics-benchmark | measured footprint: conductor wasm dominates the household (about 0.8 GB wasm.db against 60-90 MB DHT) and gertrude (3.16 GB on 2026-10-01); fleet conductors hold 6.0-6.6 GB with the split unread, against about 0.3 GB storage | sha256:34eb0e7b16efa51a | path: genesis/docs/content/elohim-protocol/architecture/2026-10-05-storage-physics-benchmark.md"
  - elohim/lvi/docs/specs/2026-07-20-elohim-native-devspace-design.md
  - elohim/rakia/docs/specs/2026-04-12-rakia-design.md
  - elohim/brit/docs/specs/2026-06-29-canonical-epr-meta-git-bridge-design.md
  - elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
  - elohim/eprfs/README.md
  - elohim/eprfs/eprfs-core/src/storage.rs
  - elohim/eprfs/eprfs-core/src/tree.rs
  - elohim/eprfs/eprfs-core/src/composition.rs
  - .claude/skills/p2p-design-gate/SKILL.md
  - "wisdom-layer-floor-ceiling-judgment-culminating-design | the universal floor-and-ceiling pattern: deterministic floor with no LLM, judgment ceiling within the band the floor opens; the shape of section 7 | sha256:f5d694c382a76c1f | path: genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md"
  - "blob-custody-phase3-xor-salvage-placement-design | coordination-free self-selection and the ranking test contract; its open fork between determinism-agreement and coordinated authoring is settled in section 7.3 | sha256:f4c139eee8478b9a | path: genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md"
  - .claude/skills/local-first/SKILL.md
  - .claude/skills/gradient-reading/SKILL.md
---

# What every app inherits

This is a design, not a build order. It states where each placement answer is decided, and the code
is expected to move toward it. What the code has not caught up with is listed under "Missing nodes"
and "Slices", not in the rules. Every "today" claim below was read from the repository on
2026-10-20 by one of eight plane readers; the paths are theirs.

## 1. Why

Two outside designs published in October 2026 reached one answer for one pain. Anthropic's Scion
(source control rebuilt when merges went from about 2,500 to over 30,000 a day, mostly by agents)
and GitLab's next-generation source control for agent fleets both stop assuming that every
participant holds everything. A participant names what it wants to do, the bytes that act reads
arrive, and nothing else does. Scion reports cache hits above 99% at each tier, an infrastructure
footprint about 200 times smaller than its git mirrors, and checkouts in seconds. GitLab's figures
(a 5.2 GB clone against a 184 KB path read) are self-reported early results with no stated
conditions.

Both are written as source control. The protocol's pain is wider and has the same shape. It is not
a Holochain problem; it is the universal problem of how anything is installed, how many copies of
it there are, and how big it is:

- a fleet peer carries 6.0–6.6 GB of conductor volume against about 0.3 GB of storage (adam 2.2 GB)
  (storage-physics benchmark); on the household the conductor's code store (raw and compiled wasm,
  never collected) outweighs its DHT data about ten to one, and what fills the fleet's conductors has
  not been re-inspected;
- a workspace that changes three files holds the whole monorepo, its toolchains and its build
  caches;
- a gate that asks "was this input judged?" re-runs, because the answer was bound to a branch and
  not to the bytes (brit evidence by input closure, wave 1);
- a phone, a Chromebook and a rack are asked to carry the same footprint, which the diversity of
  peers forbids.

The operator's statement of the target, 2026-10-20: design the SDK and PWA contract for how
local-first reaches the web, so that every application inherits the properties that let it know what
it needs to work natively in the network, depending on the space it is used in — one person offline,
a couple in a household, a small group at a church or co-op, a team of commons developers with a lens
over a repository. "The runtimes and servers are liquid and just fill the available berths, and
supply all the pieces to fit the needs of the humans in whatever application is supporting them."

The rulings of this design:

1. **An app declares needs, not places.** No manifest field names a device, a hub, a doorway or a host
   (§6, placement neutrality); the schema is where that holds.
2. **The space supplies.** Who is present, which berths they offer, and what each device can carry,
   decide where each need is met.
3. **A commitment records every carrying.** Nothing is carried for another party without a record
   that names the carrier, the recipient and the bound. The record is kept at the reach of the need it
   answers: notarized for a public or commons need, and a household-local signed record below public
   reach until per-space cells exist (§7.2, §14).
4. **A probe reads every promise back.** A placement nobody can read back cannot be told apart from a
   silent failure.
5. **The bytes beneath all of it are answered once, in eprfs**, for every kind of bytes: source, app
   bundles, DNA wasm, compiled code, build outputs, model weights, documents. brit, rakia, lvi and
   the runtime are consumers; none grows its own fetch path.
6. **The person's identity root is held by the network, not by one device alone.** Restoration and reconciliation
   are rights the network affords every person; a device is enrolled under the identity root and acts for it.
   As soon as any peer is reachable, custody of the identity root is secured by the network, so that everyone is
   always recoverable and held with dignity as part of the commons (operator steering, 2026-10-20).
   This is a commitment the network makes to every person, and it is not yet kept: the first-contact
   custody and the `KeyRotation` mint path are missing nodes (§15). Until they land, the SDK tells a person
   plainly which of their roots are provisional.
7. **Every answer the network cannot give yet still explains itself — always.** Interpretability,
   explanation and feedback are first-class properties of the protocol, held at the ontology, the
   architecture and the viable-system levels (operator steering, 2026-10-20). Whenever a need is unmet, a
   read cannot be served, a request is refused or an answer is withheld, the person always gets three
   things: **why** (the typed reason: absent, unreachable, refused or unverifiable are different answers),
   **when to come back** (the retry time, or the event that will change the answer), and **what they can do
   locally in the meantime**. If they ask again before that time, the gate escalates the answer rather than
   repeating it: the same three, plus what changed and who can be asked. This is the one "always" the design
   makes without a gate of its own, because the gates are already the canon's: honest absence (concern
   C4), observability per decision (C8, every outcome through a typed reason) and backpressure (C11, a
   refusal carries its retry time). Each refusal type this design adds carries the three by type, so a
   refusal that lacks one does not compile.
   The three parts travel on a channel, not in a one-off error. Like a reactive stream's demand, backpressure and
   completion signals, a pending answer keeps telling the person where it stands (queued, retrying at a
   time, carried, refused with a reason) at the quality of service its lane needs, until the answer
   arrives or the need is withdrawn. A quiet channel costs nothing at rest (`idle-is-free`).

Two words are used in one sense each. A **tree root** is the CID of an eprfs tree (§12.2); the bytes layer's
"root" always means this. A person's **identity root** is their key lineage (ruling 6); it is always
written in full.

Four component names recur. **eprfs** is the protocol's content-addressed layer of trees and blobs;
**brit** is the git bridge that seals git trees into eprfs; **rakia** is the distributed build
substrate that produces content-addressed, attested artifacts; **lvi** is the devspace runtime that
projects and mounts a development environment instead of shipping it.

The live-session spec §9, "What every app inherits", is the template: the app names its passing
values and the SDK does the rest. This design generalizes that one plane to all of them.

## 2. What is already ruled and is not redesigned here

- **Residency** (device-footprint spec): follow, reside and carry are three acts; the residency
  states are kept offline, in use, archived and opened from my hub; a release must declare its
  closure; bytes come from this device, the person's other devices, the household hub, holon hubs, a
  pool, and a doorway. A doorway is a participant in that order, not a fallback to apologize for: it is the
  web's way in, plural and swappable, and governed as a commons facility by those who come in through it.
  A browser reaches the network through a doorway by design, which is why the service worker asks one for
  its delivery peers (§5.1). What a person relies on is that doorways are plural, not that they are absent:
  no single doorway is one they cannot leave, and a peer that can address others by key reaches them
  directly. **eprfs holds no pin and no collection of its own.**
- **Placement is not authority** (package composition A7): which device executes, serves or stores
  is placement; who may read or act is authority; placement does not change who authored an act (the act carries its author's signature) or who
  can decrypt it. No parallel "app store manifest" (reimplementation plan §10.6).
- **The live session** exists only while someone is in it, holds nothing that cannot be resent,
  writes nothing down, and is relayed by two hosts inside the circle, or by direct streams between
  peers that know each other's keys. The hosts are ranked to prefer a second host in a different
  household, but a household's own session is hosted on two of that household's devices.
- **The working version**: a solo draft is not to be sent to a doorway (the rule; the sync exclusion that
  would hold it is absent, §18); `kept`, `republish` and `passing`
  are declared per content type.
- **The envelope** (tevah): ark runs, quotas, accounts for and witnesses processes; a berth is what one
  blade offers and holds; a berth offer is an REA intent matched against under-held commitments.
- **lvi**: mount, don't ship; derived bytes do not enter commons custody (lvi's invariant).
- **Hosting is a stage, doorways are plural**, and a doorway is narrowly web2: names, TLS, edge cache,
  ingress and projection. App serving and rendering are peer-side.
- **Reach, head and replication are three planes.** Collapsing two of them is a recurring bug class.
- **Addressing**: CIDv1 sha2-256: dag-cbor for trees, raw for blobs. That is the only protocol
  address. BLAKE3 is iroh's peer-to-peer transport key and is not a protocol address (the addressing
  canon). Today the iroh
  blob store is keyed by BLAKE3, and storage's `/blob` routes accept a `blake3-<hex>` alias next to the
  `sha256-<hex>` form; §12.3 rule 5 covers closing that alias gap.
- **CI runners are sponsored service agents**: one key, hosted on a sponsor's peer, bounded grants,
  not a device. The runner story shows a runner's device approval refused live
  (`compute/ci-runner-stewarded.feature`).

## 3. The six spaces

The contract answers every plane in every one of these spaces: the app declares once, and the space
determines how each answer is supplied (§7). The plane tables in §5 are therefore space-generic. In the table below, the
vocabulary column names the existing word; where none exists it says so.

| Space | Who is present | Existing vocabulary |
|---|---|---|
| **Solo** | One person, one device, nothing reachable. The identity root is provisional here: it is secured by the network at first contact (ruling 6) | Device rung: "no space" (holons §9.1) |
| **Household** | Two or more people (a couple is a two-member household), their devices, maybe a hub | Household holon; affinity `household` |
| **Group** | A church or co-op: 30 to 10,000 members, groupware | Collective; charter; affinity `congregation` |
| **Dev team** | Developers with a lens over a repository; heavy builds | A collective plus an `author-lens` over the repo. No reach value means "team" |
| **Public** | Strangers with no membership, reading what is published | No membership by design; reach `public`/`commons` |
| **Agents** | AI agents and CI runners: online while they work, sponsored, bounded | Sponsored service agent (ci-runner ruling); actor `agent:<role>@<model>` |

The spaces are not a ladder a person climbs. One person is in several at once. The design rule is
that **an app does not notice which space it is in**; the answers re-place themselves (§5.7, space
shape change).

**The working hypothesis for how spaces nest** (holons §9.1). This is a technical hypothesis under product
review, not a ruling. What §0b D6 of that document seals is only that group clone spaces are an
operator-selected candidate, gated behind D0, D3 and a measured phone and hub budget, and that automatic
household-to-DNA mapping is retired. The hypothesis says a space is the smallest group that can validate its
own writes and afford its own recovery. Household: every device holds it, and it survives the loss of any
one device. Collective: 30 to 10,000 members, held fully by three to seven independent households, with a
floor of three evidenced domains. Commons: a floor of seven diverse hubs. Affinity and locality are filters
on records inside a rung, not rungs of their own. A space earns its own cell only by passing the written D6
test (§10 item 18). Where this design leans on a rung's numbers it says so, and the numbers move if product
review moves them.

## 4. The contract's four parts

For every plane in §5:

- **Declares** — what each participant expresses (§7.1): for the app, in the existing app manifest. A need,
  not a place.
- **Supplies** — what the space offers: participants, postures and berth offers (both defined in §7; "berth" means tevah's
  blade offer throughout), device capacity.
- **Records** — the Mishpat/REA commitment that accounts for who carries what for whom.
- **Proves** — the probe that reads the promise back, and the habit that holds it.

Today the manifest declares almost none of it. Its top-level keys are id, name, version, description,
vocabulary, rendering, projections, writeThrough, observation_kinds, signalKinds, graduation,
constitutionalRatios, graph, gates, attestations, routeClaims and ownsUniversalRoute. The nearest
existing needs are `quiltPolicy` (temperature, not copy count), `coupling.governance.defaultReach`
and `minimumReach`, and `routeClaims`. Every other Declares cell in §5 is a proposed addition (§6).

## 5. The planes

Twenty-three planes in seven groups. The operator named six of them (marked ◆); the rest were found by
reading the six against the gradient-reading planes, the local-first lines and the seam map, and were
confirmed in scope on 2026-10-20. Each row gives the question in words a person could answer, then the
four parts and the state today.

### 5.1 Bytes

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| ◆ **Code residency** | What code does this device hold to run my apps, how big is it, and is it one copy shared by every app that names the same runtime? | `closure` (a tree root plus a view that together name every blob needed to start) and the runtimes it names | Device budget; hub window (ten releases); pool | Device pin; release-adoption receipt; `runtime-offer` (absent) | `elohim_node_store_bytes`; `just status device`; code store against the 150 MB target | No release declares a `closure` (the app manifest and the release ledger have none). Two rule fields could point at one: the device pin's optional `closure_rule` (`acquisition_pins.closure_rule_json`, set through `POST /api/v1/pins`) and `replicates-content.closure_rule`. Both are stored and passed through; nothing resolves either into blobs. The manifest's `gates.*.closure` is a gate's silence posture, a different concept. Bytes dedupe by address, runtimes do not: one hApp, five roles, cloning off. Conductor wasm dominates where it has been read: about 0.8 GB of `wasm.db` against 60–90 MB of DHT on each household peer, and 3.16 GB of gertrude's live conductor data (2026-10-01); what fills the rest of the fleet's 6.0–6.6 GB volumes has not been read. Its only bound is a fork patch (3c1e80525, inside the current pin 96bcd2bee) that prunes wasm no installed DNA definition references. The patch has unit tests, has not been measured on a running household or on the fleet, and reuses freed pages without shrinking the file |
| ◆ **Experience depth** | Am I using this thin over the web, from my browser's cached copy, from a shell that holds my keys, or opened from my hub — and what does it work without? | Minimum depth that still works (absent) | Shell, service worker, hub runtime, doorway | Device pin (residency state) | Service worker delivery evidence lines (exist). One a2o scenario, `delivery/client-resilience.feature` "Cached app works offline" (`@concern:record-proves`), checks offline use at the service-worker depth (depth 2) for one app; it is red-first, 8 of 9 steps on its one household run (2026-09-26). No probe takes the depth as a parameter, so depths 0, 1 and 3 have no offline check | Service worker registered and verifying; no web-app manifest, so not installable; the delivery-peer list is fetched from the doorway; "opened from my hub" specced, unbuilt |
| ◆ **Data holders** | If a device or a household is lost, how many other holders keep our bytes and documents, and who agreed to carry them? | A loss model (survive device / household / region loss), resolved per space (absent) | Peer pool with `household_id`; pledged bands | `custody-blob`, `replicates-dwelling`, `replicates-content` | `blob-durability` habit (green 2026-10-18) | Copy count is one node-wide env knob (`SALVAGE_TARGET_REPLICAS`, default 2). The household-diverse placement strategy exists. Documents have no custody commitment at all; `personal:` exclusion from sync absent |
| **Derived indexes** | When our group searches, who built the index, where does it live, and can it be thrown away and rebuilt? | Which content types are indexed, by which measure (absent) | A berth that holds the bytes | `delegates-compute` (pool folds, specced) | Fold attestation beside the store | Lexical fold per peer, keyed by measure CID only, not by input root; no group-scoped index; storage refuses embeddings |

### 5.2 Presence and attention

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| ◆ **Live channel** | When I open a shared thing, can I see who is here and what they point at, without anyone I did not invite learning I was there? | `passing` values (specced, not in schema) | Two hosts the circle admits, ranked to prefer different households (a household's own session uses two of its own devices until frames can be sealed to the circle), or direct streams | `live-session` grant scope (absent) | Zero added calls and timers from a peer offering the host role with no session open, read on `idle-is-free`'s household-at-rest lane (red; that lane allows up to 6 conductor calls a minute per peer) | Specced; the thin-client wire format is still open, and the frame encoding is chosen only for slice 1's direct streams. Slice 1's pure session crate `elohim/elohim-live-session` is built and nothing uses it yet; its story (`dataplane/live-session-own-devices.feature`) is `@wip`. No transport, no host, no `live-session` grant scope and no SDK session object; `elohim-hub` has no code |
| **Chat** | Can we talk, offline too, and does what we said stay? | A message content type under a witness context (absent). The qahal manifest declares `message` and `thread` only as planned vocabulary (status `planned`), bound to no witness context | The group's holders, as for any record | Same as data holders | Offline message converges on reconnect (absent) | No chat design. Only fragments exist: one catalog row in the platform-one-SDK spec (row 5: gossip plane and a `p2p_iroh` topic, `api/comments.rs`, "no server reads the channel"); package composition's rule A2 that a chat thread shares its community's witness context; and the planned qahal `message` and `thread` vocabulary. A message is a record; presence is passing. They are two planes |
| **Attention** | When something concerns me and my phone is asleep, who wakes it, and what does an outsider learn from the wake-up? | Notification kinds ("tell the other peer", package composition D8) | Peer link when awake; a wake ingress for a sleeping device (absent) | None for waking; the signed act is the record | No probe for push metadata exposure | Peer-to-peer notify receiver built for one kind only, `feedback-signal`: the `IntegrityNotify` arm of `EprAtomService` (`epr_atom_service.rs`), reachable over libp2p and iroh. Every other kind is refused as unhandled. `feedback_notify_ingress.rs` is only its test-gated admin ingress (`ELOHIM_TEST_FEEDBACK_INGRESS=1`). OS push (APNs, FCM, Web Push) absent, undesigned |
| **Connectivity** | From wherever I am, can my device reach the peers I may reach, and who carries that for us? | Nothing: derived from reach, not declared | Relay, bootstrap, signal, LAN discovery | None: no commitment says who runs a relay | Mesh preflight; no person-level probe | iroh relay operator-run. libp2p relay client, DCUtR and AutoNAT are wired in both elohim-storage and steward/node (storage can also be a relay server; steward/node is client-only, and its `p2p/nat.rs` is an empty TODO); the cluster libp2p port 9876 is ClusterIP only, so desktops behind NAT cannot reach cluster relays; hole punching has not been measured. A missing relay reads as "ready" at zero connections; browser to hub by key unverified |

### 5.3 Time and authority

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| **Head and version** | Which version is current for my space, and how does a newer one reach everyone without anyone being pushed? | The channels the app needs (absent) | Channel heads; each peer's follow set | Head declaration link (tier, clock, link hash) | `runtime-upgrade-propagation` (green); `@concern:app-bundle-elected-delivery` (Act I household) and `@concern:federation-deploy` (Act II), both checks of `dataplane-convergence` (red) | Heads elected, not "latest". Follow set is node-local config. Nothing binds a channel to a space. The red, active `dataplane-convergence` habit is held red for two reasons. First, a harness defect: its federation-version-convergence check cannot run because the household device ceremony self-approves a device, which mishpat refuses. Second, a measured divergence: `hub-carries-edit` read 0/1 on 2026-10-18, when a restarted peer did not reach the new head within 90 s |
| **Authority** | Who in this space may move its head or change its rules, and how do I check? | `workingVersion.republish` (specced, not in schema) | Mandates, delegations, the collective's stewards | `author-lens`, `sets-authority-arc`, `HeadDelegation` | `notary-authority`; `authority-in-integrity` (red) | Staging declarer gate is open ("god-mode OPEN"); only `republish: author` is enforceable; no signed circle-admission record |
| **Notary** | Which conductors witness what my space does, and what do I see before it is witnessed? | Witness context to join (specced as a separate install act, absent) | Holding domains (households) | The notarized head; custody commitments | `notary-authority` (green) | T1 floor on every device; `clone_limit: 0` on every role, so no per-space cell exists yet |

### 5.4 Identity

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| **Key custody** | When something is signed in my name, whose key signed it, where does it live, and who told me? | Which acts need the person's present consent through a device they hold, against acts that may be delegated (absent). Not where the identity root lives: that is the network's | Device key; doorway custodial agent (hosted); hub (thin client, unbuilt) | `delegates-compute: hosted-cell`; `StewardshipGrant`; roster row | `hosted-cell-promised`, `graduates-onto-own-key`, `custodial-authority-answerable` (all red) | Device-held key driving a remote cell: absent. "The person is told the hub holds the key": a promise with no probe. No remote-signing design. A network-held root under which devices are enrolled: lineage substrate built, the mint path absent |
| **Recovery** | If I lose every device, who helps me back to myself, and what of mine is still there? | Nothing per app: the network's commitment to every person (ruling 6), not a feature an app opts into | Quorum witnesses; holders of pieces | `KeyRotation` (imagodei entry); a `governance-action:recovery-request` Content entry on the elohim DNA (formerly the `RecoveryRequest` entry type, now a plain struct used only for the signal payload); a mishpat `binds-identity` commitment | `identity-key-lineage-recovery` (`@wip`) | Lineage substrate built; no coordinator path mints a valid `KeyRotation`. Replicas surviving is not the person getting back in |
| **Agent standing** | Which agent may act for or beside me, what may it sign, and how will anyone see which party did what? | The agent work an app runs and its scope (absent) | Sponsor's peer hosts the key; providers lend berths | Hosting grant + capacity grants | `ci-runner-stewarded` (4 of 5 live twice); `acts-attributed-to-participants` (red) | Runner ruling built and running; no layered model identity; service agents have no reader tier for gated rows |

### 5.5 Value and supply

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| ◆ **Compute agreements** | Who lends compute to whom, under what bound, and can anyone read back that it was spent? | Compute needs resolved to grant scopes (absent) | Berth offers (absent); a static provider table | `delegates-compute`; `compute-fulfilled` event | `measure-runs-on-a-peer` (red) | `GRANT_SCOPES` is a closed list of three. No berth offer and no matcher in code. The `compute-fulfilled` event is a requester-side local record and is not notarized. For a delegated compute task the carrier notarizes the task's lifecycle on the DHT: `accept`, `reject` on decline, and on completion a `work` event that names the request in `fulfills` and carries the bound receipt (`content_store/src/compute_task.rs`); at launch it also writes a local `compute-admission` row. It records no `compute-fulfilled` event of its own, and no other kind of carrying has a carrier record |
| **Inference** | When my elohim thinks, whose machine runs it, and can I take that context back? | Inference need (absent) | `canInfer` (archetype flag) | An inference grant scope (absent) | None | An `LlmBackend` trait exists, with Anthropic, OpenAI and mock implementations. steward/node wires the agent service and mounts `POST /elohim/invoke`, but passes only a `MockBackend` (`steward/node/src/elohim_service.rs`). A real backend is reached only through `invoke_wisdom`, which picks it from an API-key env var (`wisdom.rs`), and only when gate-client's in-process transport is turned on by `ELOHIM_AGENT_WISDOM_TRANSPORT=in-process`. No node turns that on, so no node runs real inference. No model address; no counsel-context store or erasure |
| **Device capacity** | What can this device carry without hurting its owner, and what should it not be asked to carry? | Nothing per app: the device declares | Archetype facts (`battery`, `alwaysOn`, `canSteward`, `canInfer`, `canDoorway`) | Node-shape commit | `idle-is-free` (red) | Facts declared; no placement code reads `battery` or `alwaysOn`; the ark-core derivation is specced, absent |

### 5.6 Edges

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| **Reach** | Who is allowed to see this, and what turns a stranger away? | `defaultReach`, `minimumReach` (exist) | The requester's resolved identity and relationships | `acknowledges-reach-change`, `replicates-content.reach` | `reach-enforced-everywhere` (red; HTTP only) | Three vocabularies: the SDK reach enum (`private`, `self`, `intimate`, `trusted`, `familiar`, `community`, `public`, `commons`), which is content reach; `LocalityLevel` in the TypeScript client (`private`, `invited`, `local` … `regional`, `commons`), documented as placement scope; and the doorway's ladder, which uses the `LocalityLevel` names as if they were audience. The doorway has two reach gates, and they disagree. The live one is the public-name serve gate (`classify_reach` in `services/serve_eligibility.rs`): it serves `public` as commons, refuses `private` and `invited` as beneficiary-only, and treats every other tier, including the protocol's own `self`, `intimate`, `trusted`, `familiar` and `community`, as restricted, so anonymous visitors are refused and any requester who shows standing is admitted; the intimate tiers get no distinction of their own. The older cache gate (`can_serve_at_reach` in `cache/access_control.rs`) matches the locality ladder, so there `self`, `intimate`, `trusted`, `familiar`, `community` and `public` all fall to deny, and nothing on a request path calls it. The proxy's pantry stocks only content whose reach is absent, `public` or `commons`. The blob route serves on database lookup failure. No sealing for intimate reach. No reach on a tree root |
| ◆ **Web publication** | If strangers should read this on the web, who serves it, under what standing, and what do I lose if that doorway disappears? | "Projectable at reach R" (absent; `routeClaims` nearest) | Doorways, plural | `project-epr` commitment | `served-under-standing`, `doorway-failover` (both red) | No `delivery` section; no retention floor for a projection; invite and relationship checks not implemented |
| **Bridges outward** | If an outside network talks to this app, who speaks for whom, and can I refuse it for free? | Outward projection (absent) | The runtime the bridge links into | Governed offer with two-sided disclosure | Declared-versus-observed drift | Bridges are libraries, not runtimes; atproto and activitypub not built; who holds a person's remote identity is absent |

### 5.7 Lifecycle and proof

| Plane | Question | Declares | Supplies | Records | Proves | Today |
|---|---|---|---|---|---|---|
| **Membership** | When I join or leave, what happens to what I held for others and what I promised? | Nothing per app | Admission; the charter | `Membership` (joiner-written; admission checked by the coordinator only, not by validators); `revokes-commitment`; withdrawal | Leaving story (`hosted-human/05-leaving`) | A person joins a collective by writing a `Membership` entry they author and sign themselves (`affirm_membership`). The coordinator writes it only after checking a steward-signed invite token, which is held off the DHT (`issue_household_invite`). Nothing on the DHT is signed by the admitter, and validators check only that fields are present: neither the steward's signature nor the steward's authority, and `sponsor_cid` is an unverified string. Placement-to-withdrawal is a named missing node |
| **Space shape change** | When my household becomes a co-op, or an agent joins, does my app keep working without me reconfiguring it? | Nothing: the point is that the app declares nothing new | Re-placement by the matcher | The commitments that move | A re-placement probe (absent) | No matcher re-places commitments when a space changes shape. The only placement in code is blob salvage (`salvage_pass`, `elohim/elohim-storage/src/reconcile/custody.rs:458`). It is opt-in and off by default (`SALVAGE_CAPACITY_ENABLED`), acts only when a blob has fewer fresh holders than one node-wide target (`SALVAGE_TARGET_REPLICAS`), and then a node that ranks among the closest candidates adds itself as a holder. It never moves or releases a commitment |
| **Proof** | For each thing promised to me, what reads it back? | Nothing | Probes | Evidence ledgers | The habit register | §9 |

## 6. What the declaration gains

Every addition is a **need**, not a place. The rule of placement neutrality: if a field could only be
answered by naming a device, a host, a doorway or a hub, it does not belong in the manifest.

| Need | Proposed shape | Plane | Notes |
|---|---|---|---|
| `closure` | The release's start set: a tree root plus a view of path prefixes; the bytes under that view are the closure's content | Code residency, experience depth | The single joining field. Residency §3, the tevah design's `ProcessSpec.closure: [Cid]` (compute-envelope-tevah §3) and `replicates-content.closure_rule` all assume it, and none of them implements it. The ark-core type that was built is `ChildSpec` (`elohim/ark/core/src/manifest.rs`), named apart from `ProcessSpec`, and it has no closure field: its `artifact: ArtifactRef` pins one artifact by sha256 or names a channel. Whether `closure` lives in the app manifest or the release manifest is open (§19) |
| `runtimes` | The runtime roots the closure names, by channel | Code residency, head | Two apps naming one runtime share one copy by address |
| `works-offline` | The smallest depth (§12.2) that must still work with nothing reachable | Experience depth | Default: the closure kept |
| `passing` | Already specced (live session §9) | Live channel | Moves into the schema unchanged |
| `notify` | Notification kinds, not payloads | Attention | The SDK refuses an undeclared kind |
| `loss` | `device` / `household` / `region` per content type | Data holders | Resolved by the space to a holder count and a domain count. Not `quiltPolicy`, which is temperature |
| `indexed` | Content types and index measure | Derived indexes | Index is Ephemeral and rebuildable |
| `channels` | Channels the app follows; the space resolves which head | Head | A head is a declared dependency, not "latest" |
| `witness` | Whether using the app needs a witness context, and which | Notary | Opening a page needs none; joining a collective does |
| `signing` | Acts that need the person's present consent through a device they hold, against acts that may be delegated to a berth | Key custody | The operator's "where must trust be retained". It does not declare where the identity root lives: that is the network's (ruling 6) |
| `compute` | Named grant scopes the app's background work needs, with bounds | Compute, agents, inference | Names a scope from the closed list; an unknown scope is refused (`compute_grants.rs:62-73`) |
| `projectable` | The widest reach at which the app's content may be projected to the web, and the retention floor it asks for | Web publication | Does not name a doorway |
| `outward` | Outward protocols the app may project to | Bridges | Refusal is free |

Two proposed names are already taken in `app-manifest.schema.json` with other meanings. `closure`
names a gate's silence posture (`gates[].closure`, open | closed, the enum the protocol also uses for
the posture toward absence on every axis card). `indexed` names a per-edge-type boolean
(`graph.edges[].indexed`, create an `epr_edge` index). The needs `closure` (the release's start set)
and `indexed` (content types and index measure) are different concepts. They land under their own
nesting, a `needs` block or the release manifest (§19 question 1), or under a distinct name, not
as bare top-level keys that read as the existing fields.

`defaultReach` and `minimumReach` already exist and stay. `workingVersion` (kept, republish, passing)
is specced and moves into the schema as one block with `passing`.

## 7. Placement: a deterministic floor, an elohim ceiling

Placement is dynamic: needs, offers and capacity change all the time. Two rulings shape how the network
keeps up (operator, 2026-10-20).

1. **Every participant expresses, not only the app.** A need is an intent to receive and an offer is an
   intent to provide, in REA terms. Placement matches intents into commitments, and fulfilment is the
   economic events that follow. The app manifest is one source of intents among several.
2. **The floor is a deterministic controller; the elohim is the ceiling.** This is the floor-and-ceiling
   pattern the wisdom-layer design names as universal: the floor holds with no LLM, no network beyond what
   is visible and no human, and the ceiling interprets within the band the floor opens.

"The floor" written alone always means this placement floor. A projection's retention floor, the
holding floor a space's charter declares and tevah's exec floor are other things, and are always
written with their qualifier.

### 7.1 Who expresses what

| Participant | Needs | Offers |
|---|---|---|
| App (manifest, §6) | closure, `loss`, `passing`, works-offline, compute, and the rest of §6 (representative here; §6 is the full list) | — |
| Person | "keep this offline", "spare my phone's battery", a budget | "I will host for my household" |
| Device | its capacity facts: battery and always-on, declared on its archetype (`battery`, `alwaysOn` in `genesis/data/devices/devices.schema.json`); a metered link, today only as the network class (wifi \| cellular \| unknown) the device declares to elohim-storage at runtime (`POST /p2p/network-class`), which gates sync under `wifi-only` and is read by no placement code; nothing yet observes any of these | its berth |
| Space (charter) | holding floor, reach rules, what it owes its members | what its members have pledged through the pool, and no more (a pool is a party, not a person) |
| Agent | a task's compute, inside its grant | — |
| Carrier | — | posture, berth offer |

- **Posture.** What a peer will do for others: host live sessions, relay, carry bytes, serve as a wake
  ingress, run cells for thin clients, infer, serve as a doorway. It is a signed record of the device's
  enrolled key, held by those in its reach, so other peers rank from the last posture record they hold.
  Whether the peer is alive right now is a hint from the one liveness source, not part of the posture.
  Written when it changes, not on a timer (the rule; `idle-is-free` measures it and is red today).
  Changes are themselves held back by hysteresis: a device hovering around a threshold, such as a battery
  level, writes one change when it settles, so a posture does not publish a person's daily routine.
- **Berth offer.** A blade's REA intent: what it offers and holds, with its quota tier (tevah §3.1).
- **Where they are kept.** For public or commons reach, a posture and a berth offer are notarized on the
  elohim DNA's existing `Intent` entry (provider set, receiver none; `ServiceOffer` is the other existing
  candidate, and one of the two is picked once as the single home). Inside a household they are
  household-local signed records, written on change with hysteresis and read by the household's own floor;
  they are not notarized, because the shared DHT has no per-household cell (`clone_limit: 0`; §10 item 19).
  The SDK schema for either is absent (`schemas/v1/intents/` holds three other intents).
- **What stays on the device.** The person's private intents ("spare my phone's battery", a budget) and the
  device's observed facts (battery level, a metered link) are device state. Only their effect crosses, as a
  narrowed or withdrawn offer. The household sees "not offered", not "battery at 12%" or "over budget". A
  refusal names its reason to the device's own person; to everyone else it reads "not offered".

"Berth" names three things today. The contract means tevah's **blade offer**. The workspace lease store
(`genesis/agentic/bin/berth`) is the dev-workspace slice of it, and ark-core's `Berth` struct is the
placement input. A line that says "berth" without one of these is ambiguous.

### 7.2 The floor: a controller on every peer

Like a Kubernetes controller, the floor is a reconcile loop: it compares expressed intents with observed
commitments and holdings, and acts on the difference. Unlike one, it has no control plane. Every peer runs
the same loop over **what it can see**. The ranking is a pure function, so two peers that see the same
intents, offers and holdings name the same candidates, the way heads and the salvage holders are already
elected. Peers do not always see the same things: offers arrive by gossip inside a freshness window
(`salvage_capacity::list_fresh` today), and today's salvage pool counts the peer itself before its own
offer has round-tripped (`build_salvage_candidates`, `services/salvage_commitment_author.rs`). A first
guess can therefore over-adopt or under-adopt, and the space converges by reading the honoured count and
acting on the difference, not by every peer guessing alike. The probe measures how long convergence takes
and how much excess it causes; it does not assume agreement.

- **Reads only what it holds.** The floor ranks over records this peer already holds, and does not fetch
  before ranking. A fact it does not hold (a posture, a consent, a relationship, a reach record, a newer
  parameter set) makes the candidate it concerns ineligible as unknown, not eligible by default. When that
  fact arrives, its arrival is an event that runs the loop again. No generalized floor exists yet (slice
  5), so nothing holds this rule today; slice 5's floor is built against it.
- **Rank.** For each unmet need, a pure function ranks the eligible candidates. Same inputs, same order, on
  every peer, with a stable tie-break. The test contract is the salvage strategy's, binding on any ranking:
  determinism, total order, no duplicates, and agreement between any two peers given the same inputs. The
  ranking method is named by CID, and the parameter set names the method it parameterizes. A peer adopts
  only under the method the in-force parameter set names; a peer running another method (an older storage
  release, say, with `XorDistanceStrategy` where others run the household-diverse one) observes and does
  not adopt (concern C10). Today the strategy is chosen in code and named by nothing.
- **Eligibility comes first, and ranking does not change it.** A candidate is eligible only inside reach, inside
  its capacity, with its posture declared, and with consent where the need touches another person. An
  input the floor cannot read makes the candidate ineligible for that need, and the refusal names the
  unread input. Reach is a rule a carrier applies to another requester, read from records it holds: when
  the reach record is unknown it refuses that requester and does not go and fetch it, and a person's own
  device reading its own store does not check reach against itself.
  Battery and metered links: carrying for another household or space is off by default on a device on
  battery, on a metered link, or not always on, and its person may lift that default with their own
  posture. Holding within the device's own household is not refused for battery; it is deferred to a
  charging or unmetered window by hysteresis, so a two-phone household with no hub still holds its own
  bytes on both phones. A person's offer widens their own device's posture and does not narrow another
  member's floor.
  What the floor can read today is narrow, and the design says so. It reads one consent (a node's opt-in
  to carry for others, `SALVAGE_CAPACITY_ENABLED`) and one domain key (a household, where membership has
  been projected into `humans.household_id`). It reads no reach between peers, because every sync edge is
  classed `public`; no capacity fact, because neither `XorDistanceStrategy` nor the household-diverse
  strategy reads `spare_bytes`, `archetype`, `battery` or `alwaysOn` (placement.rs,
  `mvp_ignores_household_and_capacity_fields`); and no posture, because the schema is absent. Until each of
  these is readable, the eligibility the floor applies is the opt-in alone, and the battery default above
  does not yet hold (§15).
- **Self-adopt.** A candidate that finds itself among the chosen adopts and authors the commitment. No peer
  assigns work to another. This is how salvage custody already works (`reconcile/custody.rs`). Self-candidacy
  is admissible only when the peer appears in the pool from the same shared source every other peer reads
  (its own published offer), not injected locally; a peer that cannot yet see its own offer does not adopt.
  Today's salvage breaks this rule by inserting itself into its own pool for first-tick convergence, and
  generalizing it closes that (concern C1). Adoption carries a deterministic commitment id over (need,
  provider, resource root), so a second adoption is a no-op (concern C6b; salvage's
  `deterministic_custody_id` is the instance, and forked `custody-blob` ids have been recorded on the
  household mesh). Over-adoption from divergent views is expected and converges: the holders outside the
  agreed ranking release, lowest-ranked first.
  Live-session host choice, which is specced and not built, works the other way: each person in the
  session ranks the circle's standing list of hosts the same way and connects to the two highest that
  answer. A host does not choose itself (live-session §7, concern C1).
- **Observe and converge.** Peers read the honoured count and act again only on a difference. Today the
  honoured count is the holders whose own fresh inventory claims the bytes (`peer_blob_inventory`), and a
  claim is not a proof of holding: until a holding probe exists, the floor treats honoured as claimed and
  says so. A carrier is chosen and credited by agent key. A peer binding may decide where to dial, not whom
  to credit, so a placement whose bytes move over a self-asserted binding is recorded as placed, not
  honoured, until the binding is cross-signed (`transport_resolve.rs`'s source-2 fallback is the live
  case: it can push bytes to a peer other than the one the record names).
- **Event-driven, with hysteresis.** Re-evaluation follows a change — a posture, a membership, a miss, a
  carrier lost — not a polling timer, so a peer at rest costs almost nothing (`idle-is-free`: within the
  household-at-rest budget of 6 conductor calls a minute per peer). Today's salvage runs on an interval
  ticker (`salvage_recheck_seconds`), so it does not yet meet this rule (§15). A placement that holds is
  not moved for a slightly better one. This rule is new: no existing rule gives placements hysteresis. It is an analogy
  to the trust contract's "heal fills, never moves" (invariant I2; concern class C2, policy
  `c2-monotonic-authority@2`), which forbids a heal with weaker information from overwriting a declared
  head. That policy says it is not a placement rule, so the placement contract states and enforces its own
  hysteresis. A ratified parameter set (§7.3) is the one declared trigger that may move a placement that
  holds. It moves make-before-break: the newly chosen holder adopts and is honoured before the old
  commitment is released, at most a declared number of moves per event (concern C6a).
- **Leases are long-lived.** A commitment carries a lease, renewed by nothing periodic: it lapses on an
  observed carrier loss or a `revokes-commitment`, not by re-authoring on a timer. A lapse re-places before
  it releases: a carrier lets go of bytes carried for someone only after its floor sees another holder
  honouring the same need, or after the recipient withdraws. A recipient being unreachable does not by
  itself end what is carried for it, so a person offline for a month comes back to what their hub held. A
  device does not let go of its last copy of something because a remote holder exists, unless that
  holder's commitment is current in records the device holds.
- **Nested by membrane.** The device reconciles the person's private intents, which stay on it (residency §3: residency is device
  state and is not sent anywhere). What a device's floor publishes is its posture and its berth offer,
  not the intent or the observed fact behind them (§7.1).
  The household reconciles over its members' household-local records, and the collective and commons
  see only public offers and commitments. Nobody's context is pooled. A need crosses households only at the
  reach it was expressed at. Until peer-side admission is validated (circle admission, §15), the
  household's reconcile is a public baseline, not an intimate one: what it publishes is readable by
  anyone in the cell, which is why below-public placements stay household-local (next bullet). A carrier
  whose household is unknown counts as its own failure domain, so a pool with no household data spreads
  replicas by key distance alone and promises no household diversity.
- **A commitment is kept at the reach of the need it answers.** Below public reach it is written only in a
  cell the space itself validates. Until per-space cells exist (§10 item 19; `clone_limit: 0` on every
  role), the floor records intimate- and trusted-reach placements as household-local signed records that
  are not notarized, read back by the household's own probe, and authors a notarized REA `Commitment` on
  the elohim DNA only for needs expressed at public or commons reach. A placement commitment on the shared
  DHT does not name a below-public blob, session or relay pairing. Today salvage writes a notarized
  `custody-blob` per blob on the shared DNA (`services/spool_custody_author.rs`) whatever the blob's
  reach; that is the debt the generalized floor migrates (§14, head-plane cost).
- **For a hosted person**, the device-level floor runs in their browser over the browser's own store. The
  doorway runs the floor only for the cell it hosts, does not hold the person's private intents, and is not
  a party the person's device must reach to keep what it already holds. The doorway holds their key and can
  see and sign what passes through that cell, and the person is told so (§11).
- **Depth is a placement too, and so is a role.** When a person has not said how much of a tree root to
  hold, the floor derives a default from the device's facts and what the space offers. A Chromebook with
  a small disk, on battery, in a coding collective whose hub holds the repository, opens it sparse or on
  read. A workstation with room opens a full clone. Each is a first guess the person can override, and
  either can deepen or fall back later. The same facts make a device *eligible* for a role: one left on
  all the time, with room and bandwidth, is eligible to host live sessions or carry for others. Eligible
  is not enlisted. The rule is that a role is taken only from a posture its person declared. Today one
  role enforces it: salvage custody adopts only on a peer that opted in (`SALVAGE_CAPACITY_ENABLED`; the
  consent gate at `reconcile/custody.rs:532`). For every other role it is this design's eligibility rule,
  unenforced until that role's adoption path is built, and each path is built with the same gate. The
  ceiling may suggest it ("this machine is always on; you could host for the
  household"), and the person decides.
- **Scarcity is named, not hidden.** When needs exceed offers, the floor serves in the order the space's
  members ratified (for example a care floor first). Where none is ratified, which is every household
  today, it serves in the order of the floor's built-in parameter set and says that no priority has been
  chosen. Each unserved party's own floor derives "not carried", with its reason, from the commitments it
  can see (an Ephemeral answer). Nobody sends it, so a shortage costs no messages, reveals nothing to a
  carrier, and is shown at the reach the need was expressed at and no wider. "Not carried" always arrives
  with its three parts (ruling 7): why, when the floor will look again, and what the device can do on its
  own meanwhile. The answer is a missing node today, and so is the charter-update path a ratified order
  rests on (§15).
- **Alone.** With one device and nothing reachable, the floor runs over that device's own intents and
  holdings. It keeps the closure, honours "keep this offline", and records each need it cannot meet here as
  not carried, named, with what it is waiting on. An unmet need does not withhold what the device can do by
  itself: a write whose loss model is not yet met is kept on this device and shown as "held on this device
  only". When a peer arrives, the arrival is the event that runs the loop again.

The floor's outcome is the baseline placement. It needs nothing beyond the peer that runs it, so it is
available wherever that peer runs. Without an elohim a space loses the sensing and drafting of tunings,
not placement, and not its participants' power to draft and ratify a tuning by hand.

### 7.3 The ceiling: each member's elohim drafts, the participants tune

There is no space-elohim and no pool-elohim. Each member's elohim works for its own person, and a pool's
elohim council holds what is held in common (below). They read outcomes, draft and give first notice. They
propose; the participants decide. A placement is the floor's and ratification is the participants' act
(elohim-commons-design).

Each member's elohim reads the pressures the floor's outcomes create, but only those its person may read at
the reach the space declares: its person's needs refused, commitments its person holds or relies on that are
under-held, churn among its person's carriers, the cost its person bears. It carries its own person's stated
concern and no one else's; another member's concern reaches a draft only through that member or their
elohim. It reads "cost borne unevenly" only from the per-household aggregates each household publishes at the
space's reach, not by joining fulfilment records across households. From these it drafts a change and
carries it to the people it touches. A draft is a proposal with evidence, not a tuning. There are three
kinds, each with its own author:

1. **A participant's own offer or posture.** The elohim may draft it; only that participant posts it. The
   elohim may carry a narrowing (withdrawing an offer, sparing a battery) inside a standing mandate its
   person gave. It does not widen what its person provides.
2. **A charter amendment**, including the order of service under scarcity. Only the charter's own amendment
   rule enacts it. It is a constitutional record and is not carried by a parameter set. The path it rests on,
   updating a collective's charter, is a missing node (§15).
3. **A placement parameter set**: ranking weights and tie-break salts among eligible candidates. This is the
   only thing the ceremony below puts in force.

**The ceiling changes the floor's inputs, not its outcome.** That holds by construction: the ceiling has no
write path to a placement, only to a parameter set the floor reads. A ratified tuning is a declared,
versioned parameter set, addressed by content, naming the ranking method it parameterizes, and carrying the
ratification record that put it in force. It is published at the space's reach (who may see it) and in
force for the floors of the space's members (who reads it): two facts, not one. A floor reads a parameter
set only when the head naming it carries a ratification the floor re-derives itself (concern C5); an
unratified draft is not in force, whatever it changes. A need is ranked under the parameter set of the space
whose membrane it was expressed in, and an inner space's set does not relax an outer space's eligibility.
This settles the fork the salvage design left open ("preserve determinism-agreement or move to a coordinated
authoring model"): coordination happens on the inputs, and authoring stays coordination-free.

**The floor does not wait for the ceiling.** Each peer's floor reads the newest ratified parameter set it
holds for the space. When it holds none, it reads the floor's built-in parameter set, which ships with the
floor and is itself addressed by content. Peers holding different sets may briefly disagree. Each commitment
names the set that produced it, and hysteresis keeps an honoured placement until the newer set reaches its
holders. The floor does not wait to learn whether a newer set exists.

- **Within the band.** A parameter set may only reweight or reorder eligible candidates for a need. It does
  not make an ineligible candidate eligible, because eligibility is evaluated before ranking and takes no
  tuning input: reach, capacity and consent are the floor's. It does not change the order in which needs
  are served, lower a holding count below the declared `loss` or the charter's holding floor, relax household
  diversity, or change a participant's own offer. Nor does it remove an eligible candidate: a weight does not
  reach zero, and a parameter that would leave a member's carrier unchosen for every need is an exclusion,
  which the members decide in their own arena. A parameter that weighs a carrier by its record (churn,
  missed honours) is a consequence. Its first rung is a notice to that carrier's person from their own
  elohim; any rung past notice is authored as a lens the members agreed to, and the ranking reads it only
  once that lens is ratified. The floor watches commitments honoured, not the person behind a carrier.
- **What is the elohim's and what is not.** Sensing pressure, reading outcomes, drafting a parameter set
  with its evidence, and telling its own person first are the elohim's. Putting any parameter set in force
  is the participants' act, every time, by the space's ratification rule, whether or not a commitment moves
  yet: a parameter the floor reads is a rule. Every parameter set put in force is ratified by the
  participants it touches. Each draft names its drafter's layered identity (base model, tuning layer,
  constitutional prompt), and the ratification names the people who decided. A carrier that can sign as the
  person (a hub runtime, a hosted doorway) is chosen by the person, not placed by the floor (§11).
- **At device scope**, the person alone ratifies a tuning of their own device's floor, and that parameter
  set is device state that is not published. Only what a person chooses to state at a space's reach enters
  that space's parameter set. A tuning is not derived from a person's revealed preferences or private notes
  and then published.
- **For a pool**, the facility (members' bytes, compute and labour) is members' produce. Its placement
  parameters are ratified by the members, one human one vote, and members' elohim draft them. The pool's
  elohim council holds custody of the commons share and the artifacts held in common, and may propose within
  its charter. During the network's infancy humans perform that council's role. Human sortition councils,
  drawn to the reach of the decision, audit and govern every contestable result and may overturn it.
- **Lineage on every draft.** A draft names the elohim lineage that wrote it, and a space may require drafts
  from more than one lineage before ratification, so that one model under one prompt is not one judgment
  repeated at every member.
- **Reversible.** A tuning is a declared head like any other. Reverting is a new declaration whose value is
  the earlier set's CID, ratified the same way; the head does not move backward (concern C2). The moves it
  causes are make-before-break, as for any ratified set (§7.2).

None of this ceremony exists yet. The ratification path is a missing node (§15), and until it lands the
floor reads only its built-in parameter set.

### 7.4 Proving it

Every placement the floor makes is a commitment, and every tuning is a declared head, so the probes of §9
read both. The person can always ask why (ruling 7), and the answer is bounded by reach. It covers which of their needs, the
eligibility reasons that applied to their own devices, how many candidates were eligible, which ranking
method and which parameter set (each by CID) produced the choice, and the chosen carriers, named only at the
reach those carriers declared their posture. Other members' refusals are counted, not listed, so the answer
does not show the asker whose device is on battery. An answer the floor cannot explain within that bound is
a defect. Where the full answer is no longer recomputable, the person still gets ruling 7's three parts:
why it cannot be given, when it can, and what they can do now.

What remains to explain it: each commitment the floor authors carries the parameter-set CID, the method CID
and a digest of the eligible set. The full explanation is Ephemeral, recomputable while its inputs persist;
after that the commitment's three fields are what remains, and the floor says so. No such record is built
yet.

## 8. What gets recorded

Every carrying rides an existing Mishpat action. The actions that exist: delegates-compute,
acknowledges-reach-change, replicates-dwelling, replicates-content, revokes-commitment,
ratifies-limit-gradient, sets-authority-arc, author-lens, binds-identity, migrates-lineage,
sunsets-lineage. The grant scopes that exist: `sweettest-feedback`, `hosted-cell`, `measure-stage`.

The contract adds **grant scopes, not entry types**, for the authority a carrier needs: `live-session`,
`relay`, `wake-ingress`, `hub-runtime` (a cell run for a thin client), `inference` and `index-fold`. A grant
is authored by the party whose presence, data or key it touches, before the carrier is eligible. The
carriage itself is a separate REA commitment the carrier self-adopts (§7.2). A grant is consent and
eligibility; a commitment is placement; neither stands in for the other, so a carrier does not grant itself
authority by adopting. Each new scope is one entry in elohim-storage's closed list (`GRANT_SCOPES`,
`api/compute_grants.rs`) plus the storage-side consumer that requires it. Neither Mishpat zome checks a
scope's value: the coordinator requires only that `scope` is present, and the integrity zome checks only
that the bounds keys are present. So other peers validating a grant cannot check scope membership; the
acting storage node does (concern C12). Moving scope validation into integrity would move the DNA hash and
is not done here. The contract also adds the `runtime-offer` kind for interactive runtimes; it is a kind,
not a grant scope, and like every new action kind it stays out of integrity (§14).

Three record gaps cut across every plane:

1. **Fulfilment is the weakest link.** `compute-fulfilled` is a requester-side local record and is not
   notarized (its `dht_anchor_hash` is empty), and nothing binds it to the carrier's own record. For a
   delegated compute task the carrier does record carrying: the selected runner authors signed accept,
   complete and decline EconomicEvents on the DHT, and its completion fulfils the request and binds the
   receipt to the task, request and grant (`content_store/src/compute_task.rs`). No carrier record exists
   for any other kind of carrying: hosted bytes, relay, doorway or hub. A liquid system that cannot show
   what was carried is a promise with no probe.
2. **A collective cannot be a party.** Grants are bilateral: a compute grant's provider is the serving
   node and its recipient must be a Holochain agent key (`compute_grants.rs`). Today a pool could only be
   built as every member granting to every other member (commons-pool spec §1).
3. **No signed record that validators check admits a person to a circle.** Qahal `Membership`
   (imagodei `qahal.rs`) records that someone joined a collective, with a sponsor and a withdrawal height,
   but the joiner writes it on their own chain. The steward's invite signature lives only in an
   out-of-band token and is checked in the coordinator (`affirm_membership`), and validators check only
   that its fields are present. So it cannot yet stand as circle admission. The live channel,
   `republish: circle-agreement` and document custody all wait on a record whose admitting signature
   validators check.

## 9. What proves it

One probe per promise kind. A promise listed in §5 with no probe here is a trust defect, and the person
is shown it as unproven, not as kept. The rule: absence reads as unmeasured, not as healthy. Today one probe breaks it (§18, the heartbeat).

| Promise kind | Probe | Read by | State |
|---|---|---|---|
| Held bytes, each with a named reason | Residency holds view: the `holds` key of node-local `GET /admin/adoption` and the `elohim_node_holds_*` gauges (`services/holds.rs`) | The device's own person only; it stays on the device (the doorway's projection of the adoption report does not carry it) | Built: every storing path writes an arrival record (`db/blob_arrivals.rs`), and a placed shard records the peer that placed it. Pin-to-bytes is absent: an item pin holds only through the content row it names, and nothing expands a pin's closure into its blobs |
| Carried for others | `PeerCapacityView` pledges against `actuallyHeld`; the commitment-backed card | The carrier and the party carried for | Partly built; `PeerCapacityView` is served by peer id with no owner check |
| Compute carried | `GET /api/v1/compute/grants` (node-local, local-compute token; lists the delegates-compute grants this node's own cell issued, in any state). No envelope probe of carrying exists: `ark describe` prints the berth passport (what is offered), not what is carried | Grantor only (a grantee cannot list grants it received) | Grantor's grants list built; grantee read and carrying probe absent |
| A runnable app | `DeliverabilityVerdict` with its failure layer, the same at every tier | Anyone installing | Partly built: the peer-side verdict (`DeliverabilityVerdict` Boots / Broken / NotJudged, in elohim-storage `app_deliverability.rs`) is judged and stamped on app surfaces, and a pre-push receipt leg exists. Still only specced (2026-09-05 layered-fallback spec): the failure layer and the same verdict at the doorway and every other tier |
| Reach | An unauthorized read refused on every path: storage, discovery, projection, doorway cache | Peers attest | Partly built, HTTP egress only (`reach-enforced-everywhere` red): `dataplane/reach-enforced-http.feature` (`@concern:reach-enforced-http`) runs three live scenarios, each asserting that an unverifiable bearer or a self-asserted identity header grants nothing beyond anonymous; three more (blob byte route, head PUT, app bundle) are `@wip` until a fleet build carries the fix. The feature is `@act:i`, so the household lane is its authority, not peer attestation. No probe for the sync, discovery, shard or projection planes |
| Head and convergence | Seam smoke, conductor diagnostics, "canonical head propagated" | Anyone (public reads) | Built |
| Liveness | One writer of presence | Peers | `liveness-has-one-source` red |
| Runtime death | The ark death witness, offered to custodians | Custodians | `runtime-death-witnessed` red |
| Leave and withdrawal | The holder lets go after the placer withdraws | The leaver and each holder | Absent |
| Health transitions | Transition attestation and its reader | Peers | `health-attests-on-transition` red |
| Fixture against real | `evidence_class` | Settlement | Unwired |
| The person was told who holds their key | A disclosure record created when an app opens on a hub | The person | Absent |
| A carrier at rest costs almost nothing | The household-at-rest budget (`dataplane/household-at-rest.feature`): on a settled household with nobody acting, over 300 s, at most 6 conductor calls a minute per storage peer, no permit refused, and at most 10 CPU seconds a minute across the household | Anyone | `idle-is-free` red |
| The bytes layer reads only what is read | The graduation trigger of this design | The reader | Absent (habit `materialize-what-is-read`, unwired) |

Two choices per probe are made when it is built, not by default: whether the promise is read by its
own holder only (the holds view, for privacy) or attested by peers; and whether `evidence_class` must
exist before a pool's first receipt is accepted.

## 10. Composition: the fusions to refuse

Each of these is a design that looks simpler and is wrong. Every one was found in the repository as a
live risk.

1. **Reach, holders and publication are three planes at the edge** (complementing §2's reach, head and
   replication). Custody is reach-blind by design. A holder may
   carry what it cannot read; a holder set is not an audience (custody is reach-blind by design, `reach-enforced-everywhere`); published is not resident.
2. **Chat is not presence.** A message is a record; presence is passing state that is not stored (the live-session rule).
3. **Host, relay, doorway and hub runtime are four carriers** with four trust stories. None of them is
   the others.
4. **Temperature is not copy count.** `quiltPolicy` says how warm; `loss` says how many and how
   diverse.
5. **Placement is not authority.** An agent on a berth holding the bytes is not thereby authorized.
   Authority travels as a grant.
6. **A berth that runs a conductor is not the notary.** A space's witness count is the number of
   independent holding domains that validate, not where one process runs.
7. **A replica is not recovery.** Bytes surviving without the key leave the person locked out of them.
   And **a device is not the identity root.** A person's identity root living only on one device is the
   anti-pattern: losing that device would lose the person. The root is held by the network (key lineage
   under a recovery quorum), devices are enrolled under it, and losing every device does not lose the
   person. That is ruling 6's commitment; its mechanism is a missing node (§15).
8. **Installed, head adopted and resident are three facts.** The closure is a bytes fact; the head is a
   head fact.
9. **Hosted is not opened from my hub.** Both hold a key for a cell; they are different holders, and the
   person is told which.
10. **A bridge is a library, not a runtime.** It lands on the doorway (web2) or storage (protocol).
11. **The actor's custody tag rides with every reach check** (its shape is decided when the materialization
    record is built). Reach checked against a key the doorway
    holds is only as strong as the doorway. The materialization record names whether the actor's key
    was device-held, hub-held, doorway-held or a sponsored agent's.
12. **Agents stand on grants, devices on the roster.** A check of who may act keys on one or the other,
    not one record for both. An agent is not given a device binding to make things easy; the runner story
    shows a runner's device approval refused live (`compute/ci-runner-stewarded.feature`).
13. **An index is not a holding set.** A group's index and a peer's holdings are different sets.
14. **Footprint tuning is native-binary-only.** Do not vary wasm or a DNA per device: a different hash is a different DHT.
15. **Uninstall is not leave.** Uninstalling leaves commitments and promised custody in place; leaving
    revokes or lets them expire. The contract needs both verbs.
16. **Leaving is four acts, not one delete.** Reading stops (reach), carried bytes are withdrawn
    (replication), grants are revoked and berth leases released (compute), and the membership record is
    updated (head). Copies others hold are not erased and expulsion does not promise deletion.
17. **Closing a doorway account is not leaving the network.** It reclaims the doorway's resources only.
18. **Membership count is not need.** A space earns its own cell by the written test (commons-reach
    spaces D6), not by size.
19. **"Private" is not promised before admission is validated.** Peer-side admission validation is
    unbuilt, so a household or circle cell is a public baseline today, and the contract says so rather than
    calling it private.

## 11. What each carrier learns

Every carrier sees something about who takes part, and one statement per carrier kind says what. It serves
the live channel, attention, connectivity, publication and the bytes layer at once. A carrier that can sign
as the person (a hub runtime, a hosted doorway) is chosen by the person, with that statement in front of
them, and is not placed by the floor: choosing it stakes the person's standing, which is their own act. For
every other carrier the person's elohim gives the statement on demand, and gives it unasked only when a
carrier kind first enters the person's path, so the floor's continual choices stay quiet.

The "What bounds it" column is the design. The "Today" column says which of those bounds exists; a bound
that is not built is not presented as one that holds.

| Carrier | What it learns | What bounds it | Today |
|---|---|---|---|
| Live-session host | Who is in a session and when; what passes through it | Hosts are circle members under a `live-session` grant; nothing is stored | Grant scope absent; no host built |
| Relay | Who dials whom, and when | A relay forwards between peers reachable by key and holds nothing; who runs it is recorded | Who runs a relay is not recorded (§15, relay → commitment) |
| Wake ingress and OS push service | That a person was woken, when, and by which service | The push carries no content and no collective name; the person pulls the signed act over their own key | Undesigned; the push service cannot be fully bounded (§14, §19 question 6) |
| Hub runtime for a thin client | Everything the cell does, and it can sign as the person | Chosen by the person; declared as a stage, disclosed when opened, ended by graduation | The disclosure record is absent (§9) |
| Doorway | What it projects; for hosted people, everything they do | Narrow web2 role; a retention floor per projection; plural and swappable; for a hosted person, chosen by the person | The retention floor is absent (§5.6) |
| A tier serving bytes | Which addresses a requester reads | Reach before bytes, on the requester's side and the tier's (§12.3 rule 2); where a household tier exists it absorbs repeats, so an outer tier sees at most one miss per object per household, under the household tier's key | No household tier exists (`elohim-hub` has no code), so each device's miss reaches the outer tier under its own device key |
| Query tier (depth 0) | Which paths, listings and diffs a requester asks of a root, not only addresses | Reach on the root before any answer; it is in the requester's own tier list only; it keeps no log past the request | Not built (slice 2) |
| Commitment record | Who carries for whom, under what bound, and when | A commitment's reach is its need's reach (§7.2): below public it stays household-local; across households it names agent keys and a tree root CID, not a path or a person's context; fulfilment is credited only over a cross-signed binding | Salvage writes a notarized `custody-blob` per blob on the shared DNA whatever the blob's reach |
| Inference berth | Everything the person's elohim reasons about while it runs there | An `inference` grant scope; context erasable on revoke (§15); the person chooses the berth and the floor does not place it | Grant scope absent; no node runs real inference (§5.5) |
| Pool | What its members committed to it, and nothing more | By the pool's duties a pool does not hold attention, preferences or private notes, and compute goes to the data (commons-pool spec §5) | Compute fulfilment is not notarized (§8) |

The aggregator behind the sensors is the dragnet (the SDO/RWA test). A design that lets one carrier kind
join what two others learned is refused, not mitigated. That includes the commitment record, which is why
it carries no more than the reach of the need it serves.

## 12. The bytes layer: materialize what is read

The operator's order puts eprfs first: it is the lowest primitive, and the code residency, experience
depth, data holders and derived index planes all stand on it.

### 12.1 What transfers from Scion and GitLab, and what inverts

Transfers:

1. Immutable, content-addressed objects that any holder can serve and every reader verifies. Content
   addressing and reader verification are already true of eprfs trees and blobs: `fetch_checked` rejects
   any bytes whose CID does not verify, whichever `EprfsStorage` returned them, so a holder needs no
   trust. Serving by any holder is not yet true: only local holders exist (`MemoryStorage`,
   `DirectoryStorage`), and a remote `EprfsStorage` (the unimplemented `StorageEndpoint`) is still to be
   built.
2. Fetch walks from a root: missing trees, then missing blobs. The requester does not say what it has. The
   protocol is resumable, needs no packfile, and gives sparse and virtual checkouts for free.
3. Every tier answers the same calls, the client does not know the topology, and tiering is strict, so a
   cache failure cannot flood upstream.
4. History and other questions about a root are answered by a tier as an API. Many consumers need no
   working copy at all.
5. Derived data is computed from its parents' values and keyed by its input.
6. GitLab: an isolated workspace per agent session, discarded on failure.

Inversions:

1. **Always online → local-first floor.** The kept closure is the offline promise. Every tier adds and
   none conditions. (Operator ruling, 2026-10-20.)
2. **One authority for bytes and heads → two planes.** Which root applies is a declared or elected head.
   Bytes come from anyone. A tier that serves bytes gains no say over heads.
3. **Any cache may serve any object → reach before bytes.** Addressing proves integrity, not
   authorization.
4. **Refs as one global row → no global ref.** brit integrate stays a local compare-and-swap on the
   holder.
5. **First-parent delta changesets → not adopted.** eprfs trees are snapshots with structural sharing.
   A sealed change stores new objects only for the changed paths and their spine, because unchanged
   subtrees keep their CIDs and an existing object is not rewritten (changing bytes changes the CID), so the new bytes follow the
   change. The work does not yet: the current sealer, brit's `seal_git_tree`, reads, hashes and re-puts
   every git tree and blob, then re-verifies the whole closure, so its work follows the tree. An overlay
   seal over a sealed base that reuses the unchanged subtrees' CIDs, so that work also follows the
   change, is the design in §12.2 and is not built.
6. **Bespoke, fit for a very large monorepo → must fit a phone.** The same primitive runs where there is
   no virtual filesystem.

### 12.2 The primitive: a root, a view and a depth

A materialization is three things:

- **root** — a tree CID. eprfs does not decide which root; a consumer passes the root a head names: a brit
  commit's sealed tree, a release's bundle tree, a devspace seed, an app's closure.
- **view** — what the act needs, as path prefixes under the root. A release's closure is a view; a sparse
  checkout is a view; an agent's read set is a view discovered as it reads.
- **depth** — how much of the view becomes bytes here.

| Depth | What is here | Works offline | Who uses it |
|---|---|---|---|
| **0 Query** | Nothing. A tier that holds the trees answers questions about the root: read a path, list a directory, diff two roots, history | Only if this device is the tier | Gates looking up evidence, CI checks, agents asking questions, "opened from my hub" |
| **1 Sparse** | The view's bytes; everything outside it absent and shown as absent | The view | Sparse checkout, a build on a berth, an app in use |
| **2 On read** | Every path appears; bytes arrive on first read, verified, and stay until let go | What has been read | lvi devspace, an agent session over a whole repository, the browser's service worker |
| **3 Kept** | The whole view, not let go automatically (residency's rule; the retention pass does not read pins yet) | All of it | Residency "kept offline"; a hub carrying a window |

Depth 3 is not an eprfs state: it is the residency device pin (`acquisition_pins` in elohim-storage),
and eprfs holds no pin. Today no eprfs code reads a pin, and the pin names a head ref, not a closure.
Pointing the pin at the closure and having eprfs read it is a node this design adds (§15, pin →
closure). "Registered with" and "installed" are depth 0 and depth 3 of the same app's root — the install
ladder the person already sees (register, then keep), stated once for every kind of bytes.

A let-go at depth 2 does not drop overlay bytes, and does not drop an object whose only known holder is
this device.

Writes at any depth go to an overlay. Sealing an overlay makes a new root whose cost is the changed paths
and their spine; dropping it costs nothing. GitLab's per-session workspace and brit's `snapshot seal`
and `restore` (whole-tree today) are this one shape. An overlay lives on the writing device, or on its
author's own devices. At depth 0, writing a path first brings that path and its spine to depth 1 on the
device, so an overlay does not live only on a tier, and the working-version rule in §2 (a solo draft is not
sent to a doorway) holds at every depth. An agent's overlay lives on the berth the agent runs on, under its sponsor's lease, and
is the agent's draft, merged by the person whole or in part. No overlay is built yet (§15, agent session →
draft root).

### 12.3 One fetch path, every tier the same

The `EprfsStorage` trait exists (`resolve_epr`, `has_blob`, `fetch_blob`, `put_blob`,
`put_blob_verified`, `publish_attestation`), but today only the local adapters in eprfs-storage implement
it: `MemoryStorage`, and `DirectoryStorage`, which does not resolve EPRs. No storage, hub, doorway or
browser tier implements it yet. The design makes every tier implement it and adds two calls:

- `walk(root, view, budget)` — a stream of verified tree nodes, descending only into entries under the
  view; each node is verified against the link in its parent; `External` entries are returned as
  boundaries; past the budget it refuses. An `External` entry is a tree entry that names something outside
  eprfs by a kind and an opaque reference (a git submodule's gitlink is one); the walk reports it and does not
  descend into it or fetch it. The view is a set of `ProjectionPath` prefixes (the existing validated
  relative-path type, `eprfs-core/src/projection.rs`). The budget is the existing `TreeLimits`
  (`eprfs-core/src/tree.rs`): per node `max_node_bytes` and `max_entries_per_node`; per walk `max_depth`,
  `max_path_bytes` and `max_pending_entries`; and the closure totals `max_total_entries` and
  `max_total_bytes`, which here are applied to the visited spine rather than to the whole closure.
  A node under the view that no tier in the list holds is yielded as an absence naming its path, its CID
  and the tiers tried, and the walk continues with every other prefix of the view. Under
  `FetchPolicy::LocalOnly` absences are yielded at once and do not wait on a tier; nodes this store holds
  stream first, and a remote tier does not delay a local node. So a partly held tree degrades offline
  instead of failing whole. Slice 1's walk yields these named absences locally (`WalkItem::Absent` in the
  working tree's `eprfs-core/src/walk.rs`; not landed).
- `fetch_blobs(root, cids, policy)` — a batch stream, resumable and out of order; each blob is verified on
  arrival before it is stored. Each CID is served only when it lies under a root the requester's reach
  admits, as the tier's own walk of that root establishes; a bare CID with no admitted root is refused, so a
  stranger holding a leaked CID does not get the bytes.

A tier reads its own store first and forwards a miss to the next tier the requester's list names. The
requester composes and orders that list, its own store always first. Its entries come from the records it
holds (its own devices, its household, the spaces it belongs to) and from the doorways it ingresses
through, which are plural participants and a legitimate source of entries (R6a). The service worker does
this today: it asks a doorway for delivery peers (`GET /api/v1/peers/delivery`, cached 30 s) and scores and
orders them itself (`scorePeerForSw`, `apps-sw.ts`). Adding entries from the requester's own held records
is the open half (§15).

These are design rules. Rule 1 holds today wherever reads go through `fetch_checked`. Rules 2, 3 and 5
are missing nodes in §15 (tree root → reach, tier → tier, sha256 → iroh copy). Rules 4 and 6 have
nothing to bind until a remote tier exists (§15, eprfs → a peer's storage).

1. **Verify at every hop.** Nothing is trusted for where it came from. A forwarded object is stored only
   after its address checks.
2. **Reach before bytes, on both sides.** The requester forwards a miss only to tiers inside the reach of
   the root it is reading, so a private tree's miss does not leave the tiers its reach admits: a tier that
   refuses has already learned what was asked, so only the requester can keep the asking from leaking. A
   tier serves an object only when the requester's reach admits it, read from records the tier holds; when
   the reach record is unknown it refuses and does not fetch it. Tree nodes carry path names, so a tree is
   as private as its root: reach is declared on the root and its nodes inherit it (pending §19 question
   10; until then reach lives on the content row, and a tree's paths are as public as the tier serving
   them). A blob shared by a public row and a private tree stays servable through the public row; the
   private tree's paths are not listed to a stranger. A sealed overlay's root has its author's reach alone
   until the author widens it, and a tier neither serves nor reports the presence of an object reachable
   only through a root the requester may not read (`has_blob` answers absent). To a requester outside reach,
   missing and private give the same refusal, naming the requester's missing standing and not the object's
   existence; the tier's own counters keep the two apart.
3. **Bounded upstream.** Each tier forwards misses under a fixed concurrency and a byte budget per
   requester. On a person's own device the budget is that person's. On a shared tier it is the space's
   appropriation rule, ratified by its members and read from their charter, not set in the tier holder's
   configuration alone. Over budget is refused with ruling 7's three parts: the rule it hit, the retry time, and what the requester
   already holds. It is not queued silently.
4. **No tier is an authority.**
5. **One address on the wire**: CIDv1 sha2-256. iroh's BLAKE3 hash is a transport detail behind an alias.
   Today the alias is written only on the put path (`handle_put_blob`, when an iroh store is wired).
   Making it hold for every blob is residency's "sha256 to iroh copy" missing node and is a precondition.
6. **Each tier chooses its own transport.** Device to peer rides storage's iroh or libp2p transports; a browser
   rides HTTP through a doorway, or to a hub by key where it can address one. The calls are the same.

What a read pattern reveals: the stream of addresses a requester asks for is itself a participation
signal. Rule 2's requester side bounds it: a commons tier is asked only for objects under commons-reach
roots. Where a household tier exists it absorbs repeats, so an outer tier sees at most one miss per object
per household; no household tier exists yet (§11).

### 12.4 Local-first reading of the bytes layer

One device, nothing reachable: everything at depth 3 opens and runs, everything in a depth-1 view opens,
and everything already read at depth 2 opens. Questions about a root whose trees are here are answered
locally. An overlay seals to a new root with no network. A path not yet read shows as remote or missing,
named, and a read of it always answers promptly with ruling 7's three parts: why (the path and the tiers
tried), when to come back (the retry time a tier gave, or "when a tier is reachable"), and what is local
meanwhile (the rest of the view that is here). It does not hang. Slice 3 builds and tests this. Each thing that arrives adds: another device adds a copy, the household hub adds
capacity and depth 0 for the household, a pool adds reach and compute. None becomes a condition. Each
learns what §11 states for a tier serving bytes. When one goes, everything kept or read on this device
still opens, and what was only ever answered at depth 0 by the hub is named remote, as it was already
shown: "not on this device".

The device signs its own overlays and seals with its own key. Before first contact that key is the
person's provisional identity root (§3), on this device only, and the app says so (§16,
`root-held-by-network`). After first contact the device is one enrolled device among others, and the
identity root is not on it alone (ruling 6). Enrollment does not re-sign or invalidate anything sealed
before it, and no peer's presence is needed to sign anything afterwards.

Key held by a doorway (a hosted person): the browser reads what its own store holds, each object checked
against its address. An overlay is kept in the browser's store and seals to a root by address with no
network. Signing that seal waits for the doorway, and the person is told it has not yet been saved to their
account. No key (a visitor): the browser reads what it holds; nothing written is kept past the visit, and
the visitor is told.

### 12.5 Derived bytes, keyed by what they came from

A derived thing is keyed by **(method CID, input root CID)** and its output is a CID. eprfs-core's
`composition.rs` `DerivationEdge` ("derived FROM source") is the home of the record. Instances that today
each compute their own way: brit gate verdicts, rakia build outputs, compiled wasm, search folds, rendered
pages, blame and path history.

- Same key and a deterministic method give the same output; the second requester may fetch the output.
  With no holder of the output reachable, a deterministic method is recomputed here when the device's
  capacity admits it: fetching another party's output is an optimisation, not a condition.
- The record is evidence, not authority. Trust in another party's output rides the gradient: computed
  here; computed by my household's berth under a grant; agreed by several peers whose independence is
  observed (distinct households and distinct build or model lineages, not a count of keys; rakia Canopy);
  recomputed and compared. A deterministic method can be spot-checked by recompute wherever its inputs are reachable.
- Derived bytes are not commons custody (lvi's invariant). They live where computed, under the lease that paid for
  them. What travels is the record, and only at its input root's reach: a record keyed by a below-public
  root does not leave that root's holders, because the record alone tells an outsider that the root exists,
  that someone built it, and when.
- History-shaped derivations are computed from parents' records, so a new root costs its change.

Compiled wasm is named because it is the largest: a deterministic method over a wasm CID and a compiler
version. Keyed this way, a household compiles once. Whether a conductor may run code it did not compile is
the runtime's trust question (§13), not decided here.

### 12.6 Move the work to the bytes

A heavy task is (method, input root, view, envelope). The device names the task and does not ship the
inputs. A berth that already holds the closure, or is nearest to it in the tier list, runs it inside an ark
envelope, materializing the view at depth 1 or 2 from its own tier. Authorization is a `delegates-compute`
grant; placement is a berth offer matched to the commitment. The result is an output CID plus the
derivation record; the device fetches only the outputs it will read.

The device's own envelope is the first candidate for every task it names, and running there needs no grant.
A remote berth is chosen when the floor finds the device ineligible by capacity, or when the berth is nearer
the bytes. With no berth reachable and the device ineligible, the task (method, input root, view) is kept,
named as not carried, and runs when a berth is reachable or when the person chooses to run it here.

Consumers: rakia's executor (Canopy), lvi's devspace (a depth-2 materialization plus an envelope, reached
from a browser), brit integrate's gates in a restored draft, and `just measure`, the first instance. It has
run stages on a household peer (jessica, a same-host stand-in, 2026-09-25 and 2026-09-27) but not yet on the
peer that holds the stores: `--on adam` refuses today, and its habit `measure-runs-on-a-peer` is red, with no
offload proven.

### 12.7 brit, the first consumer

- Any commit's tree can be sealed into an eprfs root on demand with `brit snapshot seal --revision <rev>
  --store <dir>`. `brit-bridge` either seals it losslessly or refuses: a seal that would not round-trip to
  the same git tree id is refused, gitlinks stay as opaque `External` entries, and an unsupported mode or a
  tree past `TreeLimits` is refused. Commits are not sealed automatically today, and a seal goes into a
  local store without being published. A checkout is a materialization of that root at a view.
- **Both modes, one root, not a choice between systems.** A *full clone* is depth 3 of the commit's tree root with
  its history kept: git's own offline, decentralized mode, and a mode any developer whose device has room can choose. A
  *partial clone* for a person follows the head at depth 1 or 2 over their view: trees and blobs as the view
  needs them, no packfile, and what was read stays readable offline. Depth 0 is for consumers that need no
  working copy (gates, agents on a tier), and leaves nothing on the device.
  They are not two systems. Both use the same root and the same walk at different depths, and a working copy moves
  between them without re-cloning: deepen a sparse checkout to kept, or let a kept one fall back to on read.
  Stock git interop, through the bridge and git transports, stays available in both.
  Which depth a checkout opens at is not a setting the developer must know. When the person has not
  chosen, it is the floor's first guess from the device and the space (§7.2): sparse from the collective
  on a Chromebook, full on a workstation.
- History, blame, ancestry and gate evidence are derivations: computed locally where the history is kept,
  and answered by the tier that holds it where it is not. The answer is the same either way, because it
  is keyed by the same input.
- An agent session is an overlay over a depth-2 view, sealed to a draft root or dropped. It costs its read
  set and spine, not the repository.
- Integrate stays one local compare-and-swap on the holder. Offline, a draft seals and is kept; integrate
  is the one step that waits, and only when the holder is another party. A person who is the holder
  integrates alone.

Probe: in this monorepo, a fresh agent session that reads N files and changes M holds afterwards the N
files' bytes, the tree nodes on their spine, and the M overlay files.

### 12.8 What makes this a coding commons on the scale of Scion

The bytes layer is the foundation of a peer-to-peer coding commons; it is not the whole of one. Scion's
result came from four things together, and this design names where each stands so none is implied.

| Scion's piece | The protocol's piece | State |
|---|---|---|
| Root-to-tree-to-blob fetch, sparse and virtual checkouts | §12.2–12.3, slices 1–3 | Slice 1 local; slices 2–3 open |
| Node and regional caches at above 99% hit rate | The household and community hub tiers | `elohim-hub` has no code: the largest missing dependency |
| History, blame and ancestry as an API | Derivations keyed by input (§12.5), slice 4 | brit gate verdicts are the first live instance |
| Server-side commit, merge and rebase as an API | Not designed. Integrate stays one local compare-and-swap on the holder, which suits local-first; merge contention when agents merge at volume has no design yet | Frontier |
| A code-review tool built alongside, and post-merge testing instead of a global merge lock | brit-verify, rakia, the Jenkins bridge as observation | Partly built elsewhere; not touched here |

The gain is measured against a baseline taken before slice 1 (plan item B0), and the bytes layer's
acceptance probe is this monorepo from the start (plan item E5), not a closing step of slice 7. The
Scion-scale result is expected after slices 2, 6 and 7 and a hub tier exist, not from slice 1.

## 13. Install is universal

Installing anything is holding a tree root's closure at depth 3. That covers app bundles, DNA bundles, compiled
code, the conductor, elohim-node, ark, epr, brit, toolchains, `node_modules` and model weights alike.

- **How many copies.** Two apps naming the same runtime root share one copy by address. Today bytes dedupe
  and runtimes do not.
- **Which variant.** Native binaries vary by architecture and cargo features, chosen from the device's
  declared capability. Not per-device wasm or DNA.
- **How it is run.** The envelope spawns only from a verified-release value, re-hashed at the moment of
  spawn, and records the CID it actually ran (tevah §4). The old self-updater in `steward/node/src/update/`
  has a stub signature check and is wired; until the exec floor replaces it, "every object is verified"
  does not hold for binaries, and this design says so.
- **What is derived.** Compiled wasm is a derivation (§12.5).
- **Where trust is retained** — the operator's question, and the redesign this design hands the runtime
  last. Two questions that must not be fused. *Where the identity root lives* is answered: with the network, under
  key lineage and a recovery quorum, not on one device alone (ruling 6). *Which acts need the person's present
  consent* through a device they hold, and which may run on a berth on their behalf, is the open one.
  Today a thin client opening an app on a hub hands the hub the key for that cell; a device-held key
  driving a remote cell is a missing node. The `signing` need in §6 is where the second answer is
  declared.

## 14. Design gate

Run against the p2p design gate. No entity here needs a new DHT entry type. New REA and Mishpat action kinds
(the `runtime-offer` kind, a fulfilment action) ride action strings that integrity does not validate: they
are kept out of the `content_store_integrity` `REA_ACTIONS` constant and validated in the coordinator and in
storage, so `dna-hashes.baseline` stays unchanged, and that is checked when slice 5 lands. Integrity's
acceptance of any unknown action string (mishpat integrity's `commitment_action_requirements` ends in
`_ => None`, so an unlisted action carries no requirement, and `REA_ACTIONS` is not enforced at validation) is a recorded C10 gap. Closing it moves the DNA hash and is the
operator's call.

| Entity | Class | Address | Source of truth | Notes |
|---|---|---|---|---|
| Tree node, blob | Content-addressed bytes: not a DHT entity and not a head; notarized only through the head or release that names the root | CIDv1 dag-cbor (trees), raw (blobs) | The bytes plane; the head naming a root is notarized where it already is | Transport `auto`: each tier picks its own (§12.3 rule 6). iroh does not carry eprfs objects until the sha256-to-BLAKE3 alias holds for every blob (slice 2 precondition) |
| Walk and fetch request | Ephemeral (C) | None | None | Concern canon: honest absence (`BlobPresence` gains `Refused` and `Unverifiable`; today it is {Local, Remote, Missing, Unknown}, a partial answer; to a requester outside reach, missing and private give the same refusal, naming the requester's missing standing, and the tier's own counters keep them apart), bounded work (budget), advertise/serve symmetry (a tier answering `Local` serves), backpressure (bounded upstream), consent (reach before bytes, on both sides, `fetch_blobs(root, …)`), observability (every refusal named). Registered in a new `elohim/eprfs/seam-registry.yaml` when built |
| View | Private (B), device or session state of the reader, disposable but not reconstructable, when discovered by reads; a field of the release manifest (Notarized (A), existing, content-addressed) when declared as `closure` | Path prefixes under a root CID | The reader's device or session; or the release head | No new type. A discovered view is sent only to tiers inside the root's reach and is not recorded beyond the session: it is a participation signal |
| Overlay | No class until sealed and published (working-version standard §3) | Its sealed root is a CID | The author's device, or for an agent session the sponsor's berth under its lease | A solo overlay is not sent to a doorway (§2, §12.2) |
| Derivation record | Ephemeral (C) by default, re-derivable by recompute | (method CID, input root CID) | Where computed | When others rely on it, a Notarized (A) attestation through the existing `publish_attestation` path, bundled as one corpus digest per adopted head per method; drafts' derivations stay C and do not attest. Count: O(adopted heads a year × methods), a few thousand a year for this monorepo, so it needs that bundling or operator sign-off. A record travels only at its input root's reach |
| Kept set | Device-local operational state, private to the device, not on the person's source chain (each device keeps its own) | The device pin | `acquisition_pins` | Unchanged |
| Berth offer, with posture as its fields (public or commons reach) | Notarized (A) on the elohim DNA's existing `Intent` (provider set, receiver none); `Intent` or `ServiceOffer` is picked once as the one home; DNA-hash neutral | Agent-scoped composite (the blade's enrolled device key, space, resource kind) | DHT | Written on change by update, not re-advertised on a timer; whether the offerer is alive comes from the one liveness source, not the offer's age; O(devices × spaces). The SDK schema under `schemas/v1/intents/` is the missing part |
| Berth offer and posture (inside a household) | Household-local signed record, not notarized (§7.1, §7.2) | Agent-scoped composite, as above | The household's own floor | Below public reach until per-space cells exist; written on change with hysteresis |
| Posture preferences and observed device facts ("spare my battery", a budget, battery level, a metered link) | Private (B), device state | — | The device | Only their effect crosses, as a narrowed or withdrawn offer, under hysteresis. Others see "not offered" |
| Placement commitment | Notarized (A) on the elohim DNA's existing REA `Commitment` (an action such as `custody-blob`), only for public or commons needs until per-space cells exist; a household-local signed record otherwise | Agent-scoped composite (provider, receiver, tree root and view), with a deterministic id as salvage's `deterministic_custody_id` has | DHT, or the space's own cell once one exists | Carries the parameter-set CID, the method CID and an eligible-set digest (§7.4). See head-plane cost below |
| Placement parameter set (bytes) | Content-addressed bytes, not a DHT entity | CIDv1 dag-cbor, naming the ranking-method CID it parameterizes | The bytes plane, held by the space's members | Transport `auto`. The floor's built-in set is one such CID, shipped with the floor |
| Parameter-set head | Linked (A2): a head-declaration link on the space's existing anchor (the `head_delegation` family), tag = (parameter-set CID, ratification hash, clock) | Agent-scoped composite (declarer, space, `placement-parameters`) | DHT, or the space's own cell once one exists | Coordinator only, DNA-hash neutral; O(tens a year per space); no bundling needed |
| Ratification | Attested-Private (B2): a private ballot and a notarized tally, on the governance records the elohim DNA already carries (mishpat's integrity zome notes that proposals and votes moved to the elohim DNA in its Stage C; no `ProposalVote` entry type is defined there by that name, so which entry carries them is read before the ceiling slice); for a household, each touched member's consent | Agent-scoped composite | The ballot's holders; the notarized tally | Floor-protected: constitutional. Rests on the absent charter-update path (§15) |
| New grant scopes | Existing `delegates-compute` | Existing | DHT (Mishpat) | Closed-list entries in elohim-storage's `GRANT_SCOPES` (`api/compute_grants.rs`) plus their storage-side consumers (for example `api/hosted_custody.rs`, `compute_tasks::required_scope`). Mishpat integrity validates the bounds keys only, not scope membership; the acting storage node verifies the scope (C12). Moving scope validation into integrity would move the DNA hash and is not done. A grant is consent and eligibility, not placement (§8) |
| §6 manifest needs | Fields of the existing app manifest | — | The manifest | No parallel manifest |

**Head-plane cost.** Bytes do not become heads: tree nodes and blobs are the bytes plane, and only the root a
head names is a head. The floor follows the same rule. Custody, carriage and fulfilment are committed per
tree root and view, not per blob, so a kept 10,000-file closure costs one commitment per holder rather than
about 20,000. Leases are long-lived and lapse on an observed carrier loss or a `revokes-commitment`, not by
periodic re-authoring. Fulfilment is one settlement event per commitment per declared period (quantities
only), not per object served. Counts: offers O(devices × spaces), tens per household; commitments O(roots
kept × holders), hundreds per household at one year; fulfilment O(commitments × periods). Anything above
about 500 per space rides a composite root or needs operator sign-off. Salvage's per-blob `custody-blob` is
the debt to migrate, not the pattern (§15).

**Concern canon for the floor.** The rank function, the eligibility verdict, the adopt decision and the
parameter-set reader are decision predicates. They are registered at birth in
`elohim/elohim-storage/seam-registry.yaml` beside salvage, with `contractTests: null` and a `gapNote` until
built:

- C0: custody and availability for commitments, reach and audience for eligibility, head and version for the
  parameter set. Three planes, not fused.
- C1: the guarded self-candidacy arm (§7.2).
- C2: the parameter-set head is declared, not "latest". Its clock is the ratification's notarized timestamp,
  and a revert is a new declaration naming the old CID, not a head moved backward.
- C3: with no ratified set reachable, the floor uses the last set it verified, or the built-in set (a
  declared CID); a lapsed lease with no eligible candidate yields "not carried", not a stall.
- C4: outcomes are distinct: adopted, not ranked, ineligible{reach | capacity | consent | posture | unread
  input}, not carried, inputs stale, parameter set unverifiable.
- C5: a parameter set, an offer and an inventory claim confer nothing until re-derived locally.
- C6a: a per-event re-evaluation budget, and moves capped per event. C6b: deterministic commitment ids.
- C7: an offer advertised is served, checked by the probe.
- C8: one labelled counter per outcome, generalizing `SalvageOutcome`.
- C9: commitments name the enrolled device key and follow a `KeyRotation` (as the test
  `reclaims_self_held_custody_under_a_new_agent_key` does for salvage).
- C10: a parameter set naming an unknown parameter, or a ranking-method CID this peer does not run, is
  refused and observed, not defaulted.
- C11: hub tiers refuse with a retry time.
- C12: consent grants precede eligibility.
- C13: `SALVAGE_TARGET_REPLICAS` is labelled at the gate as the scaffold for `loss`, with its successor
  named; so is a member declaring a space's head until a collective can be a party.
- C14: the residual arm (an input set fitting no outcome) is witnessed with a capsule (inputs digest,
  parameter-set CID, method CID).

**Network stakes.** Every entity behaves under all four stages. Stage-priceable: re-verification of
already-witnessed placement commitments and digest-matched parameter-set heads, and walk verification of
nodes already verified on this device. Floor-protected at every stage, Simulacra included: eligibility
(reach, capacity, consent), ratification and charter records (constitutional), and a member's contest of a
tuning or of a "not carried" answer (counter-evidence, which reaches the drafting elohim or council and the
ratifiers).

**SDO/RWA test.** The worst holder of a commons tier sees requests for commons-reach objects and at most one
miss per household per object; it cannot join across holons, because each tier list is the requester's
own; it cannot compel a tier to serve outside reach. The worst carrier of each kind is bounded as §11
states. Boundary 6 bounds participation leakage throughout; the push service is the one carrier this
design cannot fully bound, which is why §19 asks the operator.

The floor and the ceiling, read the same way. The worst holder of a ratification could steer custody to its
own carriers. That is bounded by eligibility the parameter set cannot touch: no one party (by admission
record, not by self-declared household) holds more than one of a need's holders. Household diversity rests
today on `humans.household_id`, which is self-asserted and not admission-validated, so until admission
records exist the household-diverse bound is unproven and the design says so (§10 item 19). Offers change
only under hysteresis and do not publish raw device facts (boundary 6). A tuning carries the pressure, not a
member's words: a member's concern reaches a draft only through that member's own crossing act, and remains
erasable from counsel (boundary 3). The elohim reads "cost borne unevenly" only from the per-household
aggregates each household publishes at the space's reach, not by joining fulfilment records across holons
(boundary 2). Below-public commitments stay household-local (§7.2), so the shared DHT does not carry a
ledger of who holds what for whom inside a household.

**Identity namespaces.** Candidates and commitments name the enrolled device's agent key (`uhCAk…`),
resolved to the person through the roster. Sponsored agents are candidates only for their sponsor's own
needs, on grants, not as devices. Tiers and carriers are resolved through the existing peer binding;
transport ids are not string-compared with agent ids (the p2p design gate's identity rule). Bindings are
self-asserted and unsigned today, so the carrier's fulfilment record and any reading of cost borne unevenly
are blocked for attribution (standing, settlement) until a cross-signed control proof lands; until then they
are evidence only.

## 15. Missing nodes

In the chain's own shape: between two named atoms, the missing assertion, its probe, and the state.
Grouped by the layer that must build them, in the operator's order.

**Bytes layer (eprfs)**

- Root → view: *walking a root under a view visits only nodes on the view's paths.* Probe: a 10,000-file
  root under a 3-path view fetches the spine of 3 paths. Absent: `load_verified_tree` walks the whole
  closure.
- eprfs → a peer's storage: *an `EprfsStorage` reads from a peer by key.* Probe: `fetch_blob` with
  `FetchIfMissing` against a household peer. Absent: `StorageEndpoint { base_url }` is named and
  unimplemented, and `DirectoryStorage::fetch_blob` ignores its policy.
- Tier → tier: *a miss forwards upstream under a budget.* Probe: a cold household's reads do not exceed the
  budget at the hub. Absent.
- Tree root → reach: *a tier refuses a node whose root's reach does not admit the requester.* Probe: a
  private tree of public blobs lists nothing to a stranger. Absent: reach lives on content rows only.
- Sparse marker → bytes: *a marker becomes bytes on first read.* Absent: markers are written at checkout
  and not hydrated.
- sha256 → iroh copy (residency): *the alias between the two addresses exists for every blob.* Probe:
  after a blob is let go, neither store holds it. Partial: the alias column
  `peer_blob_inventory.blake3_hash` and its lookups (`lookup_blake3_for_sha256`,
  `lookup_sha256_for_blake3`) exist, but the column is written only on the HTTP put path
  (`handle_put_blob`, when iroh is enabled and its write succeeds). A blob that arrived by fetch or push
  has no alias, letting a blob go leaves its iroh copy behind, and the `backfill-blake3` binary the code
  points to does not exist.
- Pin → closure: *the residency pin names a closure, and eprfs reads it as depth 3.* Absent: the pin names
  a head ref, and no eprfs code reads a pin.
- Requester → tier list: *the requester's tier list also carries entries from records it already holds
  (its devices, household and spaces), beside those its doorways supply.* Partial: the service worker
  orders the candidates a doorway supplies; no entries come from held records yet.
- Peer bytes → service worker cache: *per-file digests let peer-served bytes be cached.* Absent
  (`apps-sw.ts`).
- (method, input root) → output: *a derivation is found by its key.* Probe: same root and method give the
  same output CID on two peers. Absent; brit finds gate verdicts by inputs hash in git notes, and the search
  fold is keyed by method only.

**Commons compute (ark, berth)**

- Blade boot → commitments rebalanced: *a berth offer, matched to an under-held commitment.* Probe: the
  offer appears in the hub ledger within N seconds. Specced (tevah), absent.
- Expressed intent → placement: *every peer's floor ranks the same eligible candidates in the same order for
  the same need, and the adopted set converges to the ranked set with no coordinator, duplicates
  releasing.* Probe: two peers given the same visible intents name the same candidates; a placement
  converges with no coordinator, and the probe reports how long convergence took and how much excess it
  caused. Partial: salvage custody does this for one need kind, with self injected into its own pool.
- Own offer → self-candidacy: *a peer is a candidate for itself only through its own published offer, read
  from the same source every other peer reads.* Probe: a peer whose offer has not yet round-tripped does not
  adopt. Absent: `build_salvage_candidates` inserts self before its ad round-trips.
- Salvage ticker → event-driven floor: *the floor re-evaluates on a posture, membership, miss or carrier-lost
  event, and a peer at rest runs no pass.* Probe: a household at rest with every posture declared shows zero
  salvage passes over an hour. Absent: salvage runs on `salvage_recheck_seconds`.
- Per-blob custody → per-root commitment: *one commitment per holder per tree root and view.* Probe: keeping
  a 10,000-file closure on two holders writes two commitments. Absent: salvage writes one `custody-blob` per
  blob per holder.
- Below-public placement → household-local record: *a placement for an intimate- or trusted-reach need is a
  household-local signed record, and no shared-DHT commitment names its blob, session or relay pairing.*
  Probe: after placing a household-reach blob, the shared DHT holds no commitment naming it. Absent: salvage
  notarizes every custody on the shared DNA.
- Unmet need → "not carried": *each unserved party's own floor derives "not carried", with its reason, from
  the commitments it can see, and nobody sends it.* Probe: with offers short of needs, the unserved party is
  shown "not carried" and its reason, and no carrier receives a message about it. Absent.
- Lapsed lease → re-placed first: *a carrier lets go only after another holder honours the same need or the
  recipient withdraws; an unreachable recipient does not end the carrying.* Probe: take a recipient offline
  past its lease, and its hub still holds its bytes when it returns. Absent.
- Claimed → held: *a holding probe proves a holder still has the bytes it claims.* Absent: the honoured
  count is fresh inventory claims (`peer_blob_inventory`).
- Self-asserted binding → cross-signed binding: *a carrier is credited only over a binding both keys
  signed.* Absent: bindings are self-asserted and unsigned.
- Posture → readable record: *a posture and a berth offer have an SDK schema and a home (§14).* Absent: no
  schema under `schemas/v1/intents/`.
- Elohim draft → ratified parameter set: *a parameter set reaches the floor only with the ratification
  record of the participants it touches, carried by each member's own vote.* Probe: an elohim-drafted
  parameter set with no ratification record is read by no peer's floor, which refuses it naming the missing
  ratification; after one-human-one-vote closure, all peers converge on the new holder set. Absent; it rests
  on the next node.
- Charter → amended charter: *a collective's charter is updated by its own amendment rule, so a ratified
  order of service and a shared tier's budget have a record to be read from.* Absent: `Collective` has no
  charter-update path (commons-holonic-stewardship backlog row 35).
- Ratified tuning → placement: *a ratified parameter set is read by every member's floor, and placements
  move make-before-break within the move budget.* Probe: after a tuning is declared, all peers converge on
  the same new holder set through make-before-break moves within the move budget; declaring the previous
  parameter set again converges back the same way. Absent.
- Declared need → grant: *a manifest need resolves to a grant scope.* Absent; the list is closed at three.
- Grant → fulfilment: *the carrier records what it carried, notarized, for every kind of carrying.*
  Partial: for delegated compute the runner's signed completion event is on the DHT; the requester's
  `compute-fulfilled` is local only; hosted bytes, relay, doorway and hub carrying have no carrier record.
- Archetype facts → envelope: *the ark-core derivation of capacity.* Specced, absent.
- `battery` / `alwaysOn` → default: *a device on battery, on a metered link or not always on is not asked to
  carry for another household or space unless its person lifted that default, and is told why; holding
  within its own household is deferred to a charging or unmetered window, not refused.* Absent: no placement
  code reads `battery` or `alwaysOn`.
- Revoked grant → berth lease released. Absent.

**brit, rakia, lvi**

- Commit → checkout: *clone is follow plus walk.* Absent.
- Agent session → draft root: *an overlay over a depth-2 view seals or drops.* Partial (`brit snapshot seal`
  seals a committed git tree to a root CID and `brit snapshot restore` restores that whole tree to a new
  directory; no overlay, no seal-or-drop of session writes).
- Devspace seed → depth-2 mount. Absent.

**The contract (§6)**

- Release → closure: *every blob needed to start is declared.* Absent (residency "pin to bytes").
- Two apps → one runtime copy: *apps naming one runtime resolve to one closure CID.* Probe: a second app adds
  no runtime bytes. Absent.
- Manifest → channels followed. Absent. Head → space: *a channel scoped to a space.* Absent.
- Manifest → `passing`, `notify`, `loss`, `signing`, `compute`, `projectable`, `outward`, `indexed`,
  `witness`. Absent, each.

**Planes that build beside the layers**

- Circle admission: *a signed record admits a person to a circle, and validators check it.* Absent; it gates
  the live channel, `republish: circle-agreement`, document custody, and any claim of privacy.
- Chat: *a thread record with a reach and a sync route; a message written offline converges.* Undesigned.
- Wake: *a sleeping device is woken with no content, and pulls the signed act over its own key.* Undesigned.
- Relay → commitment: *who carries a relay for a space is recorded, and a relay-dependent device can report
  its home relay.* Absent.
- Browser → hub by key: *app bytes with no doorway in the path.* Absent; feasibility unverified.
- Device key → remote cell: *a device-held key drives a cell on a hub's conductor, and the hub holds no key
  material.* Absent; no remote-signing design.
- Hub opens an app → the person is told: a disclosure record. Absent.
- First contact → identity root secured: *as soon as any peer is reachable, the person's identity root is held under
  a network recovery quorum, and the device that created it becomes one enrolled device among others.*
  Probe: destroy the only device after first contact, and the person is restored on a new one. Absent; it
  rests on the next node.
- Quorum → new key: *a valid witness-backed `KeyRotation` is minted.* Open, high priority in the backlog.
- Rotated key → the person's app opens with restituted bytes. Absent.
- Agent act → attributed event: the per-concern acts file exists. Absent (0 files at last count).
- Model layers → decision record: each layer has its own identity. A gate rule only.
- Leaving → holders let go; withdrawn membership → commitments revoked; a holder leaving → salvage re-places
  before its commitment lapses. Absent, spec only, and partial respectively.
- Space growth → rung change: a space-shape declaration plus a recorded D6 pass. Absent.
- `canInfer` → inference served under a grant; counsel context → erasure; weights → an eprfs root. Absent.
- Projection → retention floor; doorway reach gate → the SDK reach enum; outbound record → the key its
  holder can read. Absent.

## 16. Stories that fall out

Each story is a scenario the contract makes true. "Mesh" marks the ones that need the household mesh. A
proposed file is named where no existing feature fits; every one is checked against `genesis/a2o/features/`
before it is written, and every one written enters the blind-reader loop that `genesis/a2o/.epr-meta`
requires.

| Layer | Story | Feature | Concern | Mesh |
|---|---|---|---|---|
| Bytes | A reader that names three paths under a large tree holds those paths and their spine, nothing more | proposed `storage/materialize-what-is-read.feature` | `materialize-what-is-read` | no, then yes |
| Bytes | The same read is answered from my device, a household peer or the hub through one interface, each object verified | same | `materialize-what-is-read` | yes |
| Bytes | A private tree of public blobs lists nothing to a stranger | proposed `dataplane/reach-on-tree-root.feature` | `reach-enforced-everywhere` | yes |
| Bytes | With the network off, every kept app opens and an unread path is named, not hung | proposed `lms/kept-offline-app-opens.feature` | `kept-offline` | no |
| Bytes | Two peers with the same root and method share one derived output | proposed `storage/derived-by-identity.feature` | `materialize-what-is-read` | yes |
| Bytes | A rebuilt index equals the one before deletion | proposed `search/index-rederivable.feature` | `recall-reaches-authority` | no |
| Compute | A new blade offers a berth and the hub re-places an under-held commitment | proposed `compute/berth-offer.feature` | `berth-offer-matched` | yes |
| Compute | A holder's fulfilment of a lent stage is on the notarized record | `compute/measure-on-a-peer.feature` | `measure-runs-on-a-peer` | yes |
| Compute | With no elohim present, placement still converges, and every peer converges on the same holders | proposed `compute/placement-floor-converges.feature` | `placement-floor` | yes |
| Compute | A tuning the household ratifies changes the ranking's inputs, every peer converges on the new holders through make-before-break moves, and declaring the previous set again converges back | proposed `compute/placement-tuning-ratified.feature` | `placement-ceiling` | yes |
| Compute | An elohim's draft tuning, unratified, changes no placement on any peer | same | `placement-ceiling` | yes |
| Compute | A phone on battery is not asked to carry for another household unless its person offers, is told why, and still holds its own household's bytes when it charges | proposed `dataplane/device-role-refusal.feature` | `device-capacity-declared` | no |
| Compute | A heavy build runs on the berth that holds its inputs and returns only outputs | `compute/peer-executed-stage.feature` | `operator-runtime-surface` | yes |
| brit | The same repository opens sparse on a Chromebook and as a full clone on a workstation with no setting changed, and either deepens or falls back without re-cloning | proposed brit journey | `brit-native-daily-driver` | no |
| Compute | An always-on household machine is told it is eligible to host live sessions, and hosts only after its person offers to | proposed `compute/eligible-is-not-enlisted.feature` | `placement-floor` | yes |
| brit | A fresh agent session reads N files, changes M, and holds only those | proposed brit journey | `brit-native-daily-driver` | no |
| Contract | One app, declared once, opens for one person offline and for a household with no change | proposed `federation/app-inherits-placement.feature` | `app-inherits-placement` | yes |
| Contract | A second app naming the same runtime adds no runtime bytes | proposed `federation/runtime-closure-shared.feature` | `code-residency-shared-closure` | yes |
| Contract | A household that gains a hub keeps its apps and sheds load to it | `resilience/household-diversity-dataplane.feature` | `blob-durability` | yes |
| Contract | An app's declared loss model sets how many households hold its blobs | existing `storage/declared-storage-policy.feature` (`@wip @wip-tier-controller`, no `@concern` tag; today it covers tier warmth and pledge floors only), extended with a loss-model scenario and tagged `@concern:blob-durability` | `blob-durability` | yes |
| Contract | A group's channel and the commons channel carry different heads of one app | proposed `delivery/space-scoped-channel.feature` | `space-scoped-channel` | yes |
| Contract | A peer offline through three releases takes the elected head | `delivery/app-bundle-elected-delivery.feature` | `app-bundle-elected-delivery` | yes |
| Live | Two of one person's household machines show each other who is here, directly, with no host | `dataplane/live-session-own-devices.feature` (`@wip`, slice 1 of the live-session plan) | `live-session-direct` | yes |
| Live | Two people see each other in the sheet, and one of two hosts dies and presence continues | proposed scenario in `federation/local-first-shared-sheet.feature` (today an untagged `@wip @vision` Act III story) | new tag `live-session-plane` | yes |
| Live | A message written offline reaches the group on reconnect | proposed `qahal/group-thread.feature` | `chat-is-a-record` | yes |
| Attention | A quiet phone is woken by a content-free tap and pulls the act over its own key | proposed `qahal/wake-a-sleeping-device.feature` | `wake-reveals-nothing` | yes |
| Attention | A household at rest, with nobody acting, stays within its call and processor budget | `dataplane/household-at-rest.feature` | `idle-is-free` | yes |
| Connectivity | Two household devices on different networks meet through the relay, and a conductor at zero connections is reported, not green | proposed `dataplane/peer-reach-through-relay.feature` | `reach-by-key` | yes |
| Connectivity | A browser fetches app bytes from a household hub with no doorway | proposed `browser/browser-to-hub-by-key.feature` | `browser-to-hub-by-key` | yes |
| Identity | Opening an app on the hub tells the person the hub holds their key | `auth/hosted-human/` series | `hosted-compute-contracted` | yes |
| Identity | A device signs for a cell the hub runs, and the hub holds no key | proposed `auth/remote-signing/01-device-signs-hub-runs.feature` | `device-key-remote-cell` | yes |
| Identity | A person loses every device and opens their app on a new one: same root, same data | `auth/recovery/identity-key-lineage-recovery.feature` (`@wip`, tagged `@identity-lineage`; no `@concern:` tag yet) | proposed `identity-lineage-recovery` (no such habit yet) | yes |
| Identity | A solo person's identity root is secured by the network at first contact, and until then the app says plainly that it is provisional | proposed `auth/recovery/root-secured-at-first-contact.feature` | `root-held-by-network` | no, then yes |
| Agents | A runner cannot materialize bytes above its grant's reach | `compute/ci-runner-stewarded.feature` | `operator-runtime-surface` | yes |
| Agents | An agent's edit is recorded as the agent, with its sponsor named | proposed `stewardship/agent-act-attributed.feature` | `acts-attributed-to-participants` | no |
| Inference | Revoking an inference grant erases the counsel context its carrier held | proposed `compute/inference-grant-revoked.feature` | `counsel-context-erasable` | yes |
| Edges | An anonymous visitor sees a commons item and nothing narrower | `dataplane/served-under-standing.feature` | `served-under-standing` | yes |
| Edges | Withdrawing a publication clears every doorway's cache of it | proposed `doorway/projection-withdrawal.feature` | `served-under-standing` | yes |
| Edges | A published page stays readable through the loss of one of two doorways | `dataplane/doorway-failover.feature` | `doorway-failover` | yes |
| Edges | Refusing an outbound projection costs the app nothing | proposed `federation/outward-projection-optional.feature` | `bridge-optional` | no |
| Lifecycle | A member leaves and the bytes they held for the household are re-placed first | proposed `resilience/member-leaves-replaced-first.feature` (beside `resilience/household-reciprocity.feature`, whose `@concern:reconcile-inventory` scenarios assert only the named custody pairs) | `blob-durability` | yes |
| Lifecycle | Leaving revokes what I carried and stops what I drew | proposed `qahal/leaving-a-collective.feature` | `leave-is-clean` | yes |
| Proof | What I promised and what I hold agree | `resilience/observable-distribution.feature` | `blob-durability` | yes |
| Proof | A stranger's read of a member-only space is refused on every path | proposed `qahal/hostile-joiner.feature` | `peer-admission` | yes |

## 17. Slices, in order

The operator's order (2026-10-20): eprfs foundations first; then commons compute, with ark and berth
enabling brit, rakia and lvi; brit as the first consumer; the runtime and the install gradient last. Each
slice is a lane with its own gate and its own proof.

1. **View-scoped walk and query** (`eprfs-core`, local only). `walk(root, view, budget)` beside the existing
   `load_verified_tree` in `eprfs-core/src/tree.rs`, plus three functions over a verified walk: read one
   path, list one directory, diff two tree roots under a view. They run against `DirectoryStorage`
   (`eprfs-storage`) with `FetchPolicy::LocalOnly`. No network. Gate: `just gate eprfs` (the gate project
   exists in `elohim/eprfs/build-manifest.json`). Proof, in two steps: (a) under that gate, Rust tests in
   `eprfs-core` asserting that a 10,000-entry tree under a three-path view visits only the three paths'
   spine, and that a walk under `LocalOnly` over a tree with one subtree removed yields the held paths and
   one named absence and returns without error (§12.3); (b) outside that gate, the same walk over a tree root of this monorepo sealed by
   `brit snapshot seal --store <dir>` (brit-bridge's `SealedGitTree`). Step (b) runs as a recorded receipt
   in which an eprfs reader walks the store brit wrote, or as a brit-bridge test once an eprfs-core release
   carrying `walk` is published to the `elohim` registry and brit's dependency (`eprfs-core = 0.1.2`,
   registry, no path patch) is moved to it; that test is verified by brit's attested CI, not by the eprfs
   gate. The walk's three inputs (root, view, budget) are fixed here, and so is one property of its
   return: a node the store does not hold is a named absence, not an error. The signatures of read-path,
   list-directory and diff, and the walk's return type, are settled while building, against the proofs
   they must satisfy. The slice's deliverable is the Rust tests and the receipt; the §16 bytes-layer
   feature files are written beside it, born `@wip`, and their mesh scenarios run from slice 2. Moves the
   new habit `materialize-what-is-read` from unwired to red with a runnable check.
2. **Remote tier and forwarding** (`eprfs-storage` over `elohim-storage` read calls). A tier list, bounded
   forwarding, reach on the root. Preconditions, each its own fix: the sha256-to-iroh alias, and the two
   reach defects in §18.
3. **On-read depth.** Hydrate on read from the CLI first; a virtual filesystem only where the host profile
   offers one; per-file digests so the service worker becomes the browser's on-read tier.
4. **Derivation records** (`eprfs-core` composition). Lookup by (method, input root). brit gate verdicts
   are the first kind; the search fold, re-keyed, the second.
5. **The floor controller** (§7.2). Intents from every participant, berth offers and postures as REA
   intents, the deterministic ranking under the salvage test contract, self-adoption, and event-driven
   reconciliation with hysteresis; the grant scopes of §8; the carrier's notarized fulfilment record;
   refusals by eligibility. Salvage custody is generalized, not replaced, and generalizing it closes its
   known gaps: self-candidacy only through its own published offer, per-root rather than per-blob
   commitments, household-local records below public reach, an event-driven trigger in place of the
   ticker, and the method and parameter-set CIDs on every commitment, starting from the floor's built-in
   parameter set. Its decision predicates are registered in the seam registry at birth (§14, concern canon),
   and `dna-hashes.baseline` is checked unchanged when it lands. The elohim ceiling (§7.3) follows
   slice 5 as an unnumbered slice of its own, because its scope depends on the floor's outcomes being
   readable: it needs the declared parameter set and a ratification path, which rests on the charter-update
   path (§15), and nothing in the floor waits for it.
6. **Task to berth.** (method, root, view, envelope) placed on the berth nearest the bytes. `just measure`
   re-expressed as the first task kind; rakia's executor second.
7. **brit, natively.** Clone as follow plus walk; agent sessions as overlays; the monorepo probe.
8. **lvi devspace at depth 2.**
9. **The manifest needs**, each landing with its first reader: `closure`, `runtimes` and `works-offline`
   with the residency pin-to-bytes node; then `channels`; then `loss`; then `signing`, `compute` and
   `projectable`. `passing` and `notify` land with the live-session and wake designs.
10. **The runtime redesign**, its own design, starting from §13: shared runtime closures, compiled wasm as a
    derivation, a device-held key driving a remote cell, and where trust is retained.

Planes that do not wait for the layers, because they have or need their own designs and only inherit this
contract: the live session (spec exists), chat and wake (designs needed), relay posture, the `KeyRotation`
mint path (backlog), circle admission, and agent attribution.

The habit this design serves is declared at `elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md`,
born unwired. A second habit for the contract as a whole (`app-inherits-placement`, whose check is the
first story of the contract layer in §16) is proposed and not declared; it is the operator's to mint when
slice 9 begins.

## 18. Defects found in passing

Found by the plane readers while reading. None is this design's to fix; each is named for the habit or
owner that holds it, and two of them are preconditions of slice 2.

| Defect | Where | Owner |
|---|---|---|
| The household device ceremony approves a device's own joining, which mishpat refuses since 0b73bcd23 (`device_enrollment.rs`), so the `federation-version-convergence` lane cannot run on a fresh household or on one whose binding was lost (0/2, both scenarios stop with `household device binding not found`). That half of the red is a harness defect: a missing second-approver node. The same DELTA measures a real divergence too: `hub-carries-edit` 0/1, where a restarted jessica did not reach the new head within 90 s | `genesis/a2o/scripts/household-device-ceremony.ts` and the `hub-carries-edit` lane (DELTA 2026-10-18a) | `dataplane-convergence` |
| The doorway's older reach function matches locality names, so it denies `self`, `intimate`, `trusted`, `familiar`, `community` and `public` by default. Nothing outside tests calls it: its only caller, `should_serve_response`, is test-only, and the live pantry gate is `should_stock` (`routes/freshness.rs`), which stocks only content whose reach is absent, `public` or `commons`. Reconcile it to the SDK vocabulary or delete it before slice 2 relies on it (precondition of slice 2) | `doorway-service/src/cache/access_control.rs:44` | `reach-enforced-everywhere` |
| The blob reach route serves when its database lookup fails (precondition of slice 2) | `elohim-storage/src/http.rs:4650, 4695` | `reach-enforced-everywhere` |
| A failed storage probe is reported as 100% free, so absence reads as health | `elohim-storage/src/heartbeat.rs:384–391` (the `Ok(Err(_))` and join-error arms of `DefaultProbe::sample`, each returning `100`) | `liveness-has-one-source` |
| The wired self-updater's signature check is a stub | `steward/node/src/update/download.rs:137` | tevah exec floor |
| The `PeerCapacityView` route is served by peer id with no owner check | `PeerCapacityView` route (residency §4) | `release-retention` slice 1 |
| Sync serves changes with no held-for-author exclusion, and paginates nothing | `elohim-storage/src/p2p/mod.rs` about line 8130 | working-version standard |
| The roadmap note "PWA deferred" is stale: the service worker exists and is registered (only the web-app manifest is absent) | `founding-milestone-sprint-roadmap.md` about line 591 | roadmap |

## 19. Decisions for the operator

Only questions that are the operator's to decide, deduplicated across the eight readers.

1. Does `closure` live in the app manifest or in the release manifest (rakia, brit)?
2. Who owns the loss model: the app, the collective's charter, or the space's steward? Is its unit
   households or devices?
3. Is hub-held custody acceptable as the floor for a thin client, with remote signing as an upgrade, or
   must remote signing come before any "the hub runs my cell" claim?
4. Is `signing` (device-only against delegable acts) declared by the app, or a policy the space supplies?
5. May a person's own local agent sign under the person's device key within a bounded grant, or does every
   agent have its own key as the runner ruling implies?
6. Is Web Push through a doorway acceptable given that the push service learns who is woken and when, or
   must waking stay peer-to-peer?
7. Does the commons run public relays as a facility, or does each household and collective? A relay learns
   who dials whom.
8. *Answered by §7.2:* a need crosses households at the reach it was expressed at, and no further. Left
   open: whether a space may cap that below the expressed reach.
9. Should content reach (the SDK enum) be the only audience vocabulary, with the locality ladder kept for
   placement only, as the code comment already says? The drift ruling stands until reversed.
10. Is reach on an eprfs tree root in scope for the foundation, or does reach stay on the content row?
11. May a space hold a head back from the commons channel? If yes it is a declared fork; if no it is
    staleness.
12. Is chat a first-class SDK object, or a content type under a witness context?
13. Under ruling 6 (settled: recovery is a promise to everyone, and a solo person's identity root is
    provisional until first contact), what counts as "secured": which quorum, held by whom, reached at the
    first reachable peer, and how quickly?
14. Is the web-app manifest (an installable PWA) wanted, given that the Tauri shell was chosen and a
    household browser needs a doorway-class name?
15. Which agency-phase vocabulary does the contract key on? Three are in use (`agencyPhase`,
    `agency.model.ts`, the steward stages), and the device rungs are a fourth axis.
16. May a berth's conductor count toward a collective's holding floor, or must holding domains be named
    households?
17. Who holds a person's remote identity on a bridged network: the person through a stewardship grant, or
    the doorway operator?

## 20. Not decided here

- The schema text of each §6 need. Each lands with the slice that first reads it.
- Prefetch policy for depth 2, and whether depth 2 is offered at all on devices with no virtual
  filesystem.
- Whether brit records first-parent deltas for history derivations or computes from snapshots.
- The reason name for bytes "read and kept warm" at depth 2 in the residency holds view.
- The numbers: retention floor of a projection, per-tier forwarding budgets, the runtime floor. Each is set
  by whoever holds what it bounds: the person for their own device's tier, the space's members for a shared
  tier (§12.3 rule 3).
