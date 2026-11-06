**REFUTED — R2 contradicts the explicit Intimate exemption and its own device-counting rule, while leaving recoverability, repair feasibility, and mixed-version enforcement insufficiently defined.**

1. **High — Intimate custody contradicts both the governing reconciliation and R2 itself.**

   **Claim attacked:** Intimate requires `k≥1,d≥1`, and “the person’s second device counts,” while R2.4 excludes the publisher’s own devices.

   **Evidence:** [Plan §7.2, lines 251–257](/projects/elohim/genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md:251) explicitly says that at self and intimate, custody is a local pledge, there are zero DHT heads, and **the only gate is CID-verified bytes**. [D6, lines 559–571](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:559) preserves that head budget. Zero heads alone does not prohibit local replication evidence; the plan’s explicit exemption defeats the mandatory count.

   **Failure scenario:** One person has two healthy workspaces and no external custodians. R2.3 admits their Intimate channel; R2.4 makes it permanently unelectable.

   **Fix:** Remove the Intimate election floor. Show sibling-copy resilience locally without making it a prerequisite or notarizing it.

2. **High — Adding `custodyFloor` without a compatibility transition violates D7.**

   **Claim attacked:** Enforcement can be introduced as “new fields” in `adoptionDiscipline`.

   **Evidence:** The [release schema, line 6](/projects/elohim/elohim/rakia/schemas/v1/release-manifest.schema.json:6) explicitly requires consumers to tolerate unknown properties; its [current discipline, lines 371–410](/projects/elohim/elohim/rakia/schemas/v1/release-manifest.schema.json:371) has no custody floor. Conversely, [D7, lines 415–424](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:415) says any field carrying restrictions changes the epoch; unknown epochs are retained but absent for decisions.

   **Failure scenario:** A newer release requires five custodians. An older reader accepts the open manifest while ignoring `custodyFloor`, producing incompatible admission decisions.

   **Fix:** Specify the policy epoch, enforcement boundary, and staged upgrade before activating the restriction. Artifact-supported wire epochs are not automatically the epoch governing interpretation of custody policy.

3. **Medium — The code supports the cited vocabulary and resolver, but not the proposed evidentiary strength.**

   **Claim attacked:** Existing infrastructure makes authenticated, fresh custody counts a straightforward extension.

   **Evidence:** The rung list is correct in [reach.rs:18–35](/projects/elohim/elohim/epr/src/reach.rs:18). The resolver exists, but [hub_resolver.rs:107–128](/projects/elohim/elohim/elohim-storage/src/services/hub_resolver.rs:107) can return no hub and joins through an identity projection; it does not prove physical independence. [custody.rs:507–522](/projects/elohim/elohim/elohim-storage/src/reconcile/custody.rs:507) counts fresh inventory rows without requiring fetch evidence. [gossip_dispatch.rs:410–436](/projects/elohim/elohim/elohim-storage/src/p2p/gossip_dispatch.rs:410) applies the payload’s holder and timestamp after structural verification.

   R3 correctly makes holder binding a prerequisite; R2 should not imply this is already implemented. Even after binding, an authenticated recent claim plus an arbitrarily old successful fetch proves neither present retention nor future service.

   **Fix:** Define evidence age, observation time, replay handling, withdrawal, and which authenticated holder actually supplied each verified object. Preserve `unknown` separately from confirmed loss.

4. **High — Neither the numerical ladder nor its RS interpretation establishes a recovery guarantee.**

   **Claim attacked:** The `1/2/3/5` custodian ladder and `1/2/2/3` domain ladder are defensible protocol defaults.

   **Evidence:** Reach describes openness, not a measured durability class. The supplied evidence gives no failure model deriving these numbers. [sharding.rs:140–148](/projects/elohim/elohim/elohim-storage/src/sharding.rs:140) confirms RS 4+3 defaults; [lines 386–395](/projects/elohim/elohim/elohim-storage/src/sharding.rs:386) require four distinct available shards. Three “custodians” is insufficient if each holds one shard, but potentially excessive if each holds every chunk.

   The [swarm specification, lines 141–147](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:141) explicitly distinguishes composite custody from shard availability. Phase 3 also declares per-shard salvage a follow-on, not an existing guarantee.

   **Fix:** Declare the tolerated failures and prove reconstructibility after them. Separate replica counts, distinct shard indices, and domain placement. Specify the coding unit: applying today’s 64-MiB threshold independently to tier chunks capped at 1 MiB would never select RS.

5. **High — Dwelling hubs are useful declarations, not sufficient failure domains.**

   **Claim attacked:** Distinct dwelling-hub identifiers establish independent failure domains.

   **Evidence:** [Phase 3, lines 127–135](/projects/elohim/genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:127) deliberately leaves placement extensible across household, capacity, standing, geography, and governance. The resolver establishes a social association; it does not establish power, site, provider, or administrative independence.

   **Failure scenario:** Three declared households host all their devices in one rack. Commons passes `d=3`; one rack failure removes every counted copy. Conversely, geographically independent devices belonging to one household are collapsed into one domain.

   **Fix:** Retain household as one declared dimension, with provenance and explicit unknowns. Count the correlated failures the policy actually promises to survive. Do not infer independence from identifiers or silently substitute peer keys when household resolution fails. Keep this distinct from D4’s independent-principal rule for build witnesses.

6. **High — The repair deadline is unsupported, and consent makes unconditional liveness impossible.**

   **Claim attacked:** `freshnessSecs=600` composes with unconditional repair within 900 seconds.

   **Evidence:** [Phase 2, lines 308–323](/projects/elohim/genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md:308) declares ten kicks per peer per minute, 600-second freshness, a 120-second sweep, and an 1800-second repeated-gap cooldown. Under one-kick-per-chunk scheduling, a 454-MiB artifact needs 454 kicks to refill one complete custodian: roughly 45 minutes of budget, excluding transfer and queueing. These are declared limits, not proof that every current execution path enforces them.

   The 600-second horizon can add detection delay; the cooldown can suppress repeated notifications but must not suppress repair scheduling. Neither establishes a completion bound. [Phase 3:116–119](/projects/elohim/genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:116) makes participation voluntary.

   **Fix:** Make repair time a measured objective with explicit start conditions, capacity assumptions, queue budgets, and deadline failure. A wider channel with no consenting custodians must remain visibly pending, potentially indefinitely; local work must continue.

7. **Medium — Generational retention and control-plane costs remain unspecified.**

   **Claim attacked:** Per-chunk custody is a sufficient durable policy shape.

   **Evidence:** [Device-footprint:56–58](/projects/elohim/genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md:56) specifies bounded release windows; [lines 276–283](/projects/elohim/genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md:276) moves carrying commitments toward one per hub per channel. R2 does not mandate per-chunk notarization, but leaves that expensive interpretation open.

   Illustratively, 1,000 unique 454-chunk releases annually, five custodians, and ten years produce 22.7 million chunk–holder associations per channel. Notarizing or indefinitely repairing each association would require later aggregation, retention migration, and historical-proof reconciliation across DNA moves.

   **Fix:** Bind commitments to bounded channel windows and immutable closure roots; keep availability observations operational. Define predecessor evidence and identity continuity across migrations. Explicitly reconcile D6’s release-object custody with the footprint specification before choosing notarization granularity.

### The ruling I would record instead

**R2 — custody admission for wider release channels**

Private, self, and intimate operation uses local custody pledges and creates zero DHT heads. Local put, build, reuse, and channel operation require no external replica quorum. Additional device copies provide visible resilience evidence; they do not establish independent build witnesses.

At trusted reach and above, a release channel may require custody evidence before its standing-authorized election. The requirement is a versioned constitutional policy, separate from build correctness, graduation, and byte-access authority. `adoptionDiscipline` is the correct policy home. Restrictions require D7-compatible epoch handling; an unsupported policy cannot authorize an election.

The policy identifies the complete manifest closure, coding parameters, tolerated failures, accepted evidence ages, and retention window. Admission demonstrates that every required object remains reconstructible under those failures. Whole-copy custodians and RS shard holders are counted differently. The proposed numerical ladder remains an experimental profile until failure and capacity measurements justify it.

Publisher-controlled devices do not satisfy a requirement for external custody. Household is one declared failure dimension; declarations carry provenance, and unresolved or correlated placement cannot be advertised as proven independence.

Count only authorized commitments combined with authenticated, replay-resistant availability evidence and bounded-age verification attributable to the actual serving holder. Evaluate evidence against a declared observation window. Resolve election authority through the receiving peer’s conductor; inventory remains operational evidence. Custody never grants reading, replication, or disclosure authority, and subject-standing restrictions remain binding.

Missing consent, capacity, identity bindings, or evidence produces a durable pending-election reason. It does not block local work, conscript peers, or silently weaken policy. After election, loss of evidence produces `unknown` or `under-custodied` as appropriate; it does not itself rewrite the elected head.

Repair respects consent, capacity ceilings, withdrawal, and retention obligations. Six hundred seconds may remain an initial inventory horizon; 900 seconds is a testable objective, not an unconditional promise. Report missed deadlines and their causes.

Commitments use bounded closure or channel-window scopes; freshness observations do not create per-chunk DHT heads. Migrations preserve referenced evidence and refuse unsupported semantics.

This alternative costs more policy definition and measurement now, but avoids embedding accidental device counts, scheduling assumptions, and household identifiers as permanent durability guarantees.
