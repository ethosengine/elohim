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

A compute commitment expresses: "this node, this household, this collective will provide *this many CPU-millicores and this much memory* to *this counterparty* under *these trigger conditions*, for *this duration*." It is a Commitment. It carries `resource_classified_as_json: ["compute"]`, a quantity (cpu_m, memory_Mi), a state, a counterparty, and a set of trigger kinds (request-driven, standing, subscription). Fulfillment is an Event. Breach is a FeedbackSignal of `signal_kind: "compute-breach"`. The doctrine — important, and currently expressed as architectural invariant rather than schema-level enforcement — is that **breach never contaminates attribution**: if shem's power supply dies and the compute commitments backing adam/pete/frank go into breach, the *content they authored, the recognition they hold, the citations against their work* remain queryable and unimpaired. Compute-class flow and attribution-class flow are deliberately isolated, so that a hardware failure cannot silently re-rank a contributor's standing. The isolation is documented discipline today, enforced by the projection layer respecting the classification distinction in code review and in scenario coverage; making it *structurally* enforced through a typed taxonomy split (a `compute-class` vs `attribution-class` partition on the Resource classification) is a named follow-up surfaced in the gap matrix below.

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

**Restoration.** Shem returned on May 18. The substrate's reconciliation controller (see [`project_principle_p1_reconciliation_controller`](../../../../../.claude/memory/project_principle_p1_reconciliation_controller.md)) brought the share custodians back online — though, important detail, with fresh agent keys: the original chains were lost when the hardware died and its persistent storage could not be recovered. The recovery Agreements between household-gertrude and household-matthew have to be **re-established on the new keys**. This is itself a recovery flow, run through the surviving counterparty. The substrate's commitment is that re-establishment is graceful: a new acceptance ceremony, a new pair of Commitments, the old ones moved to `state: "cancelled"` with an event of cancellation recorded. The history is preserved; the live agreement is current; the relationship continues.

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

## **Part V: Seeing What You Hold — The Stewardship Surface**

### **Mutual Aid Extends Past the Household**

The reciprocal-backup pair between Gertrude and the Dowells is the *minimum viable* shape of mutual aid expressed as protocol primitive. It is the smallest case that exercises the whole surface. But the substrate's promise — and the reason the trillion-dollar problem is solvable at consumer scale rather than at family scale — is that **the same primitive composes outward, indefinitely, across every reach the protocol already understands.**

Mutual aid agreements at household reach: backup share custody between gertrude and matthew, two parents agreeing to hold each other's child's learning record after a divorce, a household member quietly stewarding a sibling's medical directives.

Mutual aid agreements at neighborhood reach: a block's worth of households agreeing to steward each other's elder-recovery shares so that no single household becomes a single point of failure for the block, a homeschool co-op agreeing to host each other's curriculum decisions, a community garden agreeing to steward each other's plot records.

Mutual aid agreements at congregation / affinity / organization reach: a church holding directory and pastoral-care records distributed across its members' devices rather than rented from a SaaS platform, a credit union stewarding members' transaction histories on member-operated hardware, a small business stewarding customer records on the business owner's family hub plus a backup co-op of fellow small business owners.

Mutual aid agreements at commons reach: a watershed council stewarding the watershed's biodiversity records, a public-records co-op holding the county's court judgements and tax records, a podcast-supporter network stewarding the back catalog of independent journalism, a Wikipedia-shaped collective stewarding educational content, a public-domain archive stewarding orphaned cultural works.

Each of these is the same protocol shape: a Commitment, between two Agents, over a Resource classified by `signal_kind` and `resource_classified_as`, in a `state` that progresses through the lifecycle, with breach signals isolated from attribution. The compositional discipline is the same at every reach. The only thing that changes is who the counterparties are and what reach the Resource carries.

### **The Top-Level Surface — A Bar That Tells the Truth**

For a human staring at their screen — Gertrude with her recycled laptop, Matthew with his on-prem family-node, the small business owner with their home-NUC, the watershed council operator with their decommissioned office desktop — the substrate's claim has to *show up* somewhere they can see it. Not in a settings panel six clicks deep. Not as an opt-in advanced view. At the top level of the storage / compute surface, glanceable in a second:

```
  ──────────────────────────────────────────────────────────────────────────
  Your hardware                                       80 GB
  ──────────────────────────────────────────────────────────────────────────
  ▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░
  10 GB used (your own things)
  20 GB stewarded (held for others) — click to see who
  50 GB free
  ──────────────────────────────────────────────────────────────────────────
```

(Bar, circle, sparkline, whatever form the design layer settles on — the substrate's claim is invariant under presentation choice.) The three quantities are the load-bearing claims. **Used** is the steward's own footprint. **Stewarded** is what they're holding for others. **Free** is what's available to commit. The ratio of stewarded-to-used is — directly, gracefully, glanceably — *the measure of the human's pro-social participation in the substrate*. Not as a leaderboard. Not as a credit score. As a fact about the hardware, made visible.

### **Drilling Down — Three Classes of Stewardship, Three Postures**

A click on the stewarded slice opens the three-class breakdown that names what kind of pro-social participation is happening:

```
  ──────────────────────────────────────────────────────────────────────────
  20 GB stewarded — three kinds of holding
  ──────────────────────────────────────────────────────────────────────────

  Encrypted        ▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░    6 GB
                   Private shards for people you back up.
                   You cannot read these. You hold them so they can
                   come back to themselves if they lose their device.

  Social           ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░   12 GB
                   Data your communities trust you with.
                   Church directory. Co-op curriculum. Watershed
                   council minutes. The neighborhood's accounting.
                   You may be able to read it; the community
                   decides what is shared with whom.

  Commons          ▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░    2 GB
                   Public goods you carry a piece of.
                   Course material. County records. Biodiversity
                   archives. Map tiles. Independent journalism.
                   Anyone can read these; you help host them.
  ──────────────────────────────────────────────────────────────────────────
```

These are three different *postures* of pro-social participation. The substrate has been quietly carrying them as separate dimensions of the Commitment ledger; the UX makes them legible to the human stewarding the hardware.

**Encrypted custody** — opaque material the steward holds without read access. The recovery share-custody case from the Gertrude story sits here. So does any "I am holding the bytes but not the keys" arrangement: a Shamir share, a sealed envelope, an emergency cache for someone the steward loves. The ethical posture is *plausible deniability + load-bearing reliability*: the steward cannot even know what they hold; what they can promise is that it will be available when the counterparty needs it. In protocol terms: `resource_classified_as_json` carries `["encrypted-custody"]`; the steward's storage projects the bytes but the steward's elohim agent does not have the decryption material; the FeedbackSignal stream is the only thing the steward can observe (acceptance, exercise, breach).

In practice this class is built out of small high-trust mutual circles: Matthew, Jessica, James, and Gertrude all hold encrypted shares for one another, so that any member's lockout can be undone by a quorum of the other three. Every member sees, on their own stewardship surface, that some of their disk is held *for the people they love* — not as an abstraction, not as a backup-of-last-resort, but as a concrete number, attached to specific names, with a specific reason. This is the moment the substrate quietly sells itself. People do not need a whitepaper to understand it. They look at the bar, see "6 GB encrypted for Mom, Dad, James, and Grandma," and the entire P2P-as-civic-infrastructure claim lands without translation: *of course* the people who can return you to yourself are the people you actually know; *of course* the storage cost is the form mutual aid takes when expressed in bytes; *of course* this is what a substrate that respects you looks like. The visible cost is the visible relationship, and the relationship is the substrate.

**Social custody** — community-tier data the steward holds on behalf of a bounded social group. The church directory. The homeschool co-op's curriculum decisions. The watershed council's minutes. The neighborhood's shared accounting. The small business's customer records, stewarded by the business owner and a small co-op of fellow business owners. The steward may or may not be a member of the group; the *group's* governance decides who can read what. In protocol terms: `reach` is in {`household`, `neighborhood`, `community`, `organization`}; `signal_kind` carries the kind of stewardship being provided; the doorway projects member-visible queries onto the stewarded shards. This is where the bulk of the trillion-dollar problem actually moves: church directories, neighborhood association records, co-op books, small-organization data — *exactly* the kind of records that hyperscalers currently host as SaaS platforms with monthly rent and a surveillance side-channel. The substrate lets these move home to member-operated hardware.

**Commons custody** — public-reach content where the steward is hosting a piece of the digital public commons. Course material. Public records. Court judgements (anonymized appropriately). Tax records. Biodiversity archives. OpenStreetMap tiles. Independent journalism the steward chooses to support. Wikipedia-shaped collective knowledge. Out-of-copyright literature. Public health data. In protocol terms: `reach: "commons"`; the steward elects (or has elected on their behalf via household policy) which commons resources to carry; the substrate handles distribution, replication, and integrity. This is the digital-library-as-distributed-infrastructure layer — the kind of public-goods hosting that has historically required a Library of Congress, an Internet Archive, a university library, a national broadcaster, and that the substrate makes possible at distributed scale through individual humans carrying small shards.

**The civic distinction — attribution travels with the bytes.** A prior generation of peer-to-peer infrastructure — BitTorrent, the various Pirate-Bay-shaped trackers, raw IPFS hosting — proved that bytes can move at the edge without central custodians. What it could not do, structurally, was preserve the chain of recognition: the original creator of the work being moved was invisible to the protocol the moment the first copy left their machine. The hosting collapsed into anonymous byte-movement; the contribution collapsed into nothing. The substrate could neither acknowledge the author nor reward the steward; both became unnamed counters in a distribution graph. That architectural amnesia is what made the prior generation of decentralized hosting *exclusively* good at moving content that nobody was supposed to be paid for, and bad at almost everything else.

The Elohim substrate is the opposite architectural choice. Every commons shard carries attribution. The original contributor — whether or not they are present on the network — has a [ContributorPresence](../../../../../.claude/skills/rea-economics/SKILL.md) record in the substrate, a stable identifier the network maintains on their behalf. The ContributorPresence entry is a live DHT entry type with 32 fields — identity provenance, accumulating recognition (`affinity_total`, `unique_engagers`, `citation_count`, `recognition_score`), and the lifecycle states `unclaimed → stewarded → claimed`. The stewards holding commons shards generate `EconomicEvent`s of `action: "deliver-service"` against the underlying content, which flow recognition to the steward (for the hosting) and accumulate against the ContributorPresence (for the authorship). Both flows are visible in the REA ledger; neither is lost in transit; the accumulation runs continuously on the substrate that hosts the bytes.

When a podcaster who has been quietly hosted across the network for years finally creates an Elohim account and claims their ContributorPresence, the accumulated recognition becomes theirs *by right*. The transfer-on-claim mechanism is named and the fields are reserved on the entry — `claim_recognition_transferred_value`, `claim_recognition_transferred_unit`, eight `claim_verification_methods` ranging from email and ORCID to community-vouching and cryptographic proof. The path is designed; the substrate-level executor that emits the lump-sum transfer `EconomicEvent` on claim is itself a named gap in the matrix below — the entry shape is in place, the closing edge awaits implementation. What is load-bearing today is the structural commitment: the network honors original contributors by default, accumulates their recognition while they are absent, and reserves the transfer-on-claim machinery in the entry type that already runs in production. The trillion-dollar civic claim — *contribution survives transmission, present or not* — is currently load-bearing at the schema and accumulation layers, and named-but-unfinished at the claim-and-transfer layer.

This is the civic load-bearing claim: **the substrate always, always, always recognizes contribution, present or not.** A creator does not have to opt in to the network to be honored by it; the network honors them by default, holds their accrued recognition until they arrive, and transfers it to them on claim with full provenance intact. A steward does not have to know who they are hosting to be recognized for hosting them; the protocol books the recognition by class and provenance, and the steward is entitled — *by right of stewardship, not by transaction* — to participate in the value flow their hosting enables. Both directions are first-class. Both flow through the same REA Commitment + EconomicEvent ledger. Both are queryable on the deployed network. This is what makes Elohim a civic substrate rather than a piracy substrate, a commons substrate rather than a freeloading substrate, an infrastructure substrate rather than a distribution substrate. The bytes move. The recognition moves *with them*. Neither outruns the other.

### **The Composition Discipline**

The three classes are not arbitrary categories overlaid on the protocol — they are *queries against the existing Commitment ledger, partitioned by `reach` and by `resource_classified_as`*. Specifically:

- **Encrypted bucket** = `SELECT SUM(quantity) FROM rea_commitments WHERE resource_classified_as @> 'encrypted-custody' AND provider = $self AND state IN ('accepted', 'in-progress')`
- **Social bucket** = `SELECT SUM(quantity) FROM rea_commitments WHERE reach IN ('household','neighborhood','community','organization') AND resource_classified_as @> 'storage-stewardship' AND provider = $self AND state IN ('accepted', 'in-progress')`
- **Commons bucket** = `SELECT SUM(quantity) FROM rea_commitments WHERE reach = 'commons' AND resource_classified_as @> 'storage-stewardship' AND provider = $self AND state IN ('accepted', 'in-progress')`

These are not new database tables. They are not new entry types. They are *views over the existing REA Commitment store*, projected through the same `ReaCommitmentView` that already serves the compute-commitment scenarios. The UX layer (elohim-app, shefa pillar) renders these three numbers; the storage layer projects them from the live commitment ledger; the DHT notarizes the underlying agreements. Every layer is speaking the same protocol vocabulary.

This is the load-bearing point: **the visibility-of-participation is not a separate feature**. It is what the substrate looks like when you look at it. The protocol does not have to remember to *make* stewardship visible; stewardship is in the ledger, and looking at the ledger *is* the act of seeing it.

### **What the Visibility Buys**

Three things the substrate-as-visible-commons claim does that opaque hyperscaler hosting cannot:

**It makes pro-social participation legible without quantifying it into a market.** The bar shows "20 GB stewarded" — it does not show a leaderboard ranking. It does not award points. It does not create envy or competition. It does what mutual aid has always done: makes the contribution real enough to count without making it transactional enough to corrupt. The elohim agent stewarding the user's experience is explicitly the [*counsel for the relationship*](../../../../../.claude/memory/project_elohim_as_counsel.md) ([as in the Dowell-Gertrude story](../../../../data/stories/matthew-manager--as-recovery-counterparty--backup-stewardship-for-household-gertrude.md)), refusing the transactional collapse before it can land.

**It makes the commons feel real to the human carrying a piece of it.** A grandma whose 2 GB of disk is hosting a piece of the county's archived court judgements or her favorite podcaster's back catalog is *participating in the public commons in a way she can see*. That is the missing ingredient in most digital-public-infrastructure proposals: they have no surface where the contributor sees what they hold. The substrate's promise to her is not "trust us, the public goods are out there somewhere"; it is "look, 2 GB of your disk *is* the public goods, here is what."

**It makes the cost of extractive alternatives visible by contrast.** Once the user can see that 20 GB of their hardware is stewarded across three pro-social tiers, the next question is: where does the hyperscaler-equivalent of this work? When the church directory currently rented as SaaS could be hosted on the congregation's own family hubs in the *social* tier, the rental cost (in dollars and surveillance side-channel) becomes architectural rather than inevitable. The substrate's contribution to public discourse about consumer technology is, in part, this: *the stewardship surface makes the alternative legible*.

**It is the architectural proof that contribution survives transmission.** The commons-custody bar is not just a hosting receipt. It is a visible expression of the substrate's structural commitment: every byte the steward holds for the public commons is tied to a ContributorPresence — the original creator, present or absent, claimed or unclaimed — and to an `EconomicEvent` stream that accrues recognition both to the contributor and to the steward. When the steward looks at the 2 GB they are carrying for the public commons, they are not looking at orphaned bytes; they are looking at the protocol's promise to a podcaster, a journalist, a public-domain archivist, a community curriculum author, that the network *remembers them by default* — and the protocol's promise to the steward that hosting matters too. The visibility-of-participation surface is, at its base, the substrate's answer to "does my contribution survive being shared?" The answer the substrate quietly returns, in the texture of every bar and breakdown, is *yes, by design, indefinitely, in both directions*.

### **The Roadmap For Visibility**

The stewardship-surface UX is a deliverable, not a slogan. The pieces:

1. **Storage projection** — the `ReaCommitmentView` already exposes provider, receiver, resource_classified_as, quantity, state. Storage adds aggregation queries that bucket by the three classes above and serves them via a manifest-declared HTTP route. No DHT changes; no new entry types.
2. **Doorway route** — the doorway-manifest pattern declares a route like `GET /storage-stewardship/summary` that returns the three-bucket breakdown for a given agent. Doorway projects from storage.
3. **Angular surface in the shefa pillar** — a top-level dashboard widget rendering the bar / circle. Drill-down into each class. Per-counterparty visibility for the social tier (which church, which co-op). Per-public-good visibility for the commons tier (which podcasts, which archive shards).
4. **Feature files** — `feature-storage-stewardship-summary.feature` for the top-level bar; `feature-stewardship-class-drilldown.feature` for the three-class breakdown; `feature-storage-stewardship-changes.feature` for what happens when the user accepts or revokes a stewardship commitment.
5. **Story coverage** — a stewardship-class story per posture, anchored by a real human in the seed data: Gertrude on encrypted custody (already authored), a Dowell-household-style figure on social custody (homeschool co-op records?), a commons-tier steward (a podcaster-supporter or biodiversity-archive contributor) on commons custody. Each story names what it feels like to participate at that tier.

This work is downstream of the recovery-feature work in Part IV (recovery is the test that the substrate works at all), but it is the *same shape* of work, against the same primitives, surfaced through the same UX pillar (shefa). The visibility layer and the recovery layer are not separate efforts; they are two views onto the same Commitment ledger.

---

## **Part VI: The Patron-Enabled CDN — How Distribution, Succession, and Trust Become Substrate Properties**

### **Sheila's Test**

Sheila Wray Gregoire is a Canadian author and podcaster who built a public ministry around faith, marriage, and the harms of certain widely-circulating evangelical sex-and-marriage advice. She published books, ran the podcast, and over a decade gathered roughly ninety thousand followers on her Facebook business page — a working relationship to her audience, the place readers came to discover new material, ask her questions, and pass her work to friends who needed it.

Her page was hacked. The credential was stolen, sold, and ended up with an operator out of Macedonia who used the now-stolen account to broadcast pornography and clickbait into the feeds of her ninety thousand followers — under her name, in the voice the platform had taught those readers to expect. Followers reported. Patrons emailed. Volunteers filed forms. Months passed. Nobody at Meta ever picked up the phone. The architecture has no obligation in it that points back at her. The account is a credential; the credential is single-factor; the recovery path is a customer-support queue that does not exist in any operational sense for ordinary users; the audience she spent ten years gathering is held by a custodian whose revenue model has no line item for restoring her access. Meta will let the impersonation broadcast porn into the feed of every reader who once trusted her, and Meta will not pick up.

This is the architecture's silent failure mode, made specific. It is not a marginal case. It is the load-bearing pattern of centrally custodial consumer technology: ownership is a credential, credentials are stealable, the custodian holds the audience and the recovery path, and the custodian's revenue does not depend on giving any of it back when something goes wrong. Memory `project_no_sovereignty_stewardship_over_ownership` names this directly: ownership is the trap; stewardship is the protocol's structural inversion. *The substrate's answer to Sheila is not "we will run a better customer-support queue." It is "we will not hold your audience as a credential in the first place."* The recovery architecture from earlier in this chapter — Gertrude logging in with help from her people — is the same architecture that restores Sheila to herself. The trillion-dollar problem is a single problem with many tests; Sheila's hijack is one of them.

### **Stewardship Over Ownership — Anti-Capture as Substrate Primitive**

The protocol's anti-capture design is named in the [steward-affinity lifecycle](../../../../plans/2026-03-14-steward-affinity-lifecycle-design.md) plan: on centralized platforms, *"ownership" is a single credential — steal the credential, steal the page*. Anti-capture in the substrate is **earned standing across a web of demonstrated relationships**, not a single secret a hijacker can take. Four structural defenses compose:

- **Mastery gate** — A steward earns curation authority over content by demonstrating deep understanding of it, not by holding a password. Mastery is a substrate-attested credential, agent-signed, and *cannot be transferred by stealing a session token*.
- **Affinity accrual** — Standing is earned through sustained curation work and recorded in `steward_affinity` (content_id × steward_id × score). It is not granted by a platform; it accumulates over time through `curation-event` deltas; it does not move when an account credential moves.
- **Community resistance** — Other stewards have first-class governance standing (the qahal layer) to refuse hostile changes. Hijacking one credential does not coerce the rest of the curatorial web; the web's standing is independent of any single member's.
- **No single point of capture** — Stewardship is a multi-party relationship-web, not a single-key ownership claim. There is no analog of "steal the credential, steal the page."

The substrate primitive that operationalizes this is the **CustodianCommitment** entry type, live in the elohim DNA (`content_store_integrity/lib.rs:3289`). Four commitment types (`relationship` / `category` / `community` / `steward`); six selection bases (intimate relationship, trusted relationship, community member, category specialist, bandwidth capacity, geographic proximity); three shard strategies (full replica, Shamir threshold split, erasure coded); five emergency-trigger types (manual signal, trusted party, M-of-N consensus, dead-man's switch, beneficiary incapacity). The entry type's doc comment names Sheila's case as the canonical example of the **community-custody pattern**:

> *Community (100 of 100k followers) → Sheila's commons-reach content (democratic resilience)*

A hundred of her ninety thousand followers, each holding a small piece of the recovery quorum for her account, constitutes a democratic resilience that no Macedonian impersonator and no Meta queue can extinguish.

### **Patrons Are the CDN**

In the centralized model, the content-delivery network is a Cloudflare or an Akamai or a YouTube/Netflix backbone — owned by the custodian, paid for from the same revenue stream as the surveillance, optimized for the platform's engagement metrics, blind to the contributor whose work it moves. In the substrate, **the CDN is the patrons**. Every household, congregation, or affinity group holding a commons shard for a creator they support *is* the distribution edge for that creator's work in their part of the network. Aunt Carol's recycled laptop hosting two gigabytes of Sheila's back catalog is the CDN edge for Sheila's content in Aunt Carol's neighborhood; the bytes resolve from there, content-addressed, with attribution preserved in the same REA ledger that records Aunt Carol's hosting as a recognition-bearing contribution.

The architectural design for this is established. The [doorway-hub-edge spec](../../../superpowers/specs/2026-05-08-doorway-hub-edge-design.md) names distribution as one of four aggregate-scale reach-earning surfaces (alongside compute, defense, and AI-coordination), with **FANG-subsumption** as the explicit scaling target: aggregate compute (Google AI), aggregate distribution (YouTube/Netflix CDN), aggregate defense (Cloudflare), and aggregate algorithmic discernment (Facebook's feed). Hub federation absorbs these scaling concerns by *earning*, not by enlarging any single hub past what the humans inside can govern. The existing feature suite already exercises the substrate side of this claim: [`peer-mesh.feature`](../../../../a2o/features/delivery/peer-mesh.feature) — *"peers serve peers. Doorway becomes one source among many, not the mandatory funnel. The client discovers, scores, and selects the best delivery peer — preferring LAN over WAN, warm extraction over compressed, and falling back gracefully when peers are unavailable"*; [`web2-absorption.feature`](../../../../a2o/features/delivery/web2-absorption.feature) — *"the projection cache absorbs browser traffic patterns before they reach storage. Storage is a P2P node, not a CDN"*; [`content-addressing.feature`](../../../../a2o/features/delivery/content-addressing.feature), [`protocol-omnibar.feature`](../../../../a2o/features/delivery/protocol-omnibar.feature) (provenance display — *"protocol's equivalent of a browser address bar with SSL padlock"*), [`spa-bundle-delivery.feature`](../../../../a2o/features/delivery/spa-bundle-delivery.feature), [`transport-perf.feature`](../../../../a2o/features/delivery/transport-perf.feature).

What is not yet wired is the *story-side* expression — a `feature-patron-cdn-and-the-hijacked-page.feature` (the Sheila scenario as Gherkin), canonical stories for the patron-as-distributor role and the creator-as-stewardee role, and the storage-stewardship-summary surface from Part V exposing the commons-tier breakdown as visible patron contribution.

### **Discovery, Distribution, Trust and Safety — All Earned, Not Imposed**

Once distribution is a function of patron-held commons custody rather than a custodian-owned backbone, three downstream civic properties shift in shape together:

**Discovery** in the centralized model is whatever the engagement algorithm decides will keep the user clicking; what surfaces is what monetizes. In the substrate, discovery is *graph-walked* — through the [first-class graph pattern](../../../../../.claude/memory/project_first_class_graph_pattern.md) of EPRs as nodes and couplings/memberships/delegations as edges — and *reach-gated* (the gate from `services/reach_earning.rs`). A searcher finds content their standing allows them to receive, surfaced through the structural graph of attribution and patronage, not through an opaque ranking function. Creators with sustained standing accumulate discoverability because the substrate rewards earned reach, not because they bought it back from the platform that's selling it.

**Distribution** earns reach the same way every other substrate operation does — provenance, standing, receiver pre-authorization at each hop. A hijacker who steals a credential and tries to broadcast porn through Sheila's account does not get to amplify it, because *amplification is not a property of the account, it is a property of the cumulative reach-earning behavior of the agent over time*. An agent with no reach-earning history attempting fanout to ninety thousand receivers triggers the substrate's reach-gate, fails the receivers' pre-authorization, and dies at the first unconvinced hub. The "thousands of small targets" defense Cloudflare achieves through anycast is achieved here through federated reach-earning — each hub independently evaluates, the cost to attack is quadratic against the number of hubs to convince, and the substrate routes legitimate distribution while routing-around adversarial fanout.

**Trust and Safety** in the centralized model is a policy layer applied unevenly, after the fact, by an unaccountable enforcement mechanism, in service of legal and reputational cost-management for the custodian. In the substrate, **T&S is a structural property of the reach-earning gate + the social-recovery quorum + the graduated-authority delegation**. The impersonation attempt fails at the substrate floor for the reasons named above; the recovery quorum unwinds the hijack the same way it unwinds Gertrude's lost device; witnesses across the community record `signal_kind: "impersonation-claim"` against the suspected hijack and elohim-operators escalate through graduated authority. Adversarial actors do not face a customer-support queue that no longer picks up; they face a substrate that enforces what was declared at authoring time and a recovery quorum constituted by the people who actually know the impersonated party. T&S is what the substrate *is*, not a service the custodian provides on top of it.

### **Creator Succession**

Creators die. They retire. They pass the work to a successor or to a community. In the centralized model, the platform's dormancy policies eventually dissolve the account; the audience the creator built dissolves into the algorithm's general feed; the work becomes inaccessible or repurposed by whoever currently rents the namespace; the relationship between creator and audience evaporates because the platform never modeled it as a first-class thing. In the substrate, **ContributorPresence is a long-lived entry with 32 fields including stewardship transitions and claim-verification methods**, and the same graduated-authority delegation that recovers Gertrude can transition a creator's identity to a designated successor — a family member, a community trust, a co-op, a religious order, an estate. Recognition continues to flow; attribution remains accurate; the work doesn't vanish into "we delete dormant accounts after 24 months."

For collective-authored works — a co-op's curriculum, a congregation's directory, a watershed council's biodiversity records, an open-source project's documentation — succession composes even more naturally because authorship was always collective; the substrate just makes the collective custody explicit and the transitions visible. The protocol gives creators what no platform has ever given them: *durability across generations*. The work survives the creator; the audience survives the platform; the relationship between them survives both.

### **The Virtuous Cycle on a Shared Ledger**

The three flows compose on a single REA ledger and reinforce each other:

- **Patrons** holding commons shards receive recognition flows for the hosting (`signal_kind: "stewardship"` over commons-reach resources). The 2 GB Aunt Carol carries for Sheila is visible to Aunt Carol on her stewardship surface, is recognized by the network, is part of how Aunt Carol's own standing accrues.
- **Authors** receive recognition flows attached to their ContributorPresence — accumulating affinity from engagement, citation count from references, recognition score across the network — flowing to the steward today and reserved for transfer-on-claim when an absent contributor arrives.
- **The commons** receives a substrate that does not extract. No ad-tech side channel funded by Aunt Carol's hosting. No algorithmic re-ranking optimized for someone else's revenue model. No surveillance affordance riding on the distribution layer.

Each layer makes the others possible. Patrons want to support creators whose recognition the substrate durably honors. Creators want to publish where their work survives them and where their relationship to readers is structural, not platform-mediated. The commons benefits from a substrate where pro-social participation is visible, accountable, and rewarded structurally — without being financialized into a marketplace that re-creates the extraction it was meant to escape. The cycle is the same one mutual aid has always run on; the substrate makes it computable, queryable, breach-bounded, and dignity-preserving.

### **What Is Built — and What the Sheila Scenario Closes**

For Distribution, Discovery, Succession, and T&S as substrate properties, much of the foundation is LIVE: CustodianCommitment entry type and coordinator functions; steward_affinity table and pipeline integration (Stage 2 wired against affinity scores); the doorway-hub-edge architecture for federation; the peer-mesh + web2-absorption + content-addressing + protocol-omnibar + spa-bundle-delivery + transport-perf feature suite; the reach-earning gate at authoring; ContributorPresence with stewardship and claim fields; the FANG-subsumption design in `2026-05-08-doorway-hub-edge-design.md`; the graduated-authority recovery doctrine.

What is named-but-unfinished, and what the **Sheila scenario** would exercise end-to-end:

- `feature-account-takeover-recovery.feature` — the Sheila hijack as Gherkin: account stolen → impersonator attempts fanout → reach-gate breach + receiver pre-authorization failures + community impersonation-claim signals → graduated-authority quorum revokes the impersonation → original creator restored. This is the highest-leverage T&S scenario the substrate can prove, and it has no executable spec today.
- `feature-creator-succession.feature` — designated successor receives ContributorPresence via graduated-authority delegation; recognition flows continue; audience relationship intact.
- `feature-patron-cdn-discovery.feature` — content searcher resolves to nearest patron-edge through reach-walked discovery rather than to a corporate cache.
- `signal_kind: "impersonation-claim"` validator whitelist addition + standing-policy debit weights.
- Storage-stewardship summary route's commons-tier drill-down showing per-creator patron-CDN composition (which patrons hold what fraction of which creator's commons-reach content).
- Canonical stories under `genesis/data/stories/`: a Sheila-shaped persona on `as-creator-under-impersonation-attack`; a patron-shaped persona on `as-commons-custodian-for-a-creator-they-support`; a successor on `as-inheritor-of-contributor-presence`.

These complete the resilience epic's civic claims — not as new architecture, but as vocabulary added to a substrate that already speaks the language. The substrate's existing primitives carry the weight; the Sheila scenario is the executable spec that proves they carry the weight specifically for the trillion-dollar failure mode the architecture of consumer technology cannot solve.

---

## **Part VII: How the Substrate Composes — and Where It Stops**

### **Threading the Claim Through Real Code**

The trillion-dollar claim — that mutual aid expressed as REA Commitments is the substrate primitive that dissolves the convenience/dignity trade — has to compose through the actual code, not float above it. This section threads the resilience flows through the layers of the substrate as they exist today, and as they are being built. The point is not exhaustive specification; the point is to ground the philosophical claim in a real stack with a real implementation gradient, and to name honestly what is built, what is designed-not-yet-wired, and what is still aspirational. The gap matrix at the end of Part IX is the precise accounting; this section is the architectural walk.

### **The Layers**

The substrate composes through three structurally distinct layers and one optional projection (memory `project_three_layer_truth_model`):

```
DHT (Holochain)             ┃  notarized, expensive, narrow.
                            ┃  agent-signed entries; content-addressed; non-repudiable.
                            ┃  the only layer that can say "this fact is now true forever."
─────────────────────────────╂─────────────────────────────────────────────────────
libp2p + iroh (dual-stack)  ┃  data ops, per-peer, cheap.
                            ┃  the working surface — moves the bytes, runs the gossip,
                            ┃  exercises the agreements, projects the topology.
─────────────────────────────╂─────────────────────────────────────────────────────
Hub composition             ┃  runtime composition + federation + elohim-operator.
(elohim-hub)                ┃  treats the hardware as a cluster; absorbs hyperscaler-class
                            ┃  concerns inside the household, federates horizontally.
─────────────────────────────╂─────────────────────────────────────────────────────
Doorway (optional)          ┃  web2 projection surface.
                            ┃  AT-Protocol / ActivityPub / OAuth-RP / HTTP/SSR.
                            ┃  the only layer the public internet sees.
```

Each resilience flow described earlier in this chapter — proposal, acceptance, steady-state custody, exercise, breach, restoration — threads through these layers in a specific way. Walking one full pass:

**Notary (DHT) — what the agreement IS.** The Gertrude ↔ Dowell reciprocal-backup Commitment is a `Commitment` entry in the elohim DNA (`content_store_integrity/lib.rs:1336`), with `resource_classified_as_json` carrying `["recovery","share-custody"]`, a quantity (1 shamir-share), provider/receiver pointing to the household-collective IDs, and a state in the live state vocabulary (`proposed → accepted → in-progress → fulfilled / cancelled / breached`). This is LIVE; the entry types are deployed, the validators run on every peer, the validator-side discipline that HDI rules forbid `get_links` (memory `project_hdi_no_get_links_in_validators`) means cross-entry verification is the coordinator's job.

**Notary → data-ops bridge.** The Commitment, once notarized, projects into the SQLite layer (`elohim-storage/src/db/rea_commitments.rs`) through the *reconciliation controller* pattern (Principle P1, memory `project_principle_p1_reconciliation_controller`). The controller is implemented at `elohim-storage/src/reconcile/controller.rs` — currently as a *skeleton* with real handlers for imagodei/M5 recovery signals (`on_key_rotation`, `on_key_revocation`, `on_agent_peer_binding`, `on_revocation_attestation`, `on_portal_host_created/removed`) and stubs for the rest. The pattern takes inspiration from container-orchestrator controller shapes (observe → reconcile → no hesitation) but operates on substrate-native protocol signals, not on cluster-orchestrator lifecycle events — the observer is the DHT, not a cluster API. The implementation surface today is targeted at the recovery-and-key-management signals that M5 needs. **The recovery-class signal handlers — `recovery-share-custody`, `recovery-breach`, `recovery-quorum-formed`, `recovery-fulfilled` — are not yet wired in the controller**; they are the next-leverage extension once the corresponding `signal_kind` whitelist edits land in the integrity zome.

**Data ops (libp2p + iroh) — what the agreement DOES.** Phase 11 of the iroh integration landed (memory `project_iroh_phase11_all_backends_wired`) with all seven planes wired and 43 tests passing. The architecture is permanent dual-stack (per `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`): iroh 0.92 + iroh-blobs 0.94 for hub-to-hub federation (BLAKE3 chunked streaming wins 4×–290× p50), libp2p 0.54 for consumer-grade-direct (intermittent, UDP-restricted, browser-WebRTC). Both transports preserve DHT-derived integrity equally; the choice is governed by per-peer transport-profile manifest, not by a security claim. Phase 12 — `peer_transport_manifest` as permanent schema — has substrate landed (migration `2026-05-10-120000`, `peer_map` API at `p2p_iroh/peer_map.rs:1-150`) and plan-tracking RED (0/59 boxes ticked); the four iroh adapter wiring tasks are partially landed per the in-tree README. This is the layer that moves the actual bytes of an exercised recovery agreement; the substrate is in place, the consumer-grade soak is named in gate #9 of the iroh master plan and is open work.

**The topology↔REA bridge — live and queryable.** Per the *Light Up the Topology* design (`genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`), the protocol uses three live REA action conventions to bridge stewardship to topology: `project-blob` (Commitment: doorway commits to project a blob), `serve-blob` (EconomicEvent: doorway fulfills projection), `custody-blob` (Commitment: peer steward commits to custody). These are LIVE — queried in `reciprocity_view.rs:114-147` for committed-bytes math, in `cluster_view.rs:47, 202, 217` for join-to-hosting-count, in `device_capacity.rs:92` for capacity-minus-committed arithmetic, and in `rea_projection.rs:180` for the signal handler on `AgreementCommitted | ReaCommitmentCommitted | ReaEconomicEventCommitted`. The 3-bucket stewardship surface from Part V (encrypted / social / commons) is a *view* over this live infrastructure, filtered by `reach` and `resource_classified_as` — no new tables, no new entry types.

**Light Up the Graph — signals actually flow.** The companion *Light Up the Graph* sprint (`2026-05-01-light-up-the-graph-design.md`) LANDED on commit `4ea4e1558`. Pre-LUG, integration tests passed only via mock-outbound substitutions; the graph entities existed but their effects did not propagate at runtime. Post-LUG, six wiring sites are live: `api/epr.rs::put_epr` fan-out, bootstrap-manifest seeding, manifest-debit weight policy, the libp2p outbound sink + gossip publisher, the reach-earning gate (`services/reach_earning.rs:94-129`), and the Vouch primitive as `signal_kind` extension. **LUG is the proof-of-shape that the resilience epic's `signal_kind` extension claim is structurally sound** — Vouch was added without a new entry type, the validator whitelist was extended, the manifest carries the debit weight, and downstream projection respects it. The recovery-class `signal_kind` values follow exactly the same pattern.

**Light Up the Topology — substrate is legible.** The companion LUT sprint is PARTIAL. Five view modules exist (`services/{distribution_view, cluster_view, peer_topology_view, reciprocity_view}.rs`), `peer_transport_manifest` migration is on disk, the dual-publish topic directory is populated, but the M1 substrate-completion plan (`2026-05-07-topology-substrate-completion-m1-plan.md`) is the unblocking sprint — 256 unchecked tasks at plan-write. Today's `peer_topology_view.rs` still carries M3 TODOs at lines 332, 394, 405, 491, 544 for per-peer authored-CID count, batch GROUP BY, `last_sync_sec` derivation, and the `resilience_cliffs` stub returning `vec![]`. LUT is the layer that makes the stewardship surface from Part V *renderable in the UI*; M1 completion is what gates the resilience-surface widget shipping.

**Doorway (web2 projection) — the public face.** The doorway is the optional layer where the substrate becomes legible to web2 consumers (browsers, mobile apps, AT-Protocol / ActivityPub federation). Memory `project_doorway_views_through_not_owned` is load-bearing here: views are *served through* a doorway, not *owned by* one. The doorway-manifest pattern (memory `project_doorway_manifest_driven_routes`) declares routes; the storage layer implements them; the doorway proxies them. The recovery surface's `GET /storage-stewardship/summary` route lives on this layer and is doorway-manifest territory — not a doorway-authored endpoint.

### **The Care vs Compute Boundary — Discipline, Not Yet Schema**

A claim earlier in this chapter — that compute-class flow and attribution-class flow are deliberately isolated, so a hardware failure cannot silently re-rank a contributor's standing — is currently *documented discipline rather than structural enforcement*. The `RESOURCE_CLASSIFICATIONS` whitelist (`content_store_integrity/lib.rs:238-257`) mixes 16 values across both classes (`content`, `recognition`, `stewardship`, `compute`, `currency`, six shefa tokens). The compute-vs-care isolation is real — `project_compute_commitments_bounded` explicitly states it as architectural invariant — but the substrate enforces it through the projection layer's discipline (the views *don't* compose compute breach into attribution score) rather than through a typed schema partition. A future change that makes this isolation *structural* (a typed split on the Resource classification, validators that reject crossings, projector code that's actually unable to violate the rule) is the closing edge that would make the claim load-bearing at the schema level.

The trust compute gradient is in similar shape. The 707-line brainstorm at `genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md` lays out a seven-layer gradient (DHT-notarization unchanged, libp2p gossipsub modulated, Kad provider records modulated, schemaRef walks modulated, projection caching modulated, validation amortization modulated, cold-fetch peer selection modulated) — with the trust-as-efficiency-signal frame (memory `project_trust_as_efficiency_signal`) and five constitutional floor classes (LIVE as the `FloorClass` enum at `reach_earning.rs:17-24`). The substrate primitives (FeedbackSignal, standing-policy manifest, reach-earning gate) are LIVE; the seven-layer gradient itself is mostly unwired. Standing composition memory (`project_standing_composes_multiple_evidence_streams`) says standing should compose from imagodei profile + lamad recognition + FeedbackSignal debits; today only the third stream is wired, and the schema seam (`unknownTreatment.evidenceSources`) is forward-compatible but unbridged.

### **The Critical Gap — Node-Health Observable to REA Event**

The one substrate gap that *most directly* affects the resilience epic's claims is the **node-lifecycle observable** edge. The bridge today is *unidirectional*: every elohim-node, at boot, registers its committed shape as a `NodeRegistration` DHT entry via `boot_registration::register_at_boot` (`elohim-storage/src/services/boot_registration.rs:147-170`), calling the `register_node_shape` coordinator zome at `node-registry/.../shape.rs:48-76`. That is LIVE — the act of deploying a node DOES create an agent-signed, content-addressed shape claim. What is NOT live is the inverse edge: there is no substrate-native node-health observer that emits breach signals when a committed node stops fulfilling its commitments. When a peer goes silent — through hardware death, sustained unavailability, exhausted capacity, or any other observable mode of falling-short — the protocol does not autonomously emit a `compute-breach` EconomicEvent. The May 4 hardware death on the alpha cluster was handled by a human operator editing the seed-deployment record (`suspended: true` flipped, narrative preserved in `$comment` blocks); the protocol-level Event ledger stayed silent.

The closing of this edge is substrate-native and protocol-internal: peers gossip liveness, peers observe each other through libp2p connection state and through the EPR reach signal pathways already in place (see this section's data-ops layer above and the LUT topology view modules); the missing layer is the *interpretation*, the rule that says "this committed counterparty has stopped fulfilling for N rounds — emit a `compute-breach` against the underlying Commitment." This rule belongs in the elohim-operator's discernment layer (see Part VIII), bounded by the substrate-floor / elohim-ceiling pattern: substrate detects the observable fact (silence, slowness, exhaustion); the operator decides whether to escalate to breach versus to defer, witness, sponsor, or renegotiate.

The recovery agreements described throughout this chapter are NOT blocked on this gap — they operate at the REA Commitment + FeedbackSignal layer, which is LIVE and queryable through `ReaCommitmentView`. The compute-commitment substrate-floor scenarios in `a2o/features/deployment/compute-commitment-bounds.feature` are the ones blocked; they are `@wip` today and unblock when the substrate-native node-health observable lands. *The temporary developer-substrate currently underneath the alpha cluster — k8s — is incidental to this design*: the protocol's eventual answer does not include cluster APIs (see the next section on the developer-substrate trajectory). The observable layer is gossip + libp2p connection state + EPR reach signals; the interpretation layer is the elohim-operator's discernment.

### **What the Stack Actually Composes**

What this layered walk demonstrates — without overclaiming — is that the resilience epic's primary surface is *structurally feasible* on the substrate as it exists today. The Commitment entry is LIVE. The FeedbackSignal extensibility primitive is LIVE. The storage projection is LIVE. The topology↔REA bridge through `custody-blob` / `project-blob` / `serve-blob` actions is LIVE and queried in multiple production code paths. The reciprocity view that powers the per-counterparty math is LIVE. The reach-earning gate that determines authorial-time authorization is LIVE. The signal-flow wiring is LIVE (LUG closed).

What is NOT yet live, and what the resilience surface waits for, is *vocabulary* — the specific `signal_kind` extensions (`recovery-share-custody`, `recovery-breach`, `recovery-quorum-formed`, `recovery-fulfilled`), the `resource_classified_as` classifications (`recovery`, `share-custody`, `encrypted-custody`) added to the whitelist, the role records (`role-as-recovery-counterparty`, `role-as-account-claimant`), the seed-data expression of the gertrude↔dowell Agreement, the feature files that exercise the flows. These are *vocabulary additions on a substrate that already speaks the language* — finite work, well-shaped, no architectural surprises.

The substrate is closer to the trillion-dollar test than the manifesto-tier framing alone would suggest. What it is *not* close to is a deployed-at-billions-of-households state, and that is the work the rest of this chapter exists to scope.

---

## **Part VIII: The Complexity Collapse — Elohim Operators as Substrate AI**

### **The Hyperscaler Wall, Diagnosed**

The trillion-dollar problem of consumer technology is, at its root, a **complexity-management problem**. Hyperscalers did not win because their fundamental architecture was superior — peer-to-peer distribution of bytes has been operationally credible for two decades; mutual aid as a social pattern is millennia old. They won because they collapsed the *operational complexity* of running consumer-scale digital infrastructure into clean abstractions that ordinary developers, and through them ordinary humans, could build against. Pods. PersistentVolumeClaims. Deployments. Services. Ingresses. Secrets. ConfigMaps. These are not protocol-fundamental concepts. They are **complexity-collapse primitives** — clean abstractions that a small number of cloud-native operator binaries (`kubelet`, `kube-controller-manager`, `kube-scheduler`, `cloud-controller-manager`) maintain on behalf of every workload, so that the application developer does not have to think about node placement, storage attachment, network routing, restart loops, eviction policies, certificate rotation, secret injection, autoscaling, drain semantics, or any of the thousand other things that would otherwise make running consumer infrastructure impossible at human scale.

The wall around the cloud garden is built of *this complexity-management capability*. It is not built primarily of malice. Most of the careless billionaires whose accumulated capital paid for the wall's construction did not set out to extract Grandma's interiority for ad-tech revenue. They set out to solve real operational problems at consumer scale, and they solved them, and the resulting infrastructure — necessarily centralized to be operationally tractable at the time — became the substrate everything else now rents from. The wall is built of *the work that nobody else could yet do at consumer scale*: turning the chaos of clusters into clean abstractions that survive Friday-night deploys, dropped racks, exhausted disks, partial-cluster failure modes, certificate expiration, and noisy-neighbor blast radius. The wall's *result* is a centralized custodian whose revenue model extracts attention. The wall's *cause* was the complexity-management capability the centralization made possible.

This is the diagnosis that determines what the substrate has to do. **The substrate's competitive question is not "can we be more peer-to-peer than them" — it is "can we collapse equivalent complexity for ordinary humans, served by operators whose incentive geometry is aligned with the human rather than the platform."** The architecture that wins the trillion-dollar problem is the architecture that does *for P2P* what kube-controller-manager does *for k8s* — and serves the household, not the shareholder.

### **The Elohim Operator — What It Is**

Every hub in the substrate runs an **elohim-operator** — a context-bound specialist agent that fills the role a household's devops/IT person would fill if the household had one. The operator's definition lives in the iroh-libp2p complementarity spec (`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`) and is anchored by memory `project_elohim_subagent_specialists`:

> "The operator treats the hub's hardware as a cluster (whether the cluster is one recycled laptop, three NUCs, five blades, or a Tier-3 family-node-extended with hot-swap modules) and continuously negotiates: internal cluster operations (hot/cold blade migration, leader election, replica placement, storage-volume migration, blob tiering across NVMe/bulk/encrypted-shard storage); stewardship-vs-capacity tradeoffs (how much compute / bandwidth / storage / AI inference budget each Track 3 spoke commitment, each Track 2 federation contract, and each internal household need consumes; when to defer, when to spend, when to renegotiate); and external federation participation (peer-hub gossip, sponsored compute contracts sending and receiving, AbusePattern signal emission, defense-reach earning, federation-manifest declaration of which peer hubs are reachable)."

The mapping to k8s is exact in shape and inverse in posture:

| k8s primitive | What the operator collapsed | elohim-operator equivalent | Whose interest |
|---|---|---|---|
| Pod | "How does this binary survive a node failure?" | elohim-node deployment + DHT-notarized NodeRegistration | The household's |
| PersistentVolumeClaim | "Where does this state live and follow the workload?" | Stewardship Commitments on the REA ledger; tiered-quilt blob custody | The household's |
| Service / Ingress | "How do consumers reach this?" | Doorway projection routes + iroh ALPN handlers | The household's |
| ConfigMap / Secret | "How do config values flow without leaking?" | peer-policy.toml + identity-handshake handshake | The household's |
| HorizontalPodAutoscaler | "How does capacity respond to demand?" | Compute-capacity negotiation + Track 2 federation | The household's |
| Operator (e.g. cert-manager, etcd-operator) | "Domain logic for a specific workload class" | elohim-agent subagents (defender, advocate, steward, gate-discerner) | The household's |
| kube-controller-manager | "Continuous reconciliation toward declared state" | ReconcileController (Principle P1) + elohim-operator discernment | The household's |

The k8s side of this table is what the hyperscalers built to make consumer-scale infrastructure operationally tractable. The elohim side is what the substrate proposes to build to make consumer-scale infrastructure *dignifiably tractable* — same complexity collapse, opposite political geometry. The pods land in households; the volumes are stewardship contracts; the services are doorway projections; the autoscaling negotiates against federation peers rather than against a quarterly profit target.

### **DwellingHub and CollectiveHub — Two Postures of the Same Trait**

The elohim-hub crate (`elohim/elohim-hub/README.md`) names two archetypes that share a trait surface but differ in design attitude:

**DwellingHub** is the primary archetype — the home-node cluster sized to one family / one dwelling. "Dwelling" grounds the concept at its natural physical limit: the place where humans live, walls and a roof, a known set of inhabitants who steward it together. The attitude is **co-presence**. Humans and their elohim-operators are co-present in the dwelling's fabric. The fabric is visible; family members can intervene; the elohim-operator is a *fabric-helper alongside the humans*, not a fabric-owner. The Matthew/Jessica/James/Gertrude reciprocal-backup story from Part V lives in this archetype — the operator does the operational lifting (replica placement, share custody, federation contract maintenance) so the humans don't have to, but the humans dial the level of operator agency and can intervene at any layer.

**CollectiveHub** is the archetype for a collective — church, co-op, patron circle, DAO, mutual aid network. The attitude is **delegated stewardship**. The collective expects elohim-operators to carry more day-to-day fabric work autonomously, with humans designating stewardship roles rather than every member operating fabric directly. Not a hard taxonomic split from DwellingHub — both are bound by the substrate-floor / elohim-ceiling pattern below — but the *attitude* of who is most active in operation differs. This is the technical tier where the human-vs-elohim role separation is more visible, and where a small group of designated stewards (a homeschool co-op's facilitator, a congregation's deacon, a credit union's board) supervises the operator's fabric work on behalf of a larger membership.

**Hubbiness is a dial, not a binary.** The same physical device can be just-a-personal-device at 6pm and a-hub-too at 2am when it's plugged in and shared with the household. The operator's role on the device scales with the dial. Humans dial up hubbiness as comfort and capacity allow; humans hand it off as Tier-3 hardware arrives and the household-fabric-manager role migrates to a more capable operator. The *steward+device layer* gives humans the consistency of their own space; the *hub layer* gives the elohim-operator a dwelling to be opinionated and helpful at social-coordination scope without stepping on human toes. The dial is owned by the human, declared via standing manifests, signed/witnessable/reversible at every increment.

### **Substrate Floor and Elohim Ceiling — The Bounding Pattern**

The elohim-operator's authority is *structurally bounded* by the substrate-floor / elohim-ceiling pattern (memory `project_substrate_floor_elohim_ceiling`). The substrate determines deterministic gates (Allowed / Blocked / Pending) at every reach-authorization decision; the operator's discernment can *escalate* (sponsor a normally-blocked action), *witness* (record additional context), or *defer* (request human input). The operator cannot unilaterally override the substrate's gates. The substrate cannot unilaterally make the discernment calls reserved to the operator. Both layers are structurally bounded — neither is a unilateral authority.

This is the architectural answer to the alignment worry. An elohim-operator with unbounded authority over a household's substrate would be, eventually, a re-creation of the hyperscaler problem at household scale: a single concentrated capability operating on behalf of someone whose interests can drift from the household's. The substrate-floor / elohim-ceiling pattern structurally prevents this. The operator is a complexity-collapse layer; it is not a sovereignty-collapse layer. The household's authority over its own substrate remains constitutional, signed, witnessable, and reversible — by design, at every step, including against the operator's own judgment.

### **K8s as Developer Test-Bench, Not Protocol Layer**

A critical framing — and one easy to get wrong, because the analogues are so close. **Kubernetes is the development substrate the protocol's test-bench currently sits on; it is not part of the protocol.** The alpha cluster runs on k8s today because k8s is the operationally-tractable consumer-grade hardware-orchestrator available *now*, and the protocol's developers need a working substrate to develop against while the protocol's own substrate matures. Once the protocol can self-host — once `brit` (the household-cluster covenant/contract layer that replicates content via stewardship commitments) and `rakia` (the protocol's own firmament, the substrate that hosts its own development) can carry their own weight — k8s retires. The transition is named in the [substrate-as-upstream-containment](../../../../../rakia/docs/plans/2026-05-06-substrate-as-upstream-containment.md) design and the [brit migration roadmap](../../../integrations/brit-migration-roadmap.md). What remains, after k8s retires, is the *pattern* that k8s pioneered — operator-as-continuous-reconciliation, declarative-state-as-source-of-truth, controller-shape — recognized as inspirational analogues for how the elohim-operator orchestrates P2P-native compute. The mapping table above is in the chapter to make this analogue legible, not to import k8s as a substrate dependency.

### **What Is Built — and What Awaits**

The elohim-hub crate exists; per its README it is scaffold-stage. The code currently lives in `elohim-node` until a second consumer (operator UI, fixtures crate) needs the trait independently. The hub trait is sketched in `2026-05-02-elohim-hub-boundaries-design.md`, and the responsibility surface is mapped in `2026-05-08-doorway-hub-edge-design.md`. The substrate-floor / elohim-ceiling pattern is documented; the operator-as-AI-agent layer is design + intent.

The alpha cluster's *developer-substrate today is k8s*, not elohim-hub. The deployment templates render container-orchestrator manifests; the operator is *Matthew the human*, editing seed-deployment records and capacity ledgers by hand, watching dashboards, applying drift fallbacks when the orchestrator returns errors. This is the *bootstrap phase* of the operator-collapse story — a competent human carrying the operational load while the substrate that will eventually carry it for ordinary households is built. The k8s-shaped operator work Matthew does today is the *task description* for the elohim-operator-as-AI runtime that takes over: same complexity-collapse work, different runtime, eventually different developer-substrate (brit/rakia) underneath.

The transition path is the elohim-operator-as-AI runtime taking over progressively from the elohim-operator-as-Matthew, while the developer-substrate the AI operator works *against* migrates progressively from k8s toward rakia. The substrate-floor / elohim-ceiling pattern that bounds the AI operator is already documented; the elohim-agent crate exists separately and carries the specialist-subagent shapes (`project_elohim_subagent_specialists`); the bridge from elohim-hub trait to elohim-agent specialist dispatch is the next-leverage piece of substrate work. This is the substrate's answer to the *generational* framing the operator named: a generation is how long it takes for the household-operator role to migrate from competent humans doing it by hand on alpha hardware to AI agents doing it dignifiably for ordinary humans on protocol-native hardware everywhere — and for the developer-substrate underneath to migrate from rented hyperscaler-shape orchestration to the protocol's own firmament.

### **The Core Move**

The argument distilled: **the hyperscaler wall is the wall of complexity-management capability. AI is the lever that dissolves the advantage. Per-household elohim-operators do for substrate dignity what kube-controller-manager does for cluster operability — same complexity-collapse engineering, opposite political geometry.** The walled gardens of the careless billionaires were built of the work nobody else could yet do; the substrate's claim is that AI now lets us do that work, per household, in service of the household, with the operator's authority structurally bounded by the substrate floor and the household's expressed standing. The garden walls dissolve not by frontal attack but by being made *unnecessary* — by an operator layer that delivers equivalent convenience without auctioning the human.

---

## **Part IX: What Is Built, What Is Designed, What Remains**

### **The Honesty Discipline**

A chapter that anchors a trillion-dollar civic claim has to be specific about what carries weight today and what is named-but-unfinished. The matrix below is that specificity. Three columns: **LIVE** (deployed, queryable on the alpha cluster, exercised by CI), **DESIGNED** (spec exists, partial implementation, named work to close), **GAP** (not started, named-and-scoped). The matrix is not exhaustive; it pulls the load-bearing rows from the substrate research.

### **Notary Layer (Holochain DHT)**

| Capability | Status | Where |
|---|---|---|
| Agreement, Commitment, EconomicEvent, EconomicResource entry types | **LIVE** | `content_store_integrity/lib.rs:1319-1368` |
| FeedbackSignal entry + 6 bootstrap signal_kinds (squelch / correction / retraction / quarantine / vouch / forget-request) | **LIVE** | `feedback_signal.rs:42` |
| ContributorPresence entry (32 fields, 3-state lifecycle, 8 verification methods) | **LIVE** | `content_store_integrity/lib.rs:1166-1219`, `PRESENCE_STATES`, `CLAIM_VERIFICATION_METHODS` |
| 16-value resource_classified_as bootstrap whitelist (content / attention / recognition / compute / stewardship / 6 shefa tokens / etc.) | **LIVE** | `content_store_integrity/lib.rs:238-257` |
| 23 REA_ACTIONS including custody-blob, project-blob, serve-blob, deliver-service | **LIVE** | `content_store_integrity/lib.rs:202-235` |
| Recovery-class signal_kind extensions (recovery-share-custody, recovery-breach, recovery-quorum-formed, recovery-fulfilled) | **GAP** | requires whitelist edit + schema update + standing-policy debit-weight entries |
| Recovery-class resource_classified_as classifications (recovery, share-custody, encrypted-custody) | **GAP** | requires whitelist edit OR manifest-driven validator pattern |
| Typed compute-class vs attribution-class partition on Resource classification | **GAP** | currently documented discipline; not enforced at schema layer |

### **Data Ops Layer (libp2p + iroh)**

| Capability | Status | Where |
|---|---|---|
| Track 1 (kitsune2/tx5 over WebRTC) — DHT notary | **LIVE** | unchanged |
| libp2p 0.54 in elohim-storage (request-response + gossipsub + Kademlia + Circuit Relay) | **LIVE** | `elohim-storage/Cargo.toml` |
| iroh 0.92 + iroh-blobs 0.94 + iroh-gossip 0.92 for hub-to-hub federation | **LIVE** | Phase 11 closed; 43 tests |
| All 7 iroh planes ALPN-registered + backends (blob, gossip, sync, epr, epr-atom, shard, view-fed, identity-handshake, trust) | **LIVE** | Phases 1-10 |
| Dual-publish on inventory, identity-binding, recovery-invitation, recovery-revocation, feedback-signal, EPR-atom-announce | **LIVE (publish-side)** | `p2p_iroh/dual_publish/` |
| Receive-side iroh gossip subscribers wired into daemon receive handlers | **GAP** | `src/p2p/mod.rs:4396-4607` still libp2p-only |
| peer_transport_manifest schema + peer_map selection API | **LIVE (substrate)** | migration `2026-05-10-120000`, `p2p_iroh/peer_map.rs:1-150` |
| Phase 12 (peer_transport_manifest fully wired) | **DESIGNED** | 0/59 plan boxes ticked; 4 adapter wiring tasks partially landed per in-tree README |
| n0-mitigation Steps 2-5 (pkarr self-hostable resolver, full consumer-grade soak) | **GAP** | gate #9 of iroh master plan |
| Consumer-grade-hub operator runtime (Track 3 spoke registration UX) | **GAP** | aspirational |

### **Reconciliation Layer (Principle P1)**

| Capability | Status | Where |
|---|---|---|
| ReconcileController operator-controller pattern (inspired by container-orchestrator controller shapes; substrate-native observers, not cluster APIs) | **DESIGNED + skeleton** | `elohim-storage/src/reconcile/controller.rs` |
| Handlers for imagodei/M5 recovery signals (key rotation, key revocation, agent-peer-binding, revocation attestation, portal-host) | **LIVE** | `controller.rs` real handlers |
| Handlers for recovery-class signals (recovery-share-custody, recovery-breach, recovery-fulfilled) | **GAP** | not yet wired |
| Substrate-native node-health observable → REA EconomicEvent emission (peer silence / sustained unavailability / exhausted capacity → `compute-breach`) | **GAP** | no protocol-native node-health observer + breach interpreter; the load-bearing edge to close for compute-commitment-bounds.feature to pass; observable layer rides gossip + libp2p connection state + EPR reach signals, interpretation belongs in elohim-operator discernment |
| `compute-commitment-bounds.feature` scenarios passing on alpha | **GAP** | all @wip; blocked on the substrate-floor design `2026-05-04-compute-commitment-substrate-floor-design.md` not yet landed |
| ack-projection EconomicEvent + projection_events table | **LIVE** | `rea_projection.rs:178-202`, `tests/projection_ack_signal_e2e.rs` |

### **Storage Projection Layer**

| Capability | Status | Where |
|---|---|---|
| ReaCommitmentView + AgreementView (camelCase, parsed JSON, computed fields) | **LIVE** | `elohim-storage/src/views.rs:308, 1809` |
| HTTP `/db/agreements`, `/db/rea-commitments` routes | **LIVE** | `elohim-storage/src/http.rs` |
| `custody-blob` / `project-blob` / `serve-blob` action conventions queried in `reciprocity_view.rs`, `cluster_view.rs`, `device_capacity.rs`, `rea_projection.rs` | **LIVE** | as cited; the topology↔REA bridge IS live |
| Compute-commitment DB table (for "matthew commits 1000 cpu_m to alpha-cluster") | **GAP** | only `stewardship_allocations` exists (content-stewardship, NOT compute) |

### **Topology Layer (Light Up the Topology)**

| Capability | Status | Where |
|---|---|---|
| Light Up the Graph (LUG) — signal-flow wiring at six sites; Vouch primitive end-to-end as signal_kind extension | **LIVE** | commit `4ea4e1558`; 21/27 tasks closed |
| Light Up the Topology (LUT) — five view modules (`distribution_view`, `cluster_view`, `peer_topology_view`, `reciprocity_view`, `doorway_dashboard_view`) | **DESIGNED + partial** | service modules exist; M3 TODOs at `peer_topology_view.rs:332,394,405,491,544` (per-peer authored count, batch GROUP BY, last_sync_sec, resilience_cliffs stub) |
| LUT M1 substrate-completion sprint (cross-household matthew↔terrance vertical slice) | **DESIGNED** | `2026-05-07-topology-substrate-completion-m1-plan.md`; unchecked at plan-write |
| Resilience-cliff detection + presentation | **GAP** | `resilience_cliffs` returns `vec![]` |

### **Reach + Trust + Standing Layer**

| Capability | Status | Where |
|---|---|---|
| 8-value reach schema enum (private / self / intimate / trusted / familiar / community / public / commons) | **LIVE** | `elohim/sdk/schemas/v1/enums/reach.schema.json` |
| Reach gating across validator, doorway, and receive-side preference guards | **LIVE** | `content_store_integrity/lib.rs:514`, `doorway/.../cache/reach_aware_serving.rs`, `steward/node/.../storage/reach.rs`, `elohim-storage/.../p2p/reach_authorization.rs` |
| reach_earning service (5 floor classes + standing) with `ReachVerdict::{Allowed, Blocked, Pending}` | **LIVE** | `elohim-storage/src/services/reach_earning.rs` |
| Standing as derived view, 5 ordinal levels (Floor / Low / Neutral / High / Trusted), evaluator-local, not global | **LIVE** | `services/standing.rs`, `Standing::evaluate`, `StandingScore` |
| Standing-policy manifest (debit weights, thresholds, floor classes) | **LIVE** | `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json` |
| Reach enum drift — Rust `Reach::{Personal, Intimate, Household, Neighborhood, Collective, Community, District, Public}` divergent from schema enum | **GAP** | reconciliation needed; resilience epic uses third vocabulary (`household / neighborhood / community / organization / commons`) |
| Standing composes from imagodei profile + lamad recognition + FeedbackSignal debits | **DESIGNED** | schema seam exists (`unknownTreatment.evidenceSources`), bridges not implemented |
| 7-layer trust-compute gradient modulation (gossipsub, Kad provider records, schemaRef walks, projection caching, validation amortization, cold-fetch peer selection) | **DESIGNED** | `2026-04-30-trust-compute-gradient-brainstorm.md`; primitives live, gradient mostly unwired |

### **Attribution + ContributorPresence Layer**

| Capability | Status | Where |
|---|---|---|
| ContributorPresence accumulation pipeline (affinity-weighted recognition flow during stewardship) | **LIVE** | `services/recognition_pipeline_service.rs` |
| Storage projection CRUD (`CreateContributorPresenceInput`, `InitiateStewardshipInput`, `InitiateClaimInput`, `RecognitionUpdate`) | **LIVE** | `elohim-storage/src/db/contributor_presences.rs` |
| 3-state lifecycle (unclaimed → stewarded → claimed) with state-transition Events (`presence-claim`, `recognition-transfer` LamadEventTypes) | **LIVE** | `content_store_integrity/lib.rs:284-285` |
| Recognition transfer on claim (lump-sum EconomicEvent on agent claim) | **DESIGNED** | fields reserved on entry (`claim_recognition_transferred_value`, `claim_recognition_transferred_unit`); no `transfer_recognition()` coordinator function |

### **Resilience-Specific Surface**

| Capability | Status | Where |
|---|---|---|
| Three canonical recovery stories (Gertrude holds share / Dowells hold Gertrude's share / Gertrude logs in with help) | **LIVE** | `genesis/data/stories/gertrude-grandma--*--*.md`, `genesis/data/stories/matthew-manager--*--*.md`; status:draft |
| Resilience epic chapter (this document) | **LIVE** | this file |
| Gertrude + James deployments on alpha cluster | **LIVE** | deployments.json commits `7ebbeb8da`, `64f5e1b84` |
| `human-resilience.feature` scenarios (Matthew alone → at-risk; +Susan → partial; +Pete → community depth) | **DESIGNED** | scenarios @wip; resilience profile computation against mutual-aid commitments |
| `feature-social-recovery-with-help-from-family.feature` | **GAP** | highest-leverage; named in Part IV; no Gherkin on disk |
| `feature-backup-stewardship-for-household-{dowell,gertrude}.feature` reciprocal pair | **GAP** | named in Part IV; no Gherkin on disk |
| `role-as-recovery-counterparty`, `role-as-account-claimant` role records | **GAP** | named by storyteller; not in `genesis/data/lamad/content/` |
| Seed-data expression of gertrude↔dowell Agreement on the REA ledger | **GAP** | shape open: seed corpus, schema decision |
| Storage-stewardship summary route (`GET /storage-stewardship/summary` returning 3-bucket breakdown) | **GAP** | doorway-manifest declaration needed |
| Angular widget in shefa pillar (top-level bar with drill-down) | **GAP** | UI work; awaits LUT M1 |

### **Distribution / Succession / T&S — Civic Substrate Tests**

| Capability | Status | Where |
|---|---|---|
| CustodianCommitment entry type (4 commitment types, 6 selection bases, 3 shard strategies, 5 emergency-trigger types) | **LIVE** | `content_store_integrity/lib.rs:3289` |
| CustodianCommitment coordinator functions (create, accept, etc.) + wire types | **LIVE** | `content_store/lib.rs:8391+`, shefa-types |
| `steward_affinity` table + Stage 2 pipeline integration (affinity-weighted recognition) | **LIVE** | `recognition_pipeline_service.rs`, `2026-03-14-steward-affinity-lifecycle-design.md` |
| Mastery gate + curation-event endpoint + affinity deltas | **DESIGNED** | Increment 2 of steward-affinity-lifecycle plan |
| Stage 4 constitutional limits (floor/ceiling/excess redistribution) | **DESIGNED** | Increment 3 of steward-affinity-lifecycle plan |
| Peer-mesh delivery (peers serve peers; doorway as one source among many; LAN-preferring) | **DESIGNED + scenarios** | `peer-mesh.feature` @wip |
| Web2-absorption projection cache (browser traffic absorbed before reaching storage) | **DESIGNED + scenarios** | `web2-absorption.feature` |
| Protocol omnibar (provenance pill, EPR-address display, drill-down) | **DESIGNED + scenarios** | `protocol-omnibar.feature`, `2026-04-03-content-delivery-toolbar-sprint2-plan.md` |
| Content-addressing delivery + content-delivery transport-perf scenarios | **DESIGNED + scenarios** | `content-addressing.feature`, `transport-perf.feature`, `spa-bundle-delivery.feature`, `delivery-diagnostics.feature` |
| FANG-subsumption (YouTube/Netflix-class distribution by federated reach-earning, not centralization) | **DESIGNED** | `2026-05-08-doorway-hub-edge-design.md` distribution-reach surface |
| `signal_kind: "impersonation-claim"` validator whitelist | **GAP** | not in `SIGNAL_KINDS`; needed for T&S substrate floor |
| `feature-account-takeover-recovery.feature` — the Sheila scenario end-to-end | **GAP** | no Gherkin; highest-leverage T&S test the substrate can prove |
| `feature-creator-succession.feature` — ContributorPresence inheritance via graduated-authority | **GAP** | no Gherkin |
| `feature-patron-cdn-discovery.feature` — content resolves to nearest patron-edge via reach-walked discovery | **GAP** | no Gherkin |
| Storage-stewardship summary commons-tier drill-down showing per-creator patron-CDN composition | **GAP** | depends on storage-stewardship-summary route (already in Resilience-Specific table) |
| Canonical stories: creator-under-impersonation-attack / commons-custodian-for-creator / inheritor-of-contributor-presence | **GAP** | no stories under `genesis/data/stories/` |

### **Hub + Operator Layer**

| Capability | Status | Where |
|---|---|---|
| elohim-hub crate (scaffold) | **LIVE (scaffold)** | `elohim/elohim-hub/README.md` |
| DwellingHub / CollectiveHub trait sketch | **DESIGNED** | `2026-05-02-elohim-hub-boundaries-design.md`, `2026-05-08-doorway-hub-edge-design.md` |
| Substrate-floor / elohim-ceiling pattern | **DESIGNED + partial** | doctrine in memory `project_substrate_floor_elohim_ceiling`; substrate floor is real (reach_earning), ceiling discernment integration with elohim-agent crate pending |
| elohim-agent subagent specialists (defender, advocate, steward, gate-discerner) | **DESIGNED** | memory `project_elohim_subagent_specialists` |
| Bridge from elohim-hub trait to elohim-agent specialist dispatch | **GAP** | the elohim-operator-as-AI-runtime layer; next-leverage substrate work |
| Hubbiness dial in standing manifests | **DESIGNED** | concept clear; manifest schema additions pending |

### **The Roadmap, Compressed**

The matrix above names every load-bearing edge. The work in rough order of leverage (for the resilience-epic surface specifically):

1. `feature-social-recovery-with-help-from-family.feature` — the highest-leverage Gherkin; grandma-standard end-to-end recovery; no executable spec today.
2. `feature-account-takeover-recovery.feature` — the Sheila scenario as Gherkin; impersonation rejection at the reach-gate + community-quorum revocation; highest-leverage T&S test the substrate can prove.
3. Recovery-class `signal_kind` extensions (`recovery-share-custody`, `recovery-breach`, `recovery-quorum-formed`, `recovery-fulfilled`, `impersonation-claim`) into the integrity-zome whitelist + schema + standing-policy-floor manifest.
4. `recovery` + `share-custody` + `encrypted-custody` `resource_classified_as` classifications added (or the validator pattern updated to accept manifest-declared classifications).
5. `role-as-recovery-counterparty`, `role-as-account-claimant`, `role-as-creator-under-impersonation-attack`, `role-as-commons-custodian-for-creator`, `role-as-inheritor-of-contributor-presence` role records in `genesis/data/lamad/content/`.
6. Reciprocal-backup feature pair: `feature-backup-stewardship-for-household-{dowell,gertrude}.feature`.
7. `feature-creator-succession.feature` + `feature-patron-cdn-discovery.feature` — the civic-substrate companion scenarios to the recovery surface.
8. Seed-data expression of the gertrude↔dowell Agreement (shape: open; story-first decides).
9. Recovery-class signal handlers in `ReconcileController`.
10. Storage-stewardship summary HTTP route + Angular widget, including commons-tier drill-down with per-creator patron-CDN composition.
11. Recognition transfer on claim — the missing executor for the absent-contributor flow (also load-bearing for creator succession).
12. (Substrate-wide, not just resilience-surface) Substrate-native node-health observable → REA EconomicEvent edge — closes the compute-commitment-bounds.feature gap and the broader substrate-floor / elohim-ceiling claim. The observer rides protocol-native signals (gossip + libp2p connection state + EPR reach), not container-orchestrator APIs; the developer-substrate the alpha cluster currently runs on (k8s) is incidental and retires when brit/rakia mature.
13. (Substrate-wide) Reach enum reconciliation — Rust vs schema vs resilience-epic vocabularies.
14. (Substrate-wide) Trust-compute gradient layered modulation per the 707-line brainstorm.
15. (Substrate-wide) Bridge from elohim-hub trait to elohim-agent specialist dispatch — the elohim-operator-as-AI runtime.

Numbers 1–11 are *resilience-epic-scoped* (recovery surface + patron-CDN civic surface). Numbers 12–15 are the substrate-wide foundational work the trillion-dollar claim depends on, even when not directly resilience-surface. They are named here because the chapter is responsible for naming them.

---

## **Closing**

The architecture of consumer technology asks Grandma to trade her humanity for convenience. The architecture of an extractive economy asks her to do this continuously, in fractions, without ever quite noticing. The architecture of a centrally custodial AI future will ask her to surrender judgment about her own life to a system whose revenue depends on getting that judgment subtly wrong.

The protocol's commitment is that none of these trades is actually necessary. The convenience is achievable — the engineering is hard but tractable. The dignity is preservable — the substrate just has to be reciprocal rather than extractive. The AI is alignable — the substrate alignment is what makes the AI alignment possible.

What makes this commitment more than rhetoric is that it is *testable*. Not in twenty years. Not at scale. Now, on the alpha cluster, against Gertrude and Matthew and Jessica and James, with one reciprocal-backup pair, three canonical stories, a finite list of feature files to write, and an REA primitive that already exists. The grandma standard is a Gherkin scenario waiting to be authored. The trillion-dollar problem is, structurally, four minutes of recovery flow, executed correctly, with help from her people, on a substrate that doesn't betray her.

What this chapter has tried to demonstrate — walking through the patron-enabled CDN in Part VI, the substrate's actual layers in Part VII, the elohim-operator complexity-collapse in Part VIII, and the honest gap matrix in Part IX — is that the trillion-dollar civic claim is currently load-bearing at every layer where it has shipped, and named-with-scope at every layer where it has not. The work is finite. The substrate stack composes. The notary layer is in production; the data-ops layer is in permanent dual-stack with Phase 11 closed and Phase 12 in flight; the reconciliation controller carries imagodei recovery signals today and gets recovery-class signals next; the storage projection is live; the topology↔REA bridge is queried in four production code paths; the reach gate and standing evaluator run on every reach decision; the ContributorPresence entry runs in production with 32 fields and the transfer-on-claim machinery reserved. The elohim-operator runtime that will eventually carry the operational load for ordinary households is scaffold-stage today, with Matthew the human carrying that role on the alpha cluster while the protocol's eventual answer takes form. The transition is *the generational work* — the migration of the household-operator role from competent humans doing it by hand on alpha hardware to elohim-operator AI agents doing it dignifiably for ordinary humans on consumer hardware everywhere.

The other epics in this corpus — learning, governance, economy, identity — rest on this substrate. Lamad's learning recognition, Qahal's governance participation, Shefa's value flows, Imagodei's identity continuity all require a substrate that survives device loss, household failure, contributor absence, hardware decommission, account hijack, creator death, and the slow erosion of trust that takes centralized custodians down one century at a time. This chapter is therefore not one chapter among the others; it is the chapter that says what the others are scaling and delivering against. Resilience is not a feature added to the other epics — it is the substrate on which the other epics are themselves possible. The trillion-dollar civic claim is what makes their claims composable across decades and across households, what protects creators when their credentials are stolen, what carries patrons' contributions through the substrate as recognition rather than ad inventory, what passes a creator's work to a successor when the creator is gone.

That is the work. That is the entire work. Mutual aid as substrate, recovery as the test, the patron-enabled CDN as the civic distribution layer, the elohim-operator as the complexity collapse, Gertrude and Sheila as the two witnesses — one for the recovery surface, one for the impersonation-resistance surface, both proving the same substrate from opposite vantages.

---

*Related chapters: [Manifesto](../manifesto.md) · [Constitution](../constitution.md) · [Protocol Specification](../protocol-specification.md) · [Shefa Economic Infrastructure](../../../Shefa_Economic_Infrastructure_Whitepaper.md) · [Grandparent (Value Scanner persona)](../value_scanner/grandparent/README.md)*

*Related stories: [Gertrude Holds the Share](../../../../data/stories/gertrude-grandma--as-recovery-counterparty--backup-stewardship-for-household-dowell.md) · [The Dowells Hold Gertrude's Share](../../../../data/stories/matthew-manager--as-recovery-counterparty--backup-stewardship-for-household-gertrude.md) · [Gertrude Logs In with Help from Her People](../../../../data/stories/gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family.md)*

*Related testbeds: [`a2o/features/shefa/human-resilience.feature`](../../../../a2o/features/shefa/human-resilience.feature) · [`a2o/features/deployment/compute-commitment-bounds.feature`](../../../../a2o/features/deployment/compute-commitment-bounds.feature) · [`a2o/features/elohim/compute-allocation.feature`](../../../../a2o/features/elohim/compute-allocation.feature)*

*Related memory: [`project_compute_commitments_bounded`](../../../../../.claude/memory/project_compute_commitments_bounded.md) · [`project_recovery_grandma_standard`](../../../../../.claude/memory/project_recovery_grandma_standard.md) · [`project_socially_derived_security`](../../../../../.claude/memory/project_socially_derived_security.md) · [`project_graduated_recovery_authority`](../../../../../.claude/memory/project_graduated_recovery_authority.md) · [`project_elohim_as_counsel`](../../../../../.claude/memory/project_elohim_as_counsel.md) · [`project_household_is_resilience_unit`](../../../../../.claude/memory/project_household_is_resilience_unit.md) · [`project_collapse_bureaucracy_into_protocol`](../../../../../.claude/memory/project_collapse_bureaucracy_into_protocol.md)*
