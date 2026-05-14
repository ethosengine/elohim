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
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)*

### graduated-authority
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

### community-attestation
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — `(human-terrance-tutor, role-as-collective-steward, collective-governance)`

### household-fabric
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

### ungrudging-service
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

### governance
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — six homeschool families rank a history curriculum by ranked-choice; the elohim casts proxy and justifies in plain language.

### ranked-choice
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)*

### collective-decision
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)*

### elohim-justification
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)*

---

## By subject

### human-james-son (James)
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)* — protagonist; the stewarded child opening a spoke into the family ring.

### human-terrance-tutor (Terrance)
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — protagonist; the facilitator whose curriculum disposition weights the coop's ranked-choice tally and whose attestation history the elohim references in proxy votes.

### Additional characters (not the subject of their own story yet)

The following humans appear in stories but are not yet
subjects of their own canonical stories — listed as coverage gaps below.

- **human-jessica-spouse** (Jessica) — the steward in james-and-the-spoke; the proposer in the-coop-decides; coop member at Valley Homeschool Co-op.
- **human-matthew-manager** (Matthew) — co-parent; named the spoke, runs the family node.

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
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)*

> Role record `role-as-collective-steward` **does not yet exist** in
> `genesis/data/lamad/content/`. Distinct from `@as-stewardee` (an individual
> being stewarded) and from `@as-steward` (the steward of an individual ward):
> a collective-steward stewards a *collective decision process* — facilitator-
> as-quorum-keeper. Crosscuts qahal / educational / affinity governance.
> Surfaced as a coverage gap; operator decision.

---

## By feature

### stewarded-device-sync
- [james-and-the-spoke](epr:experience-story/james-son/as-stewardee/stewarded-device-sync) *(canonical / delivery_status: undelivered)*

### collective-governance
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — canonical feature exists at `genesis/a2o/features/qahal/collective-governance.feature` (one of the few stories whose canonical feature is already on disk). Anchors specifically to scenarios: "Community uses ranked-choice to pick a curriculum path" (lines 40-58), "Elohim builds governance disposition from voting history" (lines 240-247), and "Elohim votes as proxy when human hasn't engaged" (lines 249-256). Substrate-axis remains `undelivered` until `/deliver` mints a tier-3 verdict.

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
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — anchors to the social_medium epic's "earned reach" and "attention as sacred" philosophical floor. The coop's decision instantiates the principle that *reach is negotiated BEFORE distribution*, applied to governance distribution rather than content distribution.

### governance_layers/functional/qahal/ (acknowledged-gap)
- [the-coop-decides](epr:experience-story/terrance-tutor/as-collective-steward/collective-governance) *(draft / delivery_status: undelivered)* — *the qahal functional governance layer has no epic body on disk; only the feature directory at `a2o/features/qahal/` and the elohim-app pillar exist. The story carries the philosophy until an epic body lands. Cartographer should rank "author governance_layers/functional/qahal/README.md" as a candidate Objective.*

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

### Canonical features missing (cartographer to rank)
- **`stewarded-device-sync.feature`** — the canonical feature for james-and-the-spoke's triple. The earlier memory `project_stewarded_child_identity` proposes `humans-stewarded-child.feature` under `auth/`; the new triple shape suggests `stewarded-device-sync.feature` under `auth/` or a new top-level `stewardship/` directory.
- **`community-attestation.feature`** — referenced thematically by james-and-the-spoke; no canonical feature for the homeschool-coop-attestation flow yet.
- **`household-sync-handshake.feature`** — referenced thematically; no canonical feature for the spoke-open / spoke-close handshake yet.

### (subject, role, feature) triples that have no story yet
The corpus is brand new. Each row below is a candidate Tier 1 anchor.

- `(human-gertrude-grandma, role-as-stewardee, recovery-by-people)` — recovery's grandma-standard ceremony (see `project_recovery_grandma_standard`).
- `(human-gertrude-grandma, role-as-stewardee, elder-care-stewardship)` — the other end of the cradle-to-grave spectrum.
- `(human-james-son, role-as-steward, coming-of-age-graduation)` — "James at fifteen": the moment a ward becomes a steward of their own spoke.
- `(human-jessica-spouse, role-as-steward, stewarded-device-sync)` — same flow from the steward's vantage; shared body text with this story.
- ~~`(human-terrance-tutor, role-as-community-attestor, learning-attestation-cycle)`~~ — partially carried by **the-coop-decides** (Terrance as subject in role-as-collective-steward over collective-governance feature). The `role-as-community-attestor` framing remains a distinct gap; the coop story carries Terrance's facilitator-as-quorum-keeper aspect, not his individual-attestation-of-James aspect.
- `(human-pam-polarized, role-as-non-member, ungrudging-service-to-outsiders)` — externalities to neighbors who reject the network.
- `(human-adam-firstman, role-as-bootstrap-steward, alpha-cluster-bring-up)` — bootstrap pair, founder-class story.
- `(collective-maintainers, role-as-protocol-stewards, capture-resistance-handoff)` — collective-as-subject; the protocol's own self-preservation story.

### Themes with no story
- **recovery** — grandma-standard ceremony (see `project_recovery_grandma_standard`, `project_socially_derived_security`)
- **graduated-autonomy / coming-of-age** — the moment a ward becomes a steward
- **elder-care stewardship** — the other end of the cradle-to-grave spectrum
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
