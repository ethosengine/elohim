---
name: Reach is earned at authoring — and coupled to embodied responsibilities at every node
description: Burden of reach lies on author + stewarding peers. Receiver-side authorization is PRE-authorization (standing trust contract), never per-message filtering. Reach is coupled to embodied responsibilities for distribution/discovery/validation at every node.
type: project
originSessionId: f534b7ae-d435-4ab8-ab3b-f7d23b6b0ed9
---
The burden of reach lies on the **author and the peers that steward what they author**, to earn that reach. **Reach is coupled to embodied responsibilities at every node on the network** — for that reach to flow with meaning, purpose, and validation, the peers who carry it have to bear the responsibility of carrying it. Authoring-earning is one face of this; receiver pre-authorization is another.

**Why — the email lesson:**

Email collapsed because anyone could publish to anyone, putting the cost of filtering on receivers. The asymmetry — cheap to send, expensive to filter — produced spam at scale. The protocol now only works because megacorps (Gmail, Outlook) operate spam-filtering megaliths. The protocol exists; participating in it on a human scale does not. **That is the failure mode the Elohim Protocol must not replicate.** Receive-side per-message filtering is the slow-motion collapse, no matter how well-intentioned the filter.

**What's allowed on the receive side: pre-authorization, not filtering**

Receivers DO have authorization — but it's *pre-authorization*: a standing trust contract that puts the peer into the gossip topology, the Kad discovery graph, and the validation web for a given scope. The contract is established BEFORE messages flow, not evaluated per-message.

Concretely, when a peer takes on pre-authorization for a scope, they take on:
1. **Subscription** — receiving future gossip in that scope
2. **Distribution** — propagating gossip onward, serving Kad provider records, responding to fetch requests
3. **Discovery** — being available as a discovery node for that scope's content
4. **Validation** — participating in checking authors' membership/attestation/delegation claims for that scope

These are *embodied responsibilities*, not entitlements. A peer with no claim to a scope simply isn't in the topology — they don't subscribe, they don't appear in the Kad provider set, they don't get asked to vouch. There's nothing to "filter" because there's nothing to filter from. The graph topology IS the policy.

**Why this works at human scale:**

- Cost is borne by those who derive value (members of a scope steward that scope's distribution).
- Trust is anchored in relationship + membership + delegation, not perimeter security.
- The protocol stays workable for households and individuals because filtering megaliths are unnecessary.
- Spam attacks face the right asymmetry: a spammer must earn reach (relationship/membership/delegation) before they can publish at scope, and the earning is itself accountable on the DHT.

**How to apply:**

- **Author side (`FederatedEprStore::put`):** before publishing on Kad/gossip/direct-notify, validate that the EPR's signer has *earned* the declared reach. Refuse the put if not. Stage 1 = structural (signer is a known agent); Stage 2/3 = relationship / collective membership / delegation proof anchored in the envelope claims.
- **Receiver side (subscription / Kad / validation participation):** a peer participates in a scope only when they hold the standing pre-authorization for it — i.e., they have an embodied responsibility (membership, stewardship, relationship). This determines which topics they subscribe to, which provider records they advertise, which validations they perform. It is NOT a per-message filter.
- **Never:** "drop messages from unauthorized peers" / "register a gossipsub validator that filters by reach" / "block subscriptions to topics peers aren't authorized for from outside" / "rate-limit inbound messages." All variations of receive-side filtering are the email-collapse anti-pattern. If a plan / memory / PR proposes any of these, redirect to author-side earning + receiver-side pre-authorization based on embodied responsibilities.
- **Graph-first reading:** see memory pin `project_first_class_graph_pattern`. Reach is a property of graph topology, not bolted-on access control. Stage 2/3 verification = traversing the graph (DHT couplings) to confirm the author's membership/relationship/delegation backs the reach they claimed.

**Spec impact:** Phase 2B Decision #7 / Open Question O2 should be read as: "authorization proof" is what the author presents at publish time AND what receivers establish through standing pre-authorization (graph-derived, not network-derived). The phrase "subscription authorization" in any plan that pre-dates this principle should be reinterpreted as either (a) author-side reach earning, or (b) receiver-side pre-authorization rooted in embodied responsibilities — never as receive-side filtering.
