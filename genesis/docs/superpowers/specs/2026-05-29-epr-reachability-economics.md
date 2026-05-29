# EPR Reachability Economics & the Thin-Fediverse Doorway

> **Status:** Vision / design note (not an implementation plan). Captures a conceptual model
> developed 2026-05-29 while shaking out EPR-app delivery (`alpha.elohim.host` +
> `elohim.host` projecting one landing/lamad EPR). It reframes `project-epr`, the doorway,
> and account-graduation into one coherent reachability model, and sets the framing the
> near-term delivery sprint should honor. Near-term consequences are flagged inline; most of
> the model is future-work.

## Origin

We were debugging why `elohim.host/lamad` 404s. The mechanical answer is a missing storage
read route (tracked separately). But the deeper question surfaced: **is the doorway a
*projection* of pre-contracted content, or a *proxy/resolver* to any EPR the substrate
holds?** Working that through produced the model below.

## 1. The core unlock: reach and delivery are different axes

We had been collapsing two independent things into "commons reach":

- **Reach = permission.** Earned at authoring. *Who is allowed to see/read this at all.*
  Commons = nobody is denied. Private/intimate = strangers can't even see the head.
- **Delivery = economics.** *Who bears the cost of moving the bytes.* Always paid by someone;
  never free in the absolute.

A commons EPR is therefore **permissionlessly readable but not freely delivered.** "Commons"
never promised an infinite free CDN — it promised no gatekeeper. Separating these makes
"commons but metered" non-contradictory, and it is the healthier commons: it avoids
free-rider collapse by making delivery reciprocal.

### Heads vs bytes (maps onto the three-layer truth model)

- **Heads** (DHT, tiny, cheap) are served per **reach**. A doorway projects the entire commons
  head-graph for free: you see the EPR, its relationships/couplings, its steward, and the
  *shape of what lies beyond*. The map.
- **Bytes** (libp2p, expensive) are served per **reach AND delivery**. Free inside a sponsored
  frontier; beyond it, the requester must bring delivery funding.

Because the head is always visible (when commons), the graph is **honest about its own
frontier**: hitting the edge is not a 404 — it is a visible, locked node that says "there's
more here; cross by participating." The frontier is an *invitation*, not a wall — the
lived-contrast diffusion mechanic at the read path.

## 2. `project-epr` is a sponsorship / front-door primitive — not a sitemap

`project-epr` is **not** "permission to exist" and **not** a per-URL route table. It is:

> "I (steward or operator) sponsor free web2 delivery of this EPR (or this lens) at this
> named address, under these terms."

It is a compute-commitment where the doorway is the projector/receiver and the steward funds
projection. The projection set is therefore the **sponsored frontier** — the region of the
graph where delivery cost is already paid, so an anonymous browser routes for free.

**`/lamad` is a sensemaking crystal, not a shell.** It is a curated lens that imposes LMS
structure (paths, ordering, pedagogy) on an otherwise chaotic EPR graph, plus the
authoring/value-flow tooling. The lens is itself authored as EPRs and projected.

**Projection rule:** *project the organizing skeleton; resolve the leaves.* The crystal's
facets (courses, paths, stable curated traversal nodes) are worth sponsoring as named
deep-path projections (`/lamad/<deep-path>` *can* be a projection). The content leaves they
reference are CID-resolved against the frontier rule. The map is sponsored-free to browse;
following a reference *off* the map into native-p2p territory is where you meet the frontier.

**Near-term consequence (delivery sprint):** treat `project-epr` as a sponsorship/front-door
primitive. The seed creates a small set of named entry points (`/`, `/lamad`), NOT a
per-content sitemap. Do not assume "every EPR needs a project-epr commitment to be reachable."

## 3. Doorway wears two hats

- **Role 1 — Projector.** Named front doors (`/`, `/lamad`, deep-path lens facets). A handful
  of stewarded, curated, sponsored entry points. `project-epr` lives here and nowhere else.
- **Role 2 — Resolver / finance-bridge.** Given an EPR head/CID, query DHT provider records
  → "who replicates this?" → pull bytes over libp2p → cache → serve. No per-EPR contract.
  Reach is enforced **peer-side** (the serving peer decides whom to serve, by standing);
  the doorway enforces only its *own* sponsorship boundary and acts as the web2 + finance
  bridge that lets a browser participate.

Today only Role 1 is built. Role 2 (content-addressed resolve + standing-gated serve +
toll/finance-bridge) is the next epic and deserves its own P2P design gate.

## 4. Two delivery currencies — and why the asymmetry is the flywheel

Beyond the sponsored frontier, delivery is paid in one of two currencies:

- **Money (visitor, via doorway finance-bridge) — compensatory.** A micro-transaction to the
  elohim-network, split by **steward / collective / commons ratios** (the donut /
  constitutional ratios) to whoever bore the cost (e.g. the peer that served the bytes, the
  author/steward, the collective, the commons). It pays the **externality toll a web2 browser
  never sees** — but it does *not* make the content more available.
- **In-kind (steward, via replication) — generative.** You pull, cache, and pledge to host
  (a `custody-blob` / `replicates-*` compute-commitment triggered by a browse). Your access
  *adds a replica*, so the next pull is cheaper for everyone. You pay by **healing the
  commons.**

**The asymmetry is the thesis: visitors pay rent; stewards grow the commons.** Delivery cost
∝ 1 / (replication robustness): well-replicated commons content is virtually free (the
p2p-web's marginal-cost-zero, restored); boutique / single-replica content carries a toll —
until demand-driven stewardship caches enough replicas that it becomes robust and free.
**Demand drives replication.** Popular commons content heals itself; the long tail stays
toll-gated until someone cares enough to steward it. A self-balancing commons CDN with no
central host.

### Internalized externality (the moral core)

Web2 makes serving "free" by externalizing the cost into ads and surveillance. The elohim
paradigm **internalizes** it: the consumer pays the true, transparent cost of the digital
footprint they consume but do not steward, routed to the people who actually bore it. The
toll is not a paywall — it is honest pricing of an externality web2 hides. "Fruit back on
the tree" at the byte level.

## 5. The participation gradient IS the reachability gradient

1. **Anonymous visitor** — heads everywhere (commons), bytes only inside sponsored frontiers,
   or pay-per-reach via the finance-bridge toll.
2. **Hosted participant** — a doorway runs a conductor *on your behalf* (PaaS, hosting fees to
   the operator). That conductor accrues **standing** by contributing to the compute commons,
   and *standing is what peers check* before serving native-p2p bytes. The grandma path: no
   hardware, but you are imported into the p2p reciprocity economy.
3. **Device steward** — your own node; you resolve over libp2p directly; pulls of
   well-replicated content are virtually free; doorway is only your web2 fallback.

**The hosted-conductor is the bridge that imports a web2 visitor into the p2p reciprocity
economy** — it gives a browser *standing* peers will serve. The doorway sets the **address
dimension** (named context, the map); standing sets the **delivery dimension** (whether the
wilderness hands you bytes).

Standing (not per-read tolls) is the preferred unit for steward-side reach — ambient
membership, not a per-click toll-booth. The monetary toll is the *visitor* path; standing is
the *participant* path.

## 6. The thin-fediverse: doorways are commodity edges, not data-holding instances

- The **fediverse** federates *fat instances* that hold canonical data on rented
  infrastructure; the operator begs for donations because the bill scales with the data they
  hostage. Migration is agony (your data is captive).
- **Elohim** federates *thin edges over one shared content-addressed substrate*. A doorway
  holds **no canonical data** — only a projection, a cache, and a web2 service contract
  (TLS/DNS, named front doors, finance-bridge, conductor PaaS). The truth lives in the
  substrate (peers/hubs).

**Anti-monopoly by data-locality inversion:** a doorway *cannot* become Cloudflare/AWS
because it holds no hostage. Data gravity — the moat that makes hyperscalers sticky — does
not exist. Exit is free; doorways are commodity edges; competition stays live. The
friction-gradient is only the *backstop* for toll-flow concentration; the primary property
is that there is nothing to capture. Doorways are **the new Cloudflare, distributed onto home
hubs instead of data-centers** — frictionless to host *because* they are thin.

## 7. The anycast-CDN endgame

- `elohim.host` → nearest doorway edge (anycast / GeoDNS), which projects the EPR if it has
  projected reach, or resolves it (Role 2) for the visitor. Peers worldwide select a commons
  resource (a podcast, a course) served from *their* local doorway edge.
- **Origin = the replicated P2P substrate** — no single origin server, self-healing.
- **Content-addressing makes the cache coherent by construction:** EPRs are immutable (new
  version = new CID), so edges cache bytes *forever, for free*, with no invalidation protocol.
  Only **head resolution** (latest CID at a mutable name/path) needs freshness — small and
  cheap, and exactly the `project-epr`/head-pointer layer. **Immutable bytes cached eternally;
  mutable heads kept fresh cheaply.**

A content network with no center: CDN edges (thin doorways) + a P2P origin (the substrate).

### Addressing graduation (mirrors account graduation)

Naming flywheels off web2 too: web2 DNS + anycast (`elohim.host`, `did:web:apex.elohim.host`)
today → substrate-native naming (pkarr public-key records over the DHT — already on the iroh
Phase-11 gate list — and `did:peer`) at the endgame. Even *addressing* starts as a web2
bridge and migrates into the substrate, with no flag-day. This is how "P2P becomes the
foundation of a new internet" incrementally.

## 8. Doorways & nodes as stewarded, recoverable, governable assets

Doorways and the nodes they operate are themselves **stewarded assets on the DHT** — like a
human account, always recoverable and (in theory) governable by the larger network.

- **Already seeded:** the `operate-doorway` commitment carries a **succession role** — the
  recoverability hook. "Recoverable like a human account" is an existing primitive to grow,
  not greenfield.
- **Values-gate for the future work — *governable ≠ seizable*.** Doorway/node governance MUST
  inherit the same graduated, consent-based, anti-capture design as account-recovery
  (commons-elohim co-steward holding standing; intimate→qahal→global-witness escalation; no
  central authority). Otherwise "the network can govern a doorway" silently becomes "the
  network can censor or seize a doorway." Treat this as a hard gate when the epic opens.

## 9. Open questions (unresolved — for the Role-2 epic)

1. **Head-visibility** — purely reach-gated (commons heads free to all via doorway, private
   heads invisible to strangers), with the sponsorship frontier metering only *bytes*?
   (Working assumption: yes.)
2. **Unit of beyond-frontier reach** — standing-based (ambient membership), not per-read
   toll, for participants; monetary toll only for anonymous visitors. (Working assumption:
   yes.)
3. **Frontier enforcement** — doorway enforces its own sponsorship boundary; the *economic*
   frontier is enforced **peer-side** (their bytes, their standing check). If all
   doorway-enforced, we recreate a platform gatekeeper. (Working assumption: peer-side serve
   decision.)
4. **Toll settlement shape** — a boutique read may split across server (`serve-blob`),
   author/steward (custody/authorship), collective, and commons per donut ratios. Is the
   multi-party settlement the intent, or is that overkill for v1?
5. **Doorway governance** — the full design of recoverable/governable infrastructure under
   the governable≠seizable gate.

## Grounding (existing substrate & memory this builds on)

- `project_p2p_is_hosting` — peer-sharding IS hosting; doorway is optional web2 projection.
- `project_three_layer_truth_model` — DHT notary / libp2p data-ops / doorway web2 projection.
- `project_doorway_single_target_no_fanout`, `project_inventory_exchange_not_byte_replication`.
- `project_rea_compute_commitment_primitive`, `project_compute_commitments_bounded`,
  `project_dwelling_hub_replication_pattern` — the commitment rails the in-kind path rides.
- `project_friction_gradient_limitarianism`, `project_commons_elohim_co_steward`,
  `project_trust_as_efficiency_signal` — anti-concentration + cost-as-trust-signal.
- `project_peer_native_account_canonical_surface`, `project_m5_reframe_auth_portal_convergence`,
  `project_graduated_recovery_authority`, `project_socially_derived_security` — the
  participation/recovery gradient.
- `project_iroh_phase11_all_backends_wired` — pkarr gate for substrate-native naming.
- `bridges/CLAUDE.md` — the finance-bridge belongs at the doorway web2 surface, as a bridge.

## Relationship to the near-term delivery sprint

This note is mostly future-work. Its one binding near-term consequence: **the delivery sprint
treats `project-epr` as a sponsorship / named-front-door primitive** (a few entry points),
not a per-content sitemap, and leaves room for the Role-2 resolver + standing/toll economics
as the subsequent epic. Lighting up `alpha.elohim.host` + `elohim.host` on one EPR is the
first concrete instance of the thin-fediverse "one EPR, many doorways" property.
