# elohim-hub

Runtime composition primitive for the Elohim Protocol's **hub layer**: the cluster of machines in one home or one collective that stewards its people's compute, federates with peer hubs, and takes on the aggregate-scale concerns that hyperscalers usually centralize. In plain terms: the code that turns several crates into one running service for a home or a collective.

**This README is a placement guide and an architectural orientation for a crate that has no code yet.** It says where concerns belong; where the code differs, the code is expected to move. If you came here to decide where a piece of code goes, read "The web seam" as far as the heading "Rules of the seam" and stop there. It assumes you know Holochain (its DHT is the shared, validated record; its conductor is the process that runs a device's Holochain apps) and libp2p and iroh, the two peer-to-peer transports storage uses. On all of them a peer is addressed by its public key and proves it holds that key, which is what "address and verify a peer by key" means below. The placement test routes between three crates: `doorway-service` (`doorway/doorway-service/`, the gateway that serves browsers and other web2 clients), `elohim-storage` (`elohim/elohim-storage/`, a peer's storage and sync service) and `elohim-hub` (this directory). `elohim-node` (`steward/node/`) is the process that links storage and the hub on a peer.

> **Status:** no code yet. This directory holds only this README; it reserves the name. The hub trait is sketched in `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md`, and that document's section "Doorway / hub edge" is the live statement of the doorway/hub split. Hub cluster code lives in the `elohim-node` crate (`steward/node/`) until a second consumer (operator UI, fixtures crate) needs the trait independently. The web-level hub work that already exists is still in `elohim-storage` and `doorway-service`; the next section lists it as debt.

Architecture documents named below by a dated filename live in `genesis/docs/content/elohim-protocol/architecture/`. Every other document is named by its full path. `genesis/` is the monorepo's root for documentation, stories and deployment data. In this README `elohim-hub` in code font is the crate, "a hub" is the box or cluster that runs it, and "the shell" is the person's own desktop app (the Tauri app in `steward/device/`).

## The web seam (decided 2026-10-06)

A *seam* is a boundary in this architecture where a concern is given a home. The seam map (`2026-06-21-elohim-seam-map-concern-routing.md`) catalogs them. The web seam is the boundary between hub and doorway for web-level work. Its two sides are called the hub side and the doorway side. A *projection* is a view of the protocol's truth made for an audience that cannot read it directly. A *head* is the declared current version of a piece of content: the address that says which bundle is the app right now.

The rules in this section are the target. The differences between them and today's code are listed as debt at the end of the section.

`elohim-hub` is the home for **web-level concerns a peer needs when the other end can address and verify it by key**. A peer reached by its key (by the person's own desktop app, called **the shell** in this README; by another device of the same person; by a household member) still has to serve an app's files, render a first page, hand over a bundle that can be verified, and say whether a version starts. That work lives here, so that `doorway-service` stays narrow.

**The placement test is one question: does this exist only because the other end cannot address and verify a peer by key?** A browser, a crawler and a web2 protocol cannot. They need a name a registrar vouches for and a certificate an authority signs. That accommodation is the doorway. A DNS name and its certificate are the usual sign of doorway work, not its definition. This applies the boundaries design's test for any projection feature, "is this serving browsers + other doorways, or nearby peers, or both?" (`2026-05-02-elohim-hub-boundaries-design.md`, "Doorway / hub edge"), to app serving and rendering. The boundaries design carries the seam in short form in that section. If the two ever disagree, the boundaries design is the authority and this section needs repair.

| Answer | Home | Examples |
|---|---|---|
| Yes: the other end is a browser, a crawler or a web2 protocol | the doorway role: `doorway/doorway-service`, plus its deployment manifests for issuing certificates | a registered name and the certificate for it, TLS termination (`src/tls.rs`), host-to-app routing, the cache that absorbs traffic arriving with no relationship, projection into ATProto (the Bluesky protocol) |
| No: the other end reaches the peer by key, and the work is serving HTTP or rendering | `elohim/elohim-hub` | serving an elected app bundle's files, server rendering (`elohim-render`, the JavaScript runtime for it), the deliverability verdict (whether an app bundle's bytes can start: the bundle has an `index.html` and holds every file that page names), which head a shell page is built from, render capability and its breaker, the runtime chrome element, the service-worker delivery contract |
| It is about bytes, heads, custody or reach themselves | `elohim/elohim-storage` | the blob store, release adoption, retention, the reach gate |

Two cases the test settles that a "does it need DNS?" reading gets wrong:

- **A household's own browser-only clients.** A browser on the home network still needs a name and a certificate before it will run a service worker. Nothing about it is public, and it is still doorway work: the doorway service, run on the hub box beside the hub runtime. The doorway's TLS listener is where this lands: it exists "so a household mesh can be reached at `https://…elohim.local` with a locally minted CA" (`doorway/doorway-service/src/tls.rs`).
- **A cache.** A cache absorbs traffic for speed, not because the client cannot address the peer, so caching is not a naming concern. The doorway's app-file cache stays in the doorway because of whose traffic it absorbs. The test for a new cache: one keyed by content (the bundle's address and the data heads that the render read) is the hub's; one that exists to absorb a named host's anonymous traffic is the doorway's.

**Speaking for a person is not a seam question.** The network holds a person's identity: a record that names which nodes may speak for them. No node is special, and a doorway is never required: "an identity can be created, and can grow to a second node, on a network where no doorway exists" (`genesis/a2o/features/auth/device-provisioning-paths.feature`). A doorway is a node run as a service for other people. When it holds a person's key and signs for them, its sign-in page says so.

Bootstrap and signal are rendezvous points and are outside this test.

**What to do with the answer.** This crate has no code yet, and the seam does not wait for it.

- New hub-side work goes in `elohim/elohim-hub/`. The first piece creates the crate: a `Cargo.toml` here, membership in the `elohim/Cargo.toml` workspace, and a gate project in the nearest `build-manifest.json`. Do not add it to `doorway-service`, and do not add it to `elohim-storage`, which is for bytes, heads, custody and reach.
- A concern can straddle the seam. Split it: the part that needs a name stays in the doorway. The same holds between hub and storage: storage decides which release is a channel's head and holds the bytes (`elohim-storage/src/services/release_adoption/`), and the hub decides which held version to serve, such as the last one whose bytes can start.
- A module in the debt table below stays where it is until it moves. If you change one, change it toward the seam: no new hostname or certificate reads in hub-side logic, and no new hub-side logic in the doorway.
- When a module moves, delete its row.

### Rules of the seam

They are the target; the debt table below lists where today's code falls short of them:

- **The dependency arrow points one way.** `elohim-hub` never depends on `doorway-service` and never reads a hostname or a certificate. The hub runtime serves one HTTP surface per peer: its own web routes together with the storage routes it composes. A doorway keeps its single target and caches what comes back. A hub box may run the doorway service beside the hub runtime. That is two binaries on one machine, and the crate rule still holds.
- **One node process composes both; storage never knows the hub.** The process that serves web-level work is `elohim-node`: one composition root that links `elohim-storage`, `elohim-hub` and the agent crates, on every peer-capable device. `elohim-hub` may use `elohim-storage`. `elohim-storage` links neither: it "stays oblivious to hub composition", and "the arrows do not reverse" (`2026-05-02-elohim-hub-boundaries-design.md`). The decision and its reasons are recorded below.
- **A doorway relays and caches. It does not execute a render or judge a head.** Whether a version starts is a property of the bundle's bytes, judged by the peer that holds them.
- **One module, three places.** The person's own device (the shell reaches it directly), a hub serving a household's thin clients by key, and the peer behind a doorway run the same code and reach the same verdicts.
- **Integrity comes from the substrate** (the protocol's own layers: the DHT notary and the peer-to-peer data plane). The hub executes only bytes verified against their content address. A render is keyed by the bundle address and the data heads it read, so another peer can re-derive it. A render never has more authority than the person asking: an authenticated request renders as that identity or falls back to client rendering, and an anonymous render reads only what an anonymous reader may.
- **"Hub" names the seam, not a required box.** A single laptop runs the same node process and serves itself. The dial runs from "serves only me" through "serves my household" to "serves a collective". It is a setting of that one process, never a change of binary, and nothing a person can do alone waits on it moving. This is the hub-optional floor: a single laptop with no hub is a full participant, and a hub is a convenience, never a gate (`steward/node/CLAUDE.md`).

### Debt against this seam

The rules above say where things belong. This table lists what the code has not caught up with, from a first reading of module headers. Each row is work to move or change, not a description of how things should stay. The table is for whoever moves a module; placing new code does not need it.

| Where it is now | What it is | Where it belongs |
|---|---|---|
| `doorway-service/src/render/{mod,types,registry,capability,breaker}.rs`, `src/ssr.rs` | which apps this node can render, how many renders it may run at once, when to stop retrying a render that keeps failing, and the glue between the renderer and the doorway's content resolver | hub |
| `doorway-service/src/render/{bundle_heads,coherence}.rs` | following which version of an app is current, and deciding whether that version can be served | hub judges; the doorway keeps only "is my cache at that head" |
| `doorway-service/src/routes/chrome.rs` | serves the runtime chrome element, which is built into `elohim-render` and used by the shell too | hub, by the test. The module header still calls the route "doorway-specific"; this row is a judgement against it |
| `elohim-storage`: `/apps/{slug}`, the extraction cache, `src/ssr.rs` (`POST /render`, `/spa/*`), `src/app_deliverability.rs` | app file serving, rendering and the deliverability verdict inside the storage binary | hub |
| the deliverability judgement, made twice: `elohim-storage/src/app_deliverability.rs` and `doorway-service/src/render/coherence.rs` | one judges the bytes, the other judges deliverability "through this doorway" | one judgement, in the hub |
| the render cache in `doorway-service/src/ssr.rs` | keys on the URL and a spec version, with a time limit | keyed by bundle address and data heads, in the hub |
| `app/elohim-app/src/apps-sw.ts` | the service worker that verifies, unpacks and caches a bundle | shared hub code that runs in the browser and does the same verification there (the hub's client half), not owned by one app |
| peers that run the bare `elohim-storage` binary (the desktop app's sidecar, the fleet's storage pods) | the storage binary serves app files and renders pages itself | the node process, `elohim-node`, serves them and embeds storage |
| `elohim-node` is a binary with no library target (`steward/node/Cargo.toml`) | a shell that cannot spawn a child cannot link it | a library with a thin binary |
| `elohim-node` is published by no pipeline (CI only gates it) and deployed by no manifest (`genesis/data/timeline/backlog/hub-enablement-dial-readiness-2026-06-21.md`), and calls itself the always-on runtime | no device runs it today | built and shipped wherever a peer runs; always-on only when its device is |
| the desktop app's bundle declares no sidecar binary (`steward/device/src-tauri/tauri.conf.json`), though `storage.rs` looks for a bundled one | a packaged desktop app has no node to start | the node bundled with the shell |
| `elohim-node`, which mounts only a minimal blob-serving storage HTTP server (`steward/node/CLAUDE.md`) | it cannot yet serve storage's routes and the hub's web routes as one surface | one HTTP surface per peer |
| an anonymous render in `doorway-service/src/ssr.rs` | fetches on the doorway's own authority (`Ambient`) | rendered in the hub with an anonymous reader's authority |
| renderers built from a bundle on disk named by `SSR_BUNDLE_PATH` (`elohim-storage/src/ssr.rs`, `doorway-service/src/render/registry.rs`) | when no bundle slug is set, a path and not a content address | built only from bytes verified against their content address |
| a household's own certificate: issuing it, rotating it, and getting the household's devices to trust it | not built beyond the development mesh's local certificate authority (`MESH_DOORWAY_TLS=1`); `doorway-service/src/tls.rs` loads a pair made elsewhere | the doorway role |

Not debt: `doorway-service/src/render/warm_shell.rs` and `src/routes/apps.rs` are edge caches and belong in the doorway.

### `elohim-node` is the node process on every device (decided 2026-10-06)

One composition root links `elohim-storage`, `elohim-hub` and the agent crates, and every peer-capable device runs it. Three reasons:

- **One composition root means one set of verdicts.** Three places that each wire storage and the hub together would drift apart. One cannot.
- **The dial is a setting, not a reinstall.** "The same laptop plugged in overnight, shared with the household, becomes a-hub-too" (`2026-05-08-iroh-libp2p-complementarity.md`). That only works if the laptop was already running the hub's process.
- **It is what the hub design already assumes.** The boundaries design says "the unit of deployment is the lightweight elohim-node binary; *design it to be cheap to run many of*".

Where the node sits under the compute envelope (`genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md`): that spec's envelope, `ark`, stays the parent of a device's processes, "one root envelope per device". The node is the child it supervises, in the place the spec gives to storage. On a hub box and in the fleet, `ark` starts the node. On a desktop the app's sidecar is the envelope and the node is its child. Where the operating system forbids spawning a child, as on a phone, the node is linked in-process. Rendering stays an advertised capability: a device that cannot sustain the renderer serves files and verdicts and leaves rendering to the client.

## How the seam is checked when an AI agent edits

This check applies to AI coding agents only; a file a person creates by hand is not seen. A `.epr-meta` file is a per-directory metadata file that holds governance rules (here, placement constraints applied when a file is created). A coding agent's pre-write hook (`.claude/hooks/epr-meta-resolver.py`) reads them each time the agent creates or edits a file there. The `web-seam-placement` rule in `doorway/doorway-service/src/.epr-meta` fires when an agent creates a new Rust file under `doorway-service/src/`. A matching rule in `elohim/elohim-storage/src/.epr-meta` fires on a new storage file that carries app-serving or rendering code. Each has the agent start a background reviewer that applies the placement test and reports back into that agent's session. Neither blocks the write. The doorway-side statement of the seam is `doorway/doorway-service/EDGE-DESIGN.md`.

From here on, this README is for a reader designing something the hub will carry, not only placing a file: what the hub composes, its two archetypes and the principles it inherits.

## What a hub is for

The protocol keeps truth in three layers, each with one job. `doorway/CLAUDE.md` names it the "three-layer truth model (DHT / libp2p / doorway-projection)"; `genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md` gives the fuller table:

| Layer | Job |
|---|---|
| Holochain DHT | notary: who said what, when |
| peer-to-peer data plane (libp2p in the cited documents' wording; `elohim-storage` also runs iroh) | data operations: holding and moving bytes between peers |
| doorway | optional projection of that truth to browsers and the web2 world (a projection is a view of that truth made for an audience that cannot read it directly) |

The hub is not a fourth truth layer. It is how the machines in one place run the middle layer together and face it toward the people nearby. In the seam map's words (§3.9), "doorway projects outward to web2; hub projects inward to nearby peers", and in the boundaries design's, "the hub stands alone without one". The boundaries design also draws "three layers", but of crates (`elohim-hub`, `elohim-node`, `elohim-storage`). That is a different axis from the three layers of truth.

A hub composes these crates into one runtime:

| Crate | What it is |
|---|---|
| `elohim/elohim-storage` | blob storage and peer-to-peer sync, run beside the Holochain conductor |
| `elohim/elohim-render` | the JavaScript runtime for server-side rendering |
| `elohim/elohim-compute` | shared compute reporting types and traits |
| `elohim/elohim-bitswap` | block exchange between nodes |
| `elohim/elohim-agent/` | a directory of crates: AI agent orchestration, and the gate client (the library a write path calls to ask an elohim agent whether an act is allowed) |

Five terms the rest of this README leans on:

- **Substrate.** The protocol's own layers beneath any app or projection: the DHT notary and the peer-to-peer data plane together.
- **Blade and fabric.** A blade is one machine in a hub's cluster. The fabric is that local cluster as a whole: blades that join and leave, as distinct from the wide-area peer-to-peer network.
- **Reach-earning.** Reach is how far a piece of content or a request may travel. It is earned before it spreads: "the burden of reach lies on the author and the peers that steward what they author, to earn that reach", the author's node refuses to publish what its signer has not earned, and receivers hold a standing pre-authorization instead of filtering each message (`2026-04-23-epr-phase-2c-libp2p-federation-design.md` §3.4.1; `genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md`).
- **Elohim-operator.** A software agent, not a human role: "a context-bound specialist agent" that "fills the role a household's devops/IT person would fill if they had one" (`2026-05-08-iroh-libp2p-complementarity.md`). It handles cluster operations (moving work between blades, leader election, replica placement) and drafts renegotiations for a human steward to witness and sign.
- **Substrate floor, elohim ceiling.** Every decision is made in two layers. The floor "always runs, requires no AI, and returns a deterministic outcome". The ceiling adds discernment when a household's elohim agent is present: it "enriches; it never gates" (`2026-05-04-compute-commitment-substrate-floor-design.md`).

What the composed runtime is for:

- **Federating with peer hubs.** The substrate scales by more hubs, not bigger ones: the design reach is about 100 million family-node-class machines, each carrying tens to hundreds of people (`2026-05-02-elohim-hub-boundaries-design.md`).
- **Applying reach-earning at four aggregate-scale surfaces**: compute (does this request earn hub cycles), distribution (does this traffic propagate across hubs), defense, and AI coordination.
- **Running an elohim-operator** for the fabric.
- **Taking on the concerns a hyperscaler would centralize**, without the centralizing: coordinated defense against floods and recognition of traffic shapes, inference for a household's phones, observer streams, moving workloads between blades.

The defense claim is a design thesis, not shipped code: "A pattern shaped like a DDoS attack is structurally just unearned distribution reach — it dies at the first unconvinced hub", so defense is "a side-effect of earning, not a bolt-on firewall" (`2026-05-02-elohim-hub-boundaries-design.md`, "The four reach-earning surfaces at hub scale").

## Archetypes

The hub has two archetypes. They share a trait surface and differ in **design attitude**. They are separate implementations on purpose, never one `Hub` with a setting, because "governance considerations do not degrade gracefully" (`2026-05-02-elohim-hub-boundaries-design.md`).

### DwellingHub (primary)

The cluster sized to one family in one dwelling: a known set of people who live together and steward it together. It may span several blades, and "a hub grows by adding blades, never by growing one process". Scaling out means more dwellings, not bigger ones.

**Attitude: co-presence.** The people and their elohim-operator are both present in the fabric. The fabric is visible, family members can intervene, and the operator helps alongside them without owning it. This is the protocol's claim that the intelligence revolution can "scale a system *to* human complexity rather than *away from* it" (`genesis/docs/content/elohim-protocol/manifesto.md`) at its most direct.

`HouseholdHub` is "a retired synonym for `DwellingHub`" (`2026-05-02-elohim-hub-boundaries-design.md`). Rust code has no hub type yet; the only trace is a payload field, `provider_dwelling_hub_id`. The retired name can still appear in generated reports and build output until they are next regenerated, and means the same thing.

### CollectiveHub

The same primitive for a collective: a church, a co-op, a patron circle, a DAO, a mutual-aid network.

**Attitude: delegated stewardship.** A collective expects its elohim-operators to carry more of the day-to-day fabric work, with people designating stewardship roles instead of each member operating the fabric. Both archetypes are accountable, and both are bound by the substrate floor and elohim ceiling.

A CollectiveHub is sized to its collective's stewardship contract and may span several physical sites. A doorway is typical for one, because collectives usually have a public face, but not mandatory.

## Doorway optionality

A hub may or may not host a doorway. "Doorway is OPTIONAL, not architectural": the peer-to-peer mesh is the hosting layer (`genesis/docs/content/elohim-protocol/architecture/MAP.md`). A dwelling runs the doorway service when something on the other end cannot reach it by key: the public web, or its own browser-only devices. A dwelling whose people all use peer-capable shells runs none. A person is not bound to one doorway either: "a human registers with several simultaneously" for resilience, and "no single doorway is the human's home" (`2026-05-23-doorway-access-tier-patterns.md`).

The doorway is one role a hub can take, not a mandatory layer. The doorway-side statement is `doorway/doorway-service/EDGE-DESIGN.md`.

## What goes here, eventually

This directory will hold the following. The web seam's first piece creates the crate, and the Hub trait joins it from `elohim-node` when a second consumer needs it:

- `Cargo.toml`: crate definition
- `src/lib.rs`: the `Hub` trait and core types
- `src/web/`: the web seam above (app serving, rendering, the deliverability verdict)
- `src/dwelling/`: the `DwellingHub` implementation
- `src/collective/`: the `CollectiveHub` implementation
- `src/federation/`: peer-hub federation manifest, gossip, contracts
- `src/operator/`: elohim-operator role manifests for hub scope
- `src/reach/`: the four reach-earning surfaces

Until then, the architectural truth lives in:

- `2026-05-02-elohim-hub-boundaries-design.md`: the hub trait sketch, and in "Doorway / hub edge" the live doorway/hub split
- `2026-06-21-elohim-seam-map-concern-routing.md` §3.9 and §3.12: where a doorway or hub concern is routed
- `steward/node/src/` (the `elohim-node` crate): the current implementation site for hub cluster composition

This README used to name `genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md` as its canonical design. That spec was retired to git history (commit `53190a234`) and compacted into the boundaries design's "Doorway / hub edge" section.

## Where each inherited principle is written down

This crate inherits these commitments. Each row names a document a reader can open. The memory entries this section used to cite by name were folded into these documents on 2026-06-03.

| Principle | Written down in |
|---|---|
| Three layers of truth: DHT notarizes, the peer-to-peer data plane operates, the doorway projects | `doorway/CLAUDE.md`, "No Blob Fan-Out"; `genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md` |
| The substrate scales by federating full nodes, counted in people carried | `2026-05-02-elohim-hub-boundaries-design.md` |
| The hub is the runtime composition primitive, not a deployment wrapper | `2026-05-02-elohim-hub-boundaries-design.md` |
| DwellingHub and CollectiveHub are separate implementations, not one hub with a setting | `2026-05-02-elohim-hub-boundaries-design.md` |
| A hub grows by adding blades | `2026-05-02-elohim-hub-boundaries-design.md`, "Horizontal scaling is the operator's placement job" |
| The elohim-operator manages the fabric | `2026-05-08-iroh-libp2p-complementarity.md`; story `genesis/data/stories/james-son--as-stewardee--stewarded-device-sync.md` |
| Substrate floor, elohim ceiling | `2026-05-04-compute-commitment-substrate-floor-design.md` |
| Reach is earned at authoring | `2026-04-23-epr-phase-2c-libp2p-federation-design.md` §3.4.1 |
| Reach-earning is the floor of a wider sense-and-respond contract | `genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md` |
| The household is the resilience unit | `genesis/docs/content/elohim-protocol/architecture/cluster-topology.md` |
| The intelligence revolution scales to human complexity | `genesis/docs/content/elohim-protocol/manifesto.md` |
| A hub is optional for any one person | `steward/node/CLAUDE.md`, "Philosophy rail — hub-optional floor" |

Two principles this README relied on have no restatement in the canon that a search on 2026-10-06 could find: that a household is only one kind of collective and stewardship contracts run between other collectives the same way, and that the protocol's aim is to make peer-to-peer feel as effortless as "it just works". Treat both as provisional. They are carried over from retired memory entries and wait on a restatement in the architecture documents.
