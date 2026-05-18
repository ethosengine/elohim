# **Resilience — Mutual Aid as Substrate, Not Workaround**

*The trillion-dollar problem of consumer technology, restated as a substrate question; the in-kind reciprocity primitive that dissolves it; and the recovery surface where the answer becomes testable.*

---

## **Executive Summary**

The defining failure of consumer-grade digital infrastructure is not that it cannot serve Grandma. It serves her enormously well — by every measure of convenience, the hyperscalers have lapped what households can build for themselves. The failure is in what Grandma trades for that convenience: her attention, her agency, and the texture of her humanity, auctioned in continuous fractions to whichever ad market clears highest at this microsecond.

This is the trillion-dollar problem. It is not solved by a better ad-tech disclosure regime, a more articulate cookie banner, or a cleaner data-broker market. It is not solved by *exit* — by Grandma deleting her account and going back to a flip phone and a paper calendar, because what she would lose is the connective tissue of late-life social participation, and the cost of that loss falls on her family, her neighbors, and the public commons. It is not solved by *encryption*, because encryption protects packets, not relationships. It is not solved by *self-sovereign identity*, because sovereignty assumes a kind of solitary competence that nobody — least of all an 84-year-old recovering from a TIA — actually possesses or wants.

The trillion-dollar problem is solved by changing what is *underneath* the convenience: by making the substrate itself reciprocal rather than extractive. By making mutual aid a first-class primitive of the network, expressed in the same Resource-Event-Agent ledger that already accounts for compute, content, and recognition. By treating the recovery surface — what happens when Grandma loses her device or forgets a key — not as a customer-support nightmare to be solved by a 1-800 number, but as the *test* that the substrate has been built right.

This chapter lays out:

1. Why the architecture of consumer technology cannot solve the dignity problem without changing its substrate;
2. Why **mutual aid expressed as REA in-kind commitments** is the protocol primitive that dissolves the trade between convenience and humanity;
3. How the **recovery surface** is the testable, deployable proof that the substrate works — for Grandma, by the grandma standard, on the alpha cluster, today;
4. The agreement model: how a reciprocal-backup relationship between two households (Gertrude ↔ Matthew/Jessica/James) lands as machine-readable, computable, breach-bounded, dignity-preserving structure on top of the same Commitment + FeedbackSignal primitives that already model compute stewardship;
5. The roadmap of feature files, role records, signal-kind extensions, and storyteller-canonical narratives that close the loop between the philosophical claim and the testable network state.

The grandma standard is the trillion-dollar test. If she's still herself when this works for her, we have built the substrate right. If she has to give up a piece of who she is to participate, we have built another hyperscaler, no matter what we call ourselves.

---

## **Part I: The Architectural Trap**

### **What Grandma Is Trading**

When Grandma signs up for a hyperscaler photo service, she is not making a one-time choice. She is entering a continuous, micro-incremented auction. The photo upload triggers an inference pass on her face, her grandson's face, the geolocation, the timestamp, the implicit social graph. The "free" service is paid for in fractions of attention so small no individual transaction feels load-bearing — but the aggregate, over a decade of grandma-using-the-network, builds a model of her that is more legible to the system than she is to herself. That model is then leased back to anyone who wants to influence her — political campaigns, payday lenders, supplement scammers, well-meaning relatives running ad campaigns, malicious nephews, the next generation of synthetic-media operators.

The transaction is not the *content* of her photos. The transaction is the *legibility* of her interiority to an extractive economy, in exchange for a service whose engineering quality she could not realistically build for herself. The convenience is real; the cost is real; and the architecture forces them into a single bundle.

What she is trading, in protocol-native terms:
- **Attention** — the scarcest resource a human possesses, captured in continuous fractions and resold as targeting surface.
- **Agency** — the ability to make consequential choices on terms she sets, eroded as the system gets more accurate at predicting (and therefore preempting) her preferences.
- **Humanity** — the texture of unmonetized relationship, of mistakes that don't enter a permanent record, of late-life development that isn't legible as engagement metric.

These are not abstractions. The Value Scanner persona [grandparent epic](../value_scanner/grandparent/README.md) describes Margaret, a retired nurse providing 25 hours of weekly childcare — substantial care economy participation that the current architecture cannot recognize as value at all. The same architectural shape that fails to *see* her unpaid care work is the one that aggressively *sees* her browsing patterns. The blindness is selective: invisible to what matters, exquisitely sharp on what extracts.

### **Why the Trap Has No Inside-Architecture Exit**

A natural impulse — and the one almost every reform movement has tried — is to fix the trap from inside the trapping architecture. Better privacy controls. Better disclosure. Better data-portability regulations. Better consent flows. Better default-off toggles. These efforts are not worthless. They reduce specific harms. But they do not change what Grandma is *trading*, because the substrate of the trade is unchanged.

The substrate is centralized custodianship: the hyperscaler holds the data, holds the keys, holds the recovery path, holds the social graph, holds the inference layer that turns her data into a market. *Every* reform that operates above this substrate inherits its incentive geometry. The hyperscaler's revenue, the recovery flow (call center, ID verification, password reset by email she can no longer access), and the surveillance affordance are the same architectural feature wearing three different labels. You cannot regulate away an architectural primitive.

The conclusion is hard but clarifying: the substrate has to change. The custodian-of-last-resort model has to be replaced. And the replacement has to *also* meet the convenience bar that put Grandma on the hyperscaler in the first place — because a substrate she cannot use, no matter how dignified, is one she will not use.

This is the trillion-dollar problem. Not "make a worse hyperscaler with better disclosure." Make a substrate that delivers the same convenience through a different geometry of trust — one in which the recovery path runs through the people she actually loves, not through a customer-support queue in another time zone.

### **The Pro-Social Alignment Imperative**

There is a second axis to this problem, intersecting the first: artificial intelligence. The same systems that auction Grandma's attention now also model her, predict her, generate synthetic versions of her voice, and increasingly *act on her behalf* — sometimes well, sometimes catastrophically, almost always without her able to verify what's been done in her name.

If the substrate is extractive, the AI built on top of it is extractive by composition. If the substrate is centrally custodial, the AI is centrally custodial — meaning a small number of operators decide what Grandma's AI agent is allowed to do, see, and remember, on whose behalf, with what alignment.

The alternative is not "no AI." The alternative is AI whose *incentive geometry is aligned with the human it serves*, because the substrate underneath it is. An AI agent that runs on infrastructure Grandma's household stewards (or that her relatives steward on her behalf, under explicit reciprocal agreement), with memory she controls, with capability bounded by her household's expressed values, is a fundamentally different kind of artificial intelligence than one running on a server farm whose revenue model is *correctly predicting what she'll click on next*.

This is the second pillar of the trillion-dollar argument: **a pro-social AI future requires a pro-social digital substrate**. The protocol layer is not separable from the alignment layer. They co-determine each other. Get the substrate right and alignment becomes a tractable engineering problem at the per-household scale; get the substrate wrong and alignment becomes an arms race between extractive operators and the small number of regulators trying to slow the worst of them.

---

## **Part II: The Substrate Answer — Mutual Aid as Protocol Primitive**

### **Why Resilience Is Not a Feature**

Resilience is not something we add to a digital service the way we add a backup or a redundancy. Resilience is *what a properly built substrate does* when stress arrives. If the substrate is built on reciprocal mutual aid — between households, neighborhoods, communities of practice — resilience is the steady-state. If the substrate is built on a single custodian, resilience is a never-ending arms race against the single point of failure that custodian represents.

The protocol commits — both philosophically and structurally — to mutual aid as the substrate primitive. Not as charity, not as backup-of-last-resort, not as an opt-in for the technically sophisticated minority, but as the *default geometry* of how data, identity, recovery, and care move through the network.

This is not new philosophy. It is how human societies have always done resilience: through families, neighborhoods, congregations, mutual benefit societies, fraternal orders, credit unions, barn-raisings, casserole brigades after a funeral, the rotating chair at a recovery meeting. Mutual aid is the oldest functioning resilience technology humans have. The novelty here is not the *pattern*. The novelty is that the digital substrate finally lets the pattern compose at protocol scale — that the same primitive a household uses to back up a grandparent's account can be used by a watershed council to back up its records, by a co-op to back up its books, by a religious community to back up its history — and the composition is computable, observable, breach-bounded, and dignity-preserving.

### **The REA Ledger as the Common Substrate**

The Elohim Protocol uses a Resource-Event-Agent (REA) accounting model — the standard descended from McCarthy's 1982 ledger work, extended in the ValueFlows ontology, and grounded into agent-centric distributed storage through hREA on Holochain. The REA model is described in detail elsewhere in this corpus ([Shefa Economic Infrastructure](../../../Shefa_Economic_Infrastructure_Whitepaper.md), [REA economics skill](../../../../../.claude/skills/rea-economics/SKILL.md), [protocol specification](../protocol-specification.md)); the part that matters for resilience is this:

> **Every flow of value in the network — content stewardship, compute hosting, learning recognition, care-work attribution, governance participation — is expressed as a typed Commitment between Agents over a Resource, fulfilled by Events, in the context of an Agreement.**

This includes flows that have nothing to do with money. The substrate does not assume a currency. It accounts for *in-kind* commitments — "I will host your content," "I will steward your child's learning record," "I will hold a share of your recovery key" — using the same Commitment + Event + Resource primitive that would account for a payment, if and when a payment were the appropriate Resource type. The accounting is the same shape; the Resource classification differs.

This matters enormously for resilience. It means that **the recovery agreement between Gertrude and the Dowell household is not a special, novel, recovery-only protocol object**. It is a **Commitment of the kind the substrate already understands** — with a Resource type that identifies it as a share-custody-of-recovery-key-material commitment, a state that progresses through proposed → accepted → fulfilled (or breached), and a FeedbackSignal stream that records exercise, breach, restoration, and any other event the substrate needs to remember.

### **Compute Commitments — The Pattern We Are Piggy-Backing On**

The model for how this works is already in flight, in a slightly different domain. The protocol has been building toward **compute commitments as bounded REA primitives** — see [`project_compute_commitments_bounded`](../../../../../.claude/memory/project_compute_commitments_bounded.md), the [`compute-commitment-bounds.feature`](../../../../a2o/features/deployment/compute-commitment-bounds.feature) scenarios, and the [`compute-allocation.feature`](../../../../a2o/features/elohim/compute-allocation.feature) lifecycle scenarios.

A compute commitment expresses: "this node, this household, this collective will provide *this many CPU-millicores and this much memory* to *this counterparty* under *these trigger conditions*, for *this duration*." It is a Commitment. It carries `resource_classified_as_json: ["compute"]`, a quantity (cpu_m, memory_Mi), a state, a counterparty, and a set of trigger kinds (request-driven, standing, subscription). Fulfillment is an Event. Breach is a FeedbackSignal of `signal_kind: "compute-breach"`. The doctrine — important — is that **breach never contaminates attribution**: if shem's power supply dies and the compute commitments backing adam/pete/frank go into breach, the *content they authored, the recognition they hold, the citations against their work* remain queryable and unimpaired. Compute-class flow and attribution-class flow are deliberately isolated, so that a hardware failure cannot silently re-rank a contributor's standing.

This is the design pattern. Recovery agreements are the *same shape*, extended through the substrate's extension primitive — the `signal_kind` field on FeedbackSignal — without inventing a new entry type.

| Compute commitment | Recovery commitment |
|---|---|
| `resource_classified_as_json: ["compute"]` | `resource_classified_as_json: ["recovery", "share-custody"]` |
| `resource_quantity: { value: 512, unit: "memory_Mi" }` | `resource_quantity: { value: 1, unit: "shamir-share" }` |
| `provider: shem-node-pubkey` / `receiver: alpha-cluster` | `provider: household-gertrude` / `receiver: household-matthew` |
| `action: "use"` or `"deliver-service"` | `action: "deliver-service"` (or a recovery-specific REA action) |
| Trigger: `standing`, `request-driven`, `subscription` | Trigger: `request-driven` (recovery is triggered by a claim event) |
| Fulfillment Event: service rendered, capacity consumed | Fulfillment Event: share presented when a recovery quorum forms |
| Breach signal: `signal_kind: "compute-breach"` | Breach signal: `signal_kind: "recovery-breach"` (e.g. share-holder unreachable at recovery time) |
| Attribution-isolated: breach does not silence the breach-er's authored content | Dignity-isolated: a recovery failure of one share-holder triggers a graduated-authority fallback (intimate circle → qahal → global witness), never a customer-support escalation |

The reciprocity is symmetric. Gertrude's household holds a share for the Dowells; the Dowells hold a share for Gertrude. Each direction is its own Commitment, anchored in its own Agreement, fulfillable independently. Either side can be exercised without the other. The pair, taken together, *is* the mutual aid relationship in machine-readable form.

This is what we mean by *the bureaucracy collapses into protocol*. The work that a credit union's recovery officer does, that a parish recovery committee does, that a family lawyer does when an aging relative loses access to their own accounts — that work has a shape, and the shape can be expressed in commitments. Not because the protocol *replaces* those human roles, but because it gives them a substrate that doesn't fight them: a substrate where the agreement they have always informally maintained becomes computable, queryable, breach-aware, and dignity-preserving.

### **Why This Is Not Just Cleaner Plumbing**

It would be easy to read the previous section as "we added a recovery feature on top of a clever ledger." That reading misses what is doing the work.

What is doing the work is that **the resilience of the network is now an emergent property of the agreements visible on it**, not a feature added on top. You can query the state of the network and *see* whether Grandma's recovery surface is healthy. You can compute, for any human, whether their content survives a household failure, whether their identity survives a device failure, whether their compute participation survives a node failure, whether their recovery survives a single-counterparty failure. Each of these is the same shape of question: *what Commitments exist, in what state, between which Agents, against which Resources?*

The protocol expresses this in the `human-resilience.feature` scenarios already in the corpus:

```gherkin
Scenario: Matthew alone — single conductor, at risk
  Then the protection status should be "at-risk"
  And the peer count should be 0

Scenario: Matthew + Susan — household reciprocation, partial protection
  Then the protection status should be "partial"
  And the peer count should be 1

Scenario: Matthew + Susan + Pete — community depth through trust topology
  Then the trust circle count should be 2
```

The resilience profile *is* the answer to a query against the commitment ledger. It is not a custodian's report. It is not a service-level agreement. It is what the substrate, looked at honestly, *is*. And it is testable — on the deployed network, on the alpha cluster, against the actual seeded humans and the actual seeded agreements, in CI, as part of the @e2e scenario suite.

This is what we mean when we say resilience is not a feature: it is the *shape of the substrate's truth*, made visible.

---

## **Part III: The Recovery Surface — Where the Proof Lives**

### **Recovery as the Testbed for the Whole Substrate**

The recovery flow is the moment when every claim the substrate makes is tested. If Grandma cannot recover her account — if the substrate cannot get her back in, with help from her people, at the speed and dignity she deserves — then nothing else the protocol does matters, because she cannot use it. The recovery surface is the *grandma-standard load-bearing test*.

There is also a deep architectural reason recovery is the right testbed. In normal operation, the substrate's resilience is *latent*: the commitments exist, the agreements are held, the share custodians sit quietly on their share material, the FeedbackSignals are unsignaled. Recovery is the moment the latent structure *activates*. It is where you find out, in production, whether the agreements you wrote down match the system you actually built.

For this reason, the recovery surface is also the highest-leverage place to invest scenario coverage. Every recovery scenario that passes is a load-bearing claim about the substrate. Every recovery scenario that fails is a falsifiable, fixable bug in the substrate's promise to its humans.

### **The Gertrude ↔ Dowell Agreement, End-to-End**

The reciprocal-backup relationship between household-gertrude (one human, on a `device-home-nuc` hub on shem) and household-matthew (matthew, jessica, james, on a `device-family-node-base` hub on-prem) is the **minimum viable backup relationship** the protocol commits to test. We have not yet hit full high-availability. We have not yet realized a fully-meshed quorum of five or seven share-holders across an extended family network. We have *one* reciprocal pair. That is enough to exercise the *complete recovery surface*: proposing the agreement, accepting it, holding the custody in steady-state, exercising it under lockout, handling breach, restoring the agreement after a substrate event (shem's death, shem's return).

The lifecycle, traced through the canonical stories already written into the corpus:

**Proposal and acceptance.** Matthew sits at Sunday dinner with Gertrude and explains, in the language of relationship, not cryptography, what he is asking her to do. The story [Gertrude Holds the Share](../../../../data/stories/gertrude-grandma--as-recovery-counterparty--backup-stewardship-for-household-dowell.md) is the narrative anchor for this moment. In machine terms, the moment lands as a Commitment in `state: "proposed"`, then `state: "accepted"`, between provider `household-gertrude` and receiver `household-matthew`, over a Resource classified as `["recovery", "share-custody"]`. The Agreement that anchors it carries the reciprocal pair.

**Reciprocal acceptance.** The Dowell household accepts the symmetric share — the share they will hold *for Gertrude*. The story [The Dowells Hold Gertrude's Share](../../../../data/stories/matthew-manager--as-recovery-counterparty--backup-stewardship-for-household-gertrude.md) is the narrative anchor. The elohim acts as counsel for the relationship itself, refusing the transactional framing before it can land — *"not because you owe her one."* The reciprocity is real, but the protocol does not let it collapse into a ledger debt. The pair of Commitments, taken together, *is* the mutual aid relationship.

**Steady-state.** Both households hold their share custody silently. The always-on hub-class device at Gertrude's place serves the responsibility without her thinking about it. The Dowell family-node-base does the same. Neither side issues an Event; the Commitment sits in `state: "accepted"` indefinitely. No interruption, no notification, no engagement-bait. The substrate *carries* the relationship without taxing the relationship.

**Exercise.** Gertrude's phone dies. She gets a new one. She tries to sign in. She has lost the device-bound key material; she does not remember a recovery phrase, because the substrate does not ask grandmothers to remember seventeen-word recovery phrases. The story [Gertrude Logs In with Help from Her People](../../../../data/stories/gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family.md) is the load-bearing end-to-end demonstration. In machine terms, a quorum of share custodians is queried. Each receives an *ambient* request — not a panic-inducing push notification, not a customer-support phone tree, just a small "Gertrude is trying to sign in on a new device. Tap Yes if that sounds right." David, her neighbor, taps Yes. Carol, her daughter, taps Yes. Matthew, in his utility closet, taps Yes. The protocol composes the shares, reconstructs the recovery material, returns Gertrude to herself. Total elapsed time: four minutes. Total bytes of jargon Gertrude saw: zero.

**Breach and graceful degradation.** What if one of the share custodians is unreachable? What if shem went down — *as it did, on May 4* — and one of the share-holder hubs went with it? This is where the breach-handling discipline of compute commitments transfers directly. A `signal_kind: "recovery-breach"` FeedbackSignal is recorded. The protocol does **not** lock Gertrude out. The protocol *graduates the recovery authority* — intimate circle, then qahal (community), then global witness — per the [`project_graduated_recovery_authority`](../../../../../.claude/memory/project_graduated_recovery_authority.md) doctrine. Absolute lockout is treated as a substrate failure, not a security feature. At the same time, breach does not contaminate dignity: Gertrude's content, her relationships, her care-work attribution, her affinity for gardening and family history remain unaffected by the recovery-class breach. Compute breach is to attribution as recovery breach is to dignity: deliberately isolated, by design.

**Restoration.** Shem returned on May 18. The substrate's reconciliation controller (see [`project_principle_p1_reconciliation_controller`](../../../../../.claude/memory/project_principle_p1_reconciliation_controller.md)) brought the share custodians back online — though, important detail, with fresh agent keys: the original chains were lost with the PVCs. The recovery Agreements between household-gertrude and household-matthew have to be **re-established on the new keys**. This is itself a recovery flow, run through the surviving counterparty. The substrate's commitment is that re-establishment is graceful: a new acceptance ceremony, a new pair of Commitments, the old ones moved to `state: "cancelled"` with an event of cancellation recorded. The history is preserved; the live agreement is current; the relationship continues.

This is the entire recovery surface, traced end-to-end, in the protocol primitives the substrate already has, against one reciprocal pair of households on the alpha cluster.

### **Computable State, Testable on the Deployed Network**

The whole point of expressing recovery agreements as REA Commitments is that the state of the substrate's resilience becomes *queryable*. Not as a custodian's quarterly report. Not as an aspirational marketing claim. As a typed HTTP response from the storage layer's `ReaCommitmentView`, returnable by a GET against the doorway, parseable by the same a2o step definitions that already verify the `human-resilience.feature` scenarios.

The kinds of queries that become testable:

- **Per-human resilience profile.** For any seeded human, compute their resilience tier (at-risk / partial / protected / fully-protected) by counting their incoming and outgoing mutual aid Commitments across all signal_kinds. This is the `human-resilience.feature` already in CI, with new scenarios for recovery-class Commitments specifically.
- **Per-agreement health.** For any seeded Agreement (e.g., the gertrude↔dowell reciprocal pair), verify both Commitments are in `state: "accepted"`, neither is in breach, and the share custodians are reachable. This is a new test surface — `feature-backup-stewardship-for-household-{dowell,gertrude}` — that the storyteller's coverage gap analysis surfaced.
- **End-to-end recovery flow.** For a seeded human (gertrude), simulate device loss, exercise the share-quorum recovery, verify the human is restored with all their attributions intact. This is `feature-social-recovery-with-help-from-family` — the highest-leverage coverage gap, because it is the *grandma-standard load-bearing scenario* and has no executable spec today.
- **Breach handling without contamination.** For a deliberate breach (shem-style PSU failure, or a more targeted simulation), verify recovery-class commitments enter breach, the graduated-authority fallback engages, and *no* attribution-class flow is affected. This generalizes the existing compute-commitment-bounds breach scenarios.

These are not aspirational. They use the substrate the protocol already has. The Commitment entry exists in the elohim DNA. The FeedbackSignal extension primitive is live. The storage projection (`ReaCommitmentView`, `AgreementView`) is implemented. The doorway HTTP surface for querying commitments is in place. What is missing is the *signal_kind extensions* (`recovery-share-custody`, `recovery-breach`), the *seed data* expressing the gertrude↔dowell Agreement, the *step definitions* that exercise the recovery flow, and the *feature files* themselves. Each of these is a known, finite piece of work, none of it requiring new entry types.

This is what the operator's directive means in practice: **the structural schema is not a new corpus**. It is the existing REA Commitment shape, extended via signal_kind, expressed in seed data, tested in feature files, queried over the existing HTTP surface. The recovery story is the story-first authoring layer; the feature files are the testable layer; the storage and DHT are the deployed-network layer. All three are coherent because they are speaking the same protocol vocabulary.

---

## **Part IV: The Roadmap From Here**

### **What Is Already In Place**

- The reciprocal-backup *narrative anchor*: three canonical stories under [`genesis/data/stories/`](../../../../data/stories/) covering the proposal direction, the reciprocal direction, and the end-to-end recovery flow. Operator-gated for canonical-flip; substrate-axis `delivery_status: undelivered` pending the implementation below.
- The reciprocal-backup *substrate participants*: gertrude on shem (`device-home-nuc` hub), matthew on-prem (`device-family-node-base` hub), household collectives seeded for both, account packages present for both. The deployment.json topology now expresses the minimum viable backup relationship.
- The *REA primitive* the agreements piggy-back on: Commitment, Agreement, EconomicEvent entry types in the elohim DNA; FeedbackSignal with `signal_kind` extensibility; storage projection through `ReaCommitmentView`; manifest-driven HTTP routes.
- The *test pattern* from adjacent domains: `compute-commitment-bounds.feature` demonstrates how to express a bounded mutual-aid commitment with attribution-isolated breach handling; `compute-allocation.feature` demonstrates the full provisioning + settlement lifecycle. The recovery feature suite mirrors this shape directly.
- The *resilience profile* baseline: `human-resilience.feature` already exercises the at-risk / partial / protected tiering against mutual aid commitments. Recovery commitments slot into this as a new counted signal_kind.

### **What This Epic Surfaces As Work**

In rough order of leverage:

1. **`feature-social-recovery-with-help-from-family.feature`** — the highest-leverage feature file in this epic. The end-to-end grandma-standard recovery flow has no executable spec today; every other claim in this chapter is unprovable without it. Storyteller-anchored by [Gertrude Logs In with Help from Her People](../../../../data/stories/gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family.md).
2. **`feature-backup-stewardship-for-household-dowell.feature`** and **`feature-backup-stewardship-for-household-gertrude.feature`** — the reciprocal pair. Tests proposal, acceptance, steady-state custody, fulfillment on exercise, breach on share-holder unreachability, re-establishment after a substrate failure. Storyteller-anchored by the two share-custody stories.
3. **`signal_kind` extensions** — add `recovery-share-custody`, `recovery-breach`, `recovery-quorum-formed`, `recovery-fulfilled` to the FeedbackSignal validator whitelist in the elohim DNA integrity zome. No new entry types; new vocabulary on the existing primitive.
4. **`role-as-recovery-counterparty`** and **`role-as-account-claimant`** role records in [`genesis/data/lamad/content/`](../../../../data/lamad/content/) — universal-shape roles surfaced as coverage gaps by the storyteller; pattern follows the `role-as-stewardee` and `role-as-collective-steward` precedent.
5. **Seed-data expression** of the gertrude↔dowell Agreement — a JSON record under a new seed corpus (the shape is open: it may live alongside existing collectives, alongside compute-capacity, or as a new top-level `agreements/` corpus that the schema points to). This is where the *machine-readable* form of the agreement lives, separate from the narrative. Story-first discipline: write the stories and feature files first, then the seed-data shape crystallizes against the actual queries the feature files run.
6. **Storage view extensions and HTTP route surfaces** for querying recovery-class commitments specifically — additional fields on `ReaCommitmentView` if needed, or compositions through the existing query surface. The doorway-manifest pattern declares the routes; storage implements; doorway projects.
7. **Step definitions** for the recovery feature files — extending the patterns established by `compute-allocation.steps.ts` and `resilience.steps.ts`. The bulk of this is composing existing primitives (creating a Commitment, transitioning its state, recording an Event, emitting a FeedbackSignal) into the recovery-specific sequences.

### **What This Epic Deliberately Does *Not* Do**

- It does not introduce a new top-level DHT entry type. The substrate's economy is unchanged; the recovery surface is an extension of the existing primitive.
- It does not specify the cryptographic share-custody mechanism (Shamir thresholds, key-derivation parameters, on-disk material format). Those are below the protocol layer this chapter operates at, addressed in the [protocol specification](../protocol-specification.md) and the security design documents. The claim here is structural: *whatever the share mechanism is, the agreement that governs its custody is an REA Commitment.*
- It does not address full-mesh recovery topologies, n-of-m quorum tuning, or recovery-network compositions across collectives larger than the household. Those are next-epic scope. One reciprocal pair is enough to exercise the surface; the substrate composes naturally upward when more relationships exist.
- It does not introduce a new corpus for recovery agreements. The structural schema question — `recoveryCircle` field on `humans.schema.json` versus a new `recovery-agreements/` corpus versus a coupling on the collective — is *answered by deferring*: the existing REA Commitment is the corpus. Any new field is either an `signal_kind` value or a `resource_classified_as` classification. Schema additions are protocol-law (per the [`feedback_schema_first_ioc`](../../../../../.claude/memory/feedback_schema_first_ioc.md) discipline) and are not warranted by this epic.

### **The Grandma Standard As Test**

The grandma standard, stated as a falsifiable substrate test, is:

> **Gertrude can lose her device and be back, restored to her full self with all her relationships and contributions intact, in under five minutes, with help from people she actually loves, without typing a seed phrase, without calling customer support, without reading a single word of cryptographic jargon, and without giving up any piece of who she is to the network or to its operators.**

When `feature-social-recovery-with-help-from-family.feature` passes on the alpha cluster, that test is met. When it fails, the substrate has not yet earned its grandma. The feature file is the trillion-dollar problem expressed as Gherkin: a hard, specific, deployable bar, against which any number of well-intentioned reform efforts can be measured and most found wanting.

This is not modest scope. The trillion-dollar problem of consumer technology is the trillion-dollar problem precisely because no one has built a substrate that meets this bar at consumer scale with consumer convenience. The claim of this protocol — load-bearing on every other claim — is that mutual aid expressed as REA Commitments, composed across reciprocal household relationships, exercised through ambient-rather-than-pushy notification flows, governed by graduated-authority recovery doctrine, served by elohim agents whose alignment is to the human rather than the operator, on hardware that includes the household's own recycled laptops and home NUCs and chromebooks — *is* such a substrate.

We are not there yet. We have a reciprocal pair on the alpha cluster, three canonical stories, an existing REA primitive, and a clear list of feature files to write. That is enough to start. The substrate, once the first reciprocal pair passes the grandma standard, composes outward at network scale without re-architecture — the same primitive that holds gertrude↔dowell holds household↔neighborhood↔congregation↔watershed-council, indefinitely, as long as the human relationships exist to ground it.

---

## **Closing**

The architecture of consumer technology asks Grandma to trade her humanity for convenience. The architecture of an extractive economy asks her to do this continuously, in fractions, without ever quite noticing. The architecture of a centrally custodial AI future will ask her to surrender judgment about her own life to a system whose revenue depends on getting that judgment subtly wrong.

The protocol's commitment is that none of these trades is actually necessary. The convenience is achievable — the engineering is hard but tractable. The dignity is preservable — the substrate just has to be reciprocal rather than extractive. The AI is alignable — the substrate alignment is what makes the AI alignment possible.

What makes this commitment more than rhetoric is that it is *testable*. Not in twenty years. Not at scale. Now, on the alpha cluster, against Gertrude and Matthew and Jessica and James, with one reciprocal-backup pair, three canonical stories, a finite list of feature files to write, and an REA primitive that already exists. The grandma standard is a Gherkin scenario waiting to be authored. The trillion-dollar problem is, structurally, four minutes of recovery flow, executed correctly, with help from her people, on a substrate that doesn't betray her.

That is the work. That is the entire work. Mutual aid as substrate, recovery as the test, Gertrude as the witness.

---

*Related chapters: [Manifesto](../manifesto.md) · [Constitution](../constitution.md) · [Protocol Specification](../protocol-specification.md) · [Shefa Economic Infrastructure](../../../Shefa_Economic_Infrastructure_Whitepaper.md) · [Grandparent (Value Scanner persona)](../value_scanner/grandparent/README.md)*

*Related stories: [Gertrude Holds the Share](../../../../data/stories/gertrude-grandma--as-recovery-counterparty--backup-stewardship-for-household-dowell.md) · [The Dowells Hold Gertrude's Share](../../../../data/stories/matthew-manager--as-recovery-counterparty--backup-stewardship-for-household-gertrude.md) · [Gertrude Logs In with Help from Her People](../../../../data/stories/gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family.md)*

*Related testbeds: [`a2o/features/shefa/human-resilience.feature`](../../../../a2o/features/shefa/human-resilience.feature) · [`a2o/features/deployment/compute-commitment-bounds.feature`](../../../../a2o/features/deployment/compute-commitment-bounds.feature) · [`a2o/features/elohim/compute-allocation.feature`](../../../../a2o/features/elohim/compute-allocation.feature)*

*Related memory: [`project_compute_commitments_bounded`](../../../../../.claude/memory/project_compute_commitments_bounded.md) · [`project_recovery_grandma_standard`](../../../../../.claude/memory/project_recovery_grandma_standard.md) · [`project_socially_derived_security`](../../../../../.claude/memory/project_socially_derived_security.md) · [`project_graduated_recovery_authority`](../../../../../.claude/memory/project_graduated_recovery_authority.md) · [`project_elohim_as_counsel`](../../../../../.claude/memory/project_elohim_as_counsel.md) · [`project_household_is_resilience_unit`](../../../../../.claude/memory/project_household_is_resilience_unit.md) · [`project_collapse_bureaucracy_into_protocol`](../../../../../.claude/memory/project_collapse_bureaucracy_into_protocol.md)*
