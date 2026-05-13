---
name: First-class graph pattern is the protocol primitive
description: The Elohim Protocol IS a graph. EPRs are nodes, couplings/memberships/delegations/attestations are edges. Reach, discovery, validation, and distribution all derive from graph topology. Design must support this as a first-class primitive, not a bolt-on.
type: project
originSessionId: f534b7ae-d435-4ab8-ab3b-f7d23b6b0ed9
---
The Elohim Protocol's primary abstraction is a **content-addressed graph**, not a publish/subscribe network, not a request/response API, and not a key-value store. EPRs are nodes (content-addressed envelopes carrying coupled story + value + governance); couplings and memberships and delegations and attestations are edges. Every protocol property — reach, discovery, validation, distribution, recovery, trust — derives from graph topology.

**Why — coupled story + value + governance flowing with meaning:**

The protocol exists to let "rich human-scale primitives for our coupled story + value + governance to flow with meaning, purpose, and validation through the network." That's only possible if the substrate first-classes the graph. A network that thinks in messages-and-topics can carry traffic but cannot carry meaning. Meaning requires:

- **Couplings as first-class edges** — knowledge ↔ value ↔ governance is the EPR coupling primitive. Stripping any leg breaks the unit.
- **Membership / relationship / delegation as first-class edges** — these aren't permission lookups; they're graph traversals that establish reach earning + pre-authorization + validation paths.
- **Reach as a topological property** — what a peer can author / subscribe to / steward / discover is computed by walking the graph from that peer outward. It is not a flag set on a row.
- **Discovery as graph traversal** — finding who has content X means walking from X's coupling/scope edges to peers in that scope. Kad providers are an operational projection of this graph, not the graph itself.

**Implications for design:**

- Any feature that asks "can peer P do X?" must ground the answer in graph topology, not in a permission table or a configuration file. The answer is "P can do X iff there is a path in the graph from P to a credential-bearing edge sufficient for X."
- Any feature that asks "who has content C?" must consider the graph-derived discovery set first, with Kad/gossip as operational projections.
- Phase 3's manifest-graph resolver and Phase 4's GraphQL surface are the surfaced graph layer; Phase 2B's substrate (epr_atoms, projector, fanout, reach earning) is laying the graph rails. Anything in 2B that contradicts the graph reading should be flagged.
- Storage projection tables are a *cache* over the graph — never the source of truth. The DHT-anchored couplings + memberships are the truth; the projector materializes views; the Kad provider records advertise availability of nodes; the gossip topics propagate change events.

**How to apply:**

- For any "can / cannot" question (authorization, discovery, distribution, recovery): formulate as graph traversal. If you find yourself reaching for an ACL or a config-file rule, you're solving the wrong problem.
- For any "who has X" question: formulate as discovery in the graph. The answer should be a set of peers with traversable paths to X's scope.
- For any "is this valid" question: formulate as walking from the asserted credential to ground truth in the DHT. Validation is graph traversal, not network authentication.
- Per memory pin `project_reach_earned_at_authoring`: reach is a graph-topology property. Earning = the author has the credentialed paths. Pre-authorization = the receiver has standing in the relevant subgraph. Both are graph queries.
- Per memory pin `project_three_layer_truth_model`: DHT is the graph (the manifest), libp2p is the controller observing/propagating it, doorway is the web2 projection. None of those layers is the graph itself except DHT.
- Phase boundary: Phase 4 GraphQL is the *surfacing* of the graph for federated query. The graph itself exists from Phase 2B onward as the substrate. The substrate must support graph reasoning even before Phase 4 exposes it.
