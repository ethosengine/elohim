# Stories — Index

Storyteller-maintained catalog of canonical narratives. See `CONVENTIONS.md` for
the schema and `.claude/agents/storyteller.md` for the agent that maintains this
file.

Every story has a triple identity: **(subject, role, feature)**. Each row below
includes that triple when listing a story. Filenames encode the triple via
`<subject>--<role>--<feature>.md`.

Stories below are listed by `status` (author-axis) with `delivery_status` (substrate-axis) shown on each row. The two axes are orthogonal — see `CONVENTIONS.md` "Two orthogonal axes" section.

- **canonical** — operator-confirmed; safe graduation target for memory entries (gated by delivery-axis).
- **draft** — storyteller is composing; not yet a graduation target.
- **retired** — superseded but preserved; carries history.

`delivery_status` values: `undelivered` < `envisioned` < `backlog` < `refined` < `wip` < `active.alpha` < `active.beta` < `active.latest-stable` < `stable`, with `regression` as orthogonal-sideways. Only `/deliver` mints `active.*`/`stable`/`regression`; everything else is upstream-authoring substrate.

---

## By theme

### stewardship
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — `(human-james-son, role-as-stewardee, stewarded-device-sync)`


### ceremonial-ux
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)*
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)*
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)*
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)*

### graduated-authority
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — intimate-circle quorum (layer 1) restores Gertrude on a new device without ever surfacing higher layers.

### community-attestation
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — `(human-terrance-tutor, role-as-collective-steward, collective-governance)`

### household-fabric
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — extends household-as-trust-boundary laterally: counterparty contracts are household-to-household, not human-to-human.

### ungrudging-service
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — the share sits in the kitchen-drawer NUC for three years without needing Gertrude's attention; presence is the consent, not vigilance.
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — the protocol restores her photos and steps back; no upsell, no marketing follow-up.

### governance
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — six homeschool families rank a history curriculum by ranked-choice; the elohim casts proxy and justifies in plain language.

### ranked-choice
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)*

### collective-decision
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)*

### elohim-justification
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)*

### recovery
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — Gertrude accepts share-custody for the Dowell family; the technical primitive is invisible, the relational primitive is foregrounded.
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — the reciprocal direction; the elohim names the moment the relationship could be contaminated by accounting.
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — new phone, three of five share-holders, four minutes; the grandma-standard met.

### grandma-standard
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — accept-share ceremony in large-letter language; no jargon, no key material shown.
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — the bar from `project_recovery_grandma_standard` instantiated end-to-end.

### socially-derived-security
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)*
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)*
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)*

### reciprocal-backup
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — Gertrude → Dowell direction.
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — Dowell → Gertrude direction; same shape, reciprocal vantage.

### non-transactional
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — the elohim refuses the ledger frame on the household's behalf; reciprocity is co-incidence, not debt.

### elohim-as-counsel
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — the elohim acts as counsel for the *relationship*, not for either party — naming the contamination before it can land.

### ambient-notifications
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — small green light; no modal; *there is no hurry; it can wait until after lunch.*
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — share-holders receive one gentle ask; no daily nudges; the closing flow surfaces as a quiet green light.

### no-customer-support
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — recovery routes through "your people," not a corporate help desk.

---

## By subject

### human-james-son (James)
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — protagonist; the stewarded child opening a spoke into the family ring.

### human-terrance-tutor (Terrance)
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — protagonist; the facilitator whose curriculum disposition weights the coop's ranked-choice tally and whose attestation history the elohim references in proxy votes.

### human-gertrude-grandma (Gertrude)
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — protagonist; the grandmother who accepts share-custody for the Dowell household; large-text, simple-navigation, "I just keep this safe for the kids."
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — protagonist; the grandmother recovered onto a new phone via three of her five share-holders; the grandma-standard met end-to-end.

### human-matthew-manager (Matthew)
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — protagonist; accepts share-custody from Gertrude for her household; the elohim names the moment the relationship could be contaminated by accounting and refuses it on his behalf.
  *(previously listed as appears-but-not-subject for james-and-the-spoke / the-coop-decides — now subject of his own story.)*

### Additional characters (not the subject of their own story yet)

The following humans appear in stories but are not yet
subjects of their own canonical stories — listed as coverage gaps below.

- **human-jessica-spouse** (Jessica) — the steward in james-and-the-spoke; the proposer in the-coop-decides; coop member at Valley Homeschool Co-op; the co-signer (and the one who actually taps Accept) in the-dowells-hold-gertrudes-share.

---

## By role

### @as-stewardee
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

> Role record `role-as-stewardee` **does not yet exist** in
> `genesis/data/lamad/content/`. Closest extant role is
> `role-social-medium-child.json` (ages 8-17, social_medium pillar). The
> stewardee framing crosscuts social_medium / governance / educational — a
> dedicated `role-as-stewardee` would be a first-class addition. Surfaced as a
> coverage gap; operator decision.

### @as-collective-steward
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)*

> Role record `role-as-collective-steward` **does not yet exist** in
> `genesis/data/lamad/content/`. Distinct from `@as-stewardee` (an individual
> being stewarded) and from `@as-steward` (the steward of an individual ward):
> a collective-steward stewards a *collective decision process* — facilitator-
> as-quorum-keeper. Crosscuts qahal / educational / affinity governance.
> Surfaced as a coverage gap; operator decision.

### @as-recovery-counterparty
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — Gertrude → Dowell direction.
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — Dowell → Gertrude direction.

> Role record `role-as-recovery-counterparty` **does not yet exist** in
> `genesis/data/lamad/content/`. Distinct from:
> - `role-social-medium-elder.json` (an archetype, not a role in the
>   counterparty sense — Gertrude's elder archetype is upstream of the
>   counterparty role she occupies here).
> - `@as-stewardee` / `@as-steward` (a steward acts FOR a ward; a recovery-
>   counterparty acts WITH a peer — peer-to-peer reciprocal, not
>   asymmetric-protective).
> - `@as-account-claimant` (the inverse direction; holding for someone is
>   distinct from claiming for oneself).
>
> Crosscuts imagodei (identity continuity), shefa (reciprocal-flow), qahal
> (graduated authority), social_medium/elder (dignity-preserving recovery).
> Useful well beyond Gertrude — every share-holder in every recovery setup
> occupies this role. Two stories now flag the gap. **Cartographer-rank: high.**

### @as-account-claimant
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)*

> Role record `role-as-account-claimant` **does not yet exist** in
> `genesis/data/lamad/content/`. Distinct from:
> - `role-social-medium-elder.json` (archetype, not role).
> - `@as-stewardee` (claimant asks for herself; stewardee is asked-on-behalf-of).
> - `@as-recovery-counterparty` (the inverse direction — holding vs claiming).
>
> Crosscuts imagodei (identity), social_medium/elder (dignity-preserving recovery),
> family/elder (constitutional standing). The role is universal across recoveries —
> any human asking for recovery is in this role at the moment of asking, regardless
> of archetype. **Cartographer-rank: high.**

---

## By feature

### stewarded-device-sync
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

### collective-governance
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — canonical feature exists at `genesis/a2o/features/qahal/collective-governance.feature` (one of the few stories whose canonical feature is already on disk). Anchors specifically to scenarios: "Community uses ranked-choice to pick a curriculum path" (lines 40-58), "Elohim builds governance disposition from voting history" (lines 240-247), and "Elohim votes as proxy when human hasn't engaged" (lines 249-256). Substrate-axis remains `undelivered` until `/deliver` mints a tier-3 verdict.

### backup-stewardship-for-household-dowell
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)*

> Feature file `genesis/a2o/features/.../backup-stewardship-for-household-dowell.feature`
> **does not yet exist**. Adjacent features the story touches:
>
> - `auth/recovery/recovery-shamir-optional.feature` (exists; Matthew-as-subject, treats share-recipient as substrate fixture)
> - `auth/recovery/recovery-m5-vote-as-emergency-contact.feature` (exists; the approval-card UI pattern)
>
> The canonical share-holder-as-subject feature has not been authored. The
> companion story for the Dowell-side direction
> (`backup-stewardship-for-household-gertrude`) flags the same gap with the
> roles inverted. **Cartographer-rank: high** — three stories in this batch
> all flag recovery-feature gaps; see coverage gaps below for the consolidated
> NEEDS-NEW-FEATURE entries.

### backup-stewardship-for-household-gertrude
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)*

> Feature file `genesis/a2o/features/.../backup-stewardship-for-household-gertrude.feature`
> **does not yet exist**. Same adjacent-feature set as the reciprocal story.
> The reciprocal pair (this + gertrude-holds-the-share) form the minimum
> bilateral counterparty shape that `recovery-shamir-optional.feature` treats
> as substrate-given. Authoring either feature would inform the other.

### social-recovery-with-help-from-family
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)*

> Feature file `genesis/a2o/features/.../social-recovery-with-help-from-family.feature`
> **does not yet exist**. The closest substrate-side coverage:
>
> - `auth/recovery/recovery-m5-lost-key-entry.feature` (exists; entry-point routing)
> - `auth/recovery/recovery-m5-vote-as-emergency-contact.feature` (exists; the share-holder side)
> - `auth/recovery/recovery-shamir-optional.feature` (exists; share-custody substrate; all scenarios @wip)
> - `auth/recovery/recovery-m5-portal-host-discovery.feature` (exists; doorway discovery)
>
> The grandma-standard UX invariant — *no jargon shown to the claimant; no
> seed phrases ever; ambient notifications throughout; the elohim translating
> between substrate and human end-to-end* — is not load-bearingly tested by
> any of the above. This canonical feature would be the test for the entire
> `project_recovery_grandma_standard` memory. **Cartographer-rank: highest**;
> the story's primary delivery-debt is on this feature, and the load-bearing
> recovery-grandma-standard memory currently has NO Gherkin coverage at all.

> Feature file `genesis/a2o/features/.../stewarded-device-sync.feature`
> **does not yet exist**. Adjacent features the story touches (declared in
> `adjacent_features`):
>
> - `lamad/learning-journey.feature` (exists)
> - `lamad/path-adaptation.feature` (exists)
> - `lamad/assessment-completion-feedback.feature` (exists)
> - `content/stewardship-allocation.feature` (exists; affinity-based allocation, not the spoke flow)
>
> The canonical feature that proves this story's experience is real has not
> been authored. The earlier memory `project_stewarded_child_identity`
> proposes `humans-stewarded-child.feature` under `auth/` as the scaffold;
> the storyteller suggests the canonical name `stewarded-device-sync.feature`
> to align with the triple. Cartographer to rank.

---

## By epic

### social_medium/child/
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — `README.md` (graduated reach, protection-by-design, age-appropriate ceremony)

### governance_layers/geographic_political/family/child/
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — `README.md` (child constitutional role, growing autonomy, family-layer authority)

### governance_layers/geographic_political/family/parent/
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — `README.md` (parental authority bounded by service-to-child, household privacy)

### governance_layers/functional/educational/student/
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — *epic body is `.gitkeep` only; story anchors here as the first concrete instantiation of a student's experience inside the educational governance layer.*

### social_medium/ (epic.md)
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — anchors to the social_medium epic's "earned reach" and "attention as sacred" philosophical floor. The coop's decision instantiates the principle that *reach is negotiated BEFORE distribution*, applied to governance distribution rather than content distribution.

### social_medium/elder/
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — `README.md` (dignity-preserving protection, elder agency, wisdom-keeper-not-burden).
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — `README.md` (the elder is not a recipient of charity; she is a peer in a reciprocal relationship).
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — `README.md` (adaptive interface without condescension; agency preserved through recovery).

### governance_layers/geographic_political/family/elder/
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)* — `README.md` (elder constitutional role within family layer; the dignity-floor).
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)* — `README.md` (elder family-layer dignity from the counterparty side).
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)* — `README.md`.

### governance_layers/functional/qahal/ (acknowledged-gap)
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(canonical / delivery_status: undelivered)* — *the qahal functional governance layer has no epic body on disk; only the feature directory at `a2o/features/qahal/` and the elohim-app pillar exist. The story carries the philosophy until an epic body lands. Cartographer should rank "author governance_layers/functional/qahal/README.md" as a candidate Objective.*

### recovery/ (acknowledged-gap — recurring across three stories)
- [gertrude-holds-the-share](epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell) *(draft / delivery_status: undelivered)*
- [the-dowells-hold-gertrudes-share](epr:experience-story/matthew-manager/as-recovery-counterparty/backup-stewardship-for-household-gertrude) *(draft / delivery_status: undelivered)*
- [gertrude-logs-in-with-help-from-her-people](epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family) *(draft / delivery_status: undelivered)*

> **No `recovery/` or `resilience/` top-level epic body exists in
> `docs/content/elohim-protocol/`.** The recovery principle is carried in
> memory (project_recovery_grandma_standard, project_socially_derived_security,
> project_graduated_recovery_authority, project_elohim_as_counsel) and in the
> scattered feature suite at `a2o/features/auth/recovery/`, but there is no
> anchoring epic. All three new stories surface the same gap; their
> `anchors_epics` lists fall back to social_medium/elder and family/elder
> for the philosophical anchor. **Cartographer-rank: high** — three stories
> in a single batch converging on the same epic absence is a strong NEEDS-NEW-EPIC
> signal. Proposed epic title: `social_medium/recovery/README.md` or
> `governance_layers/cross_cutting/recovery/README.md` (depends on whether
> the philosophy is read as social_medium-aligned or as a cross-cutting
> governance pattern).

---

## By graduated memory

When the operator confirms a story `canonical`, the listed memory entries can be
deferred to the story by the librarian. The story becomes the load-bearing
artifact; the underlying memory entry can be archived (graduate) or sent to deep
tier (memorialize).

### Graduate (story carries the lesson; memory entry safe to archive)

#### project_household_fabric
- Graduated by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — archived at `.claude/archive/2026-05-14/graduated/project_household_fabric.md` on 2026-05-14.

#### project_multi_device_humans
- Graduated by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — archived at `.claude/archive/2026-05-14/graduated/project_multi_device_humans.md` on 2026-05-14.

#### project_ungrudging_service
- Graduated by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — archived at `.claude/archive/2026-05-14/graduated/project_ungrudging_service.md` on 2026-05-14.

### Memorialize (deep-tier preserve; story-pointer leads back when needed)

#### project_stewarded_child_identity
- Memorialized by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — memorialized at `.claude/archive/2026-05-14/memorialized/project_stewarded_child_identity.md` on 2026-05-14. The entry names Terrance as stewardee (superseded by James in the story); cradle-to-grave-lifecycle framing and the catch-up-burst vs steady-state sync distinction preserved in deep tier.

#### project_stewardship_philosophy
- Memorialized by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — memorialized at `.claude/archive/2026-05-14/memorialized/project_stewardship_philosophy.md` on 2026-05-14. The six-principle frame (graduated capability / accountable authority / visible shape / etc.) stays in deep archive; the story carries the lived shape but not every principle by name.

#### project_bootstrap_to_elohim_security_gradient
- Memorialized by: [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — memorialized at `.claude/archive/2026-05-14/memorialized/project_bootstrap_to_elohim_security_gradient.md` on 2026-05-14. The Stage-1 structural/social security pattern is dramatized in the story; the technical gradient stays in deep tier.

---

## Coverage gaps

Themes, subjects, roles, features, and epics that have **no story yet**.
Cartographer reads this section when ranking candidate Objectives.

### Role definitions missing
- **`role-as-stewardee`** — required by james-and-the-spoke's triple; closest existing is `role-social-medium-child.json`. A first-class `role-as-stewardee` would crosscut social_medium / governance / educational and serve every station of the cradle-to-grave stewardship spectrum (child, legal-custody ward, elder under POA, etc.). **Operator decision: create this role record?**
- **`role-as-steward`** — the inverse role (Jessica's role in this story). Multiple existing humans occupy it; not yet a first-class role.
- **`role-as-community-attestor`** — Terrance's role in this story. Distinct from `role-social-medium-content-creator` or `role-as-tutor`; would also serve neighborhood-witness scenarios.
- **`role-as-recovery-counterparty`** — required by gertrude-holds-the-share and the-dowells-hold-gertrudes-share triples. Peer-to-peer reciprocal role; distinct from steward/stewardee asymmetry. Universal across every share-holder in every recovery setup. Crosscuts imagodei / shefa / qahal / social_medium/elder. **Two stories in this batch flag the gap. Cartographer-rank: high.**
- **`role-as-account-claimant`** — required by gertrude-logs-in-with-help-from-her-people triple. Universal across every human asking for their own recovery (or any account-state-restoration). Crosscuts imagodei / social_medium/elder / family/elder. **Cartographer-rank: high.**

### Canonical features missing (cartographer to rank)
- **`stewarded-device-sync.feature`** — the canonical feature for james-and-the-spoke's triple. The earlier memory `project_stewarded_child_identity` proposes `humans-stewarded-child.feature` under `auth/`; the new triple shape suggests `stewarded-device-sync.feature` under `auth/` or a new top-level `stewardship/` directory.
- **`community-attestation.feature`** — referenced thematically by james-and-the-spoke; no canonical feature for the homeschool-coop-attestation flow yet.
- **`household-sync-handshake.feature`** — referenced thematically; no canonical feature for the spoke-open / spoke-close handshake yet.
- **`backup-stewardship-for-household-dowell.feature`** — share-holder-as-subject ceremony from the receiving side (gertrude-holds-the-share). Adjacent: `auth/recovery/recovery-shamir-optional.feature` (Matthew-as-subject; treats share-recipient as fixture). Proposed location: `auth/recovery/` or new `auth/recovery/share-custody/`. **Cartographer-rank: high.**
- **`backup-stewardship-for-household-gertrude.feature`** — the reciprocal-direction companion. Authoring either feature informs the other. Same proposed location.
- **`social-recovery-with-help-from-family.feature`** — the canonical feature for the entire `project_recovery_grandma_standard` memory. Tests the grandma-standard UX invariant (no jargon to claimant, no seed bytes, ambient throughout, elohim-mediated translation). Adjacent: the existing `auth/recovery/recovery-m5-*.feature` suite, which tests substrate-side mechanics but not the load-bearing UX invariant. **Cartographer-rank: highest** — this is the load-bearing test for the foundational recovery memory, and no Gherkin yet covers it.

### Epics missing (cartographer to rank)
- **`recovery/` or `resilience/` epic body** — three stories in this batch (gertrude-holds-the-share, the-dowells-hold-gertrudes-share, gertrude-logs-in-with-help-from-her-people) all flag the same gap. No `docs/content/elohim-protocol/.../recovery/README.md` exists; the recovery principle is currently load-bearing in memory + feature suite alone, with no anchoring epic. Proposed locations: `social_medium/recovery/README.md` (if read as social_medium-aligned), `governance_layers/cross_cutting/recovery/README.md` (if read as a cross-cutting governance pattern). The Gertrude/Dowell story-batch is the substrate that would inform the epic. **Cartographer-rank: high.**

### (subject, role, feature) triples that have no story yet
The corpus is brand new. Each row below is a candidate Tier 1 anchor.

- ~~`(human-gertrude-grandma, role-as-stewardee, recovery-by-people)`~~ — partially covered by **gertrude-logs-in-with-help-from-her-people** with the more accurate role `role-as-account-claimant` (the claimant in recovery is asking for themself, not being stewarded). The grandma-standard recovery ceremony is now narratively carried.
- `(human-gertrude-grandma, role-as-stewardee, elder-care-stewardship)` — the other end of the cradle-to-grave spectrum; cognitive-change scenarios where Gertrude becomes stewarded rather than stewarding.
- `(human-james-son, role-as-steward, coming-of-age-graduation)` — "James at fifteen": the moment a ward becomes a steward of their own spoke.
- `(human-jessica-spouse, role-as-steward, stewarded-device-sync)` — same flow from the steward's vantage; shared body text with this story.
- ~~`(human-terrance-tutor, role-as-community-attestor, learning-attestation-cycle)`~~ — partially carried by **the-coop-decides** (Terrance as subject in role-as-collective-steward over collective-governance feature). The `role-as-community-attestor` framing remains a distinct gap; the coop story carries Terrance's facilitator-as-quorum-keeper aspect, not his individual-attestation-of-James aspect.
- `(human-pam-polarized, role-as-non-member, ungrudging-service-to-outsiders)` — externalities to neighbors who reject the network.
- `(human-adam-firstman, role-as-bootstrap-steward, alpha-cluster-bring-up)` — bootstrap pair, founder-class story.
- `(collective-maintainers, role-as-protocol-stewards, capture-resistance-handoff)` — collective-as-subject; the protocol's own self-preservation story.
- `(human-jessica-spouse, role-as-recovery-counterparty, backup-stewardship-for-household-*)` — Jessica's vantage on the reciprocal-backup ceremony; she is the one who actually taps Accept in the-dowells-hold-gertrudes-share. Shared body text candidate.
- `(human-matthew-manager, role-as-account-claimant, social-recovery-with-help-from-family)` — Matthew on the receiving end of his own recovery; the more technically-fluent version of gertrude-logs-in-with-help-from-her-people, instructive for showing that the grandma-standard UX serves builders too.

### Themes with no story
- ~~**recovery**~~ — now covered by three stories in this batch. Remaining sub-themes: lost-device-with-no-time (mid-life-emergency), recovery-while-under-duress (elohim-as-counsel in motion), cross-doorway-recovery.
- **graduated-autonomy / coming-of-age** — the moment a ward becomes a steward
- **elder-care stewardship** — the other end of the cradle-to-grave spectrum; cognitive-change scenarios
- **legal-custody stewardship** — court-mediated capability grants
- **ungrudging service to non-members** — externalities to opt-outs
- **multi-doorway resilience** — the human registered with two doorways failing over
- **avodah / contribution as worship** — contribution-as-protocol-participation
- **economic flows** — no story carries the protocol's economic shape into kitchen-table language
- **living-memory** — the memory-as-substrate epic that this catalog itself serves has no story

### Epics with no story
- **economic_coordination/** (all sub-epics)
- **autonomous_entity/** — the elohim-agent perspective
- **public_observer/** — the witness-and-attestation epic
- **value_scanner/** — referenced by family/child + family/parent README, never instantiated
- **observer-protocol/** — adjacent to public_observer
- **global-orchestra/** — protocol-as-symphony framing
- **living_memory/** — the meta-story this catalog serves

---

## Maintenance notes

- This INDEX is hand-curated. When a story is added or its frontmatter
  changes, the storyteller updates this file in the same edit.
- **Per-row format**: every row shows `*(status / delivery_status: X)*` — author-axis
  and substrate-axis together. The two are orthogonal. Storyteller maintains the
  author-axis; the `deliver-bridge` auto-poller maintains `delivery_status` by
  reading `/deliver`'s tier-3 verdicts. Storyteller does NOT author
  `delivery_status` directly.
- A future validator (planned in `CONVENTIONS.md`) will flag orphan stories
  absent from this INDEX, AND flag rows where the rendered `delivery_status`
  drifts from the story frontmatter. Until then, the storyteller is the
  validator on the author-axis; the `deliver-bridge` poll output is the
  validator on the substrate-axis.
- The "Coverage gaps" section is the cartographer's input. Treat it as a
  rolling backlog of candidate next-stories, not as a complete enumeration.
- Retired stories: `james-and-the-spoke.md` (the legacy-frontmatter sibling
  of the active triple file) is preserved in the directory with
  `status: retired` per CONVENTIONS.md.
- `james-son--as-stewardee--stewarded-device-sync` flipped from `draft` to
  `canonical` on 2026-05-14. The librarian ceremony that day graduated
  three memory entries (household_fabric / multi_device_humans /
  ungrudging_service) and memorialized three more (stewarded_child_identity /
  stewardship_philosophy / bootstrap_to_elohim_security_gradient) into
  `.claude/archive/2026-05-14/`. Run #2 retro surfaced that this graduation
  happened with `delivery_status: undelivered` (the canonical feature
  `stewarded-device-sync.feature` does not exist); per the new Run #2
  taxonomy this is `graduated-narratively`, and a `delivery-debt` flag
  (cartographer backlog: "author `stewarded-device-sync.feature` + run
  through `/deliver`") was attached. See [[feedback_story_delivery_status_axis]].
- `terrance-tutor--as-collective-steward--coop-decides` (slug: `the-coop-decides`)
  flipped from `draft` to `canonical` on 2026-05-15 (Run #6 Wave 4 operator
  approval). No body changes — storyteller Wave 2 read surfaced no revision
  dimension. No memory entries graduated/memorialized in this flip (the
  `graduates_memory: []` and `memorializes: []` frontmatter blocks remain
  empty by design; the story's lesson is governance-process, which the
  current memory corpus does not yet hold as a discrete-archivable entry).
  Substrate-axis remains `undelivered` (canonical feature `collective-
  governance.feature` exists on disk but no `/deliver` tier-3 verdict has
  been minted; per Run #2 taxonomy this is `graduated-narratively` for the
  story, with delivery-debt carryforward on the verdict pass).
- **2026-05-18 — Gertrude reciprocal-backup batch (three stories, all `draft`)**:
  authored against the deployment commit `64f5e1b84` that formalized
  Gertrude as a deployed peer on shem (device-home-nuc; remote+performance
  nodeTypes) to serve as the reciprocal-backup counterparty for the Dowell
  household. Three stories landed:
    1. `gertrude-grandma--as-recovery-counterparty--backup-stewardship-for-household-dowell` (slug: `gertrude-holds-the-share`) — share-acceptance from the holder's vantage.
    2. `matthew-manager--as-recovery-counterparty--backup-stewardship-for-household-gertrude` (slug: `the-dowells-hold-gertrudes-share`) — reciprocal direction; the elohim as counsel for the relationship.
    3. `gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family` (slug: `gertrude-logs-in-with-help-from-her-people`) — the grandma-standard met end-to-end.
  All three remain `status: draft` pending operator review; none have flipped
  to canonical and no memory entries have graduated. The batch surfaced
  three coverage gaps consistently: (a) two new roles (`role-as-recovery-counterparty`,
  `role-as-account-claimant`); (b) three new canonical features
  (`backup-stewardship-for-household-dowell.feature`,
  `backup-stewardship-for-household-gertrude.feature`,
  `social-recovery-with-help-from-family.feature`); (c) one new epic
  (`recovery/` or `resilience/` epic body absent from
  `docs/content/elohim-protocol/`). Per the operator's commit message, the
  structural schema for the stewardship-agreement field-shape is being
  deliberately informed by these stories first — the stories are
  story-first substrate for the protocol-law schema, not the other way
  around. Substrate-axis is `undelivered` (floor) on all three; per Run #2,
  the operator-confirm flip will be `graduated-narratively` until the
  feature files land + `/deliver` mints verdicts.
