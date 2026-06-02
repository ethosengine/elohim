---
status: design   # content audit — inventory + integration decisions, planning input
related:
  - 2026-05-22-scenario-archaeology-and-archetype-map.md   # the Sprint 0.5 archaeology this audit extends
---

# Value-Scanner Content Audit

**Date:** 2026-05-22
**Author:** Sprint 0.5 follow-up audit (qahal-pillar archetype work)
**Status:** Audit report — operator decisions surfaced; no file moves executed.
**Companion docs:**
- Gospel-tier vision: `/projects/elohim/genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`
- Epic: `/projects/elohim/genesis/docs/content/elohim-protocol/value_scanner/epic.md`
- .feature archaeology: `/projects/elohim/genesis/docs/plans/2026-05-22-scenario-archaeology-and-archetype-map.md`

---

## Why this audit exists

The Sprint 0.5 scenario archaeology inventoried 76 `.feature` files at `/projects/elohim/genesis/a2o/features/` and surfaced two MVP-critical gaps: faith-community (zero primary scenarios) and life-group (zero primary scenarios). That inventory did **not** look at the much larger value-scanner content body — 21 archetype directories of canonical narrative at `/projects/elohim/genesis/docs/content/elohim-protocol/value_scanner/` and 1,681 generated `.json` scenarios at `/projects/elohim/genesis/data/lamad/content/scenario-value-scanner-*.json`. That corpus is the substrate-primitive expression of the household-as-living-core claim in vision spec Section 1.2.

This audit closes that gap. It inventories what exists, situates it inside the Qahal architecture catalog, cross-references it against the .feature archaeology, and surfaces operator decisions about integration.

---

## Section 1 — Epic framing summary

The value-scanner epic (`value_scanner/epic.md`, ~3,000 words) is *The Strawberry Revolution*. Ten-year-old Tommy Parker stands in the produce aisle of his neighborhood corner store on a Tuesday afternoon and scans a container of strawberries for his six-year-old sister Emma. His phone vibrates gently: *"Emma will love these! +5 care bonus for thinking of sister."* In that one moment, the epic crystallizes its central claim: **care has become computable, valuable, and exchangeable**.

The framing rests on four moves the protocol makes that legacy economic substrate cannot:

1. **Reframe family coordination as value creation rather than overhead.** Today, the average parent spends 11 hours per week on household management — invisible labor that legacy economics treats as zero-value because it has no instrument to measure it. The value-scanner *is the instrument*. Care is the primary currency; money is one type of value among many.

2. **Subsume the "monthly rent on childhood" extraction stack.** Families pay $5–15/month each to Greenlight, FamZoo, Capital One Teen, Acorns Early — up to $300/year per family to "teach kids about money," which actually teaches them consumption, surveillance, and that value equals dollars. The epic names this as a hidden tax on modern families and proposes a free, local-first, values-aligned alternative.

3. **Make care visible through the multi-elohim negotiation pattern.** Each scan triggers a millisecond conversation between five agents — Personal Elohim, Family Elohim, Store Elohim, Community Elohim, Global Elohim — each protecting different stakes (individual budget, family needs, store interest, community values, global supply chain awareness). Tommy doesn't see this; he sees "Family approved! Get the large size." The protocol carries the complexity; the child carries the dignity.

4. **Bundle the day into Story.** The protocol loop is **Story → Scan → Negotiate → Bundle → Story**. Morning planning aggregates into missions. Active shopping produces real-time negotiation. Checkout resolves into a single QR code containing economic transaction, value creation, story elements, and REA flows. Evening reflection becomes family narrative — Tommy made breakfast for Emma; Emma helped the neighbor; ninety minutes of parent time saved; Johnson Farm supported.

This connects directly to vision-spec Section 1.2: *"The household is the living core where the protocol becomes embodied... The value-scanner machinery is the substrate primitive that makes care-economy REA visible at the individual and household level."* And to 7.6a: *"The substrate spreads by being lived... The diffusion does not require persuasion. The diffusion requires the seed — the household — to be reachable and operable for ordinary people. Once that is real, the rest follows."* The epic is the seed. The 1,681 scenarios are 1,681 worked moments of the seed taking root — Tommy at the corner store, Sarah managing the household's 11-hour-per-week coordination load, the grandparent across household boundaries, the worker in a food-service industry that exploits, the person with disabilities navigating accommodation. Every cell of the matrix is an instance of *"why isn't this like home?"* common sense being formed.

---

## Section 2 — Archetype directory inventory

The `/projects/elohim/genesis/docs/content/elohim-protocol/value_scanner/` tree contains **22 entries** (epic.md + 21 directories). Of the 21 directories, **19 are archetype directories with READMEs and corresponding scenario JSON** (the audit found a 20th, `student`, that the original brief omitted). Three further directories — `audio/`, `documents/`, `organizations/` — are *not* archetype directories; they are source-material references (e.g., `documents/artificial_intelligence_values_and_alignment/`, `organizations/about_p2p_foundation/`) bundled in for narrative grounding. They have no scenarios.

The 19 archetype directories carry developmental / life-stage / condition framing as follows. The `archetype_name` field is extracted verbatim from each README's frontmatter.

| Archetype | README path | One-sentence framing |
|---|---|---|
| **young-child** | `value_scanner/young_child/README.md` | "Young Child in Pure Discovery" — ages ~3–5, pre-literate scanning, parental pre-approval everywhere, learning-and-helping only. |
| **child** | `value_scanner/child/README.md` | "Child in Guided Autonomy" — ages 8–10 (Emma's stage), pre-approved items, small discovery budget, gentle nudges toward healthy choices. |
| **preteen** | `value_scanner/preteen/README.md` | "Preteen in Real Responsibility" — ages 11–13, manages actual budget, trades tokens with peers, starts helping younger siblings. |
| **teen** | `value_scanner/teen/README.md` | "Teen in Full Participation" — ages 14+, complete autonomy within family values, mentors younger siblings, shopping becomes intentional values expression. |
| **young-adult** | `value_scanner/young_adult/README.md` | "Young Adult in Early Independence" — first solo household, learning the substrate's adult-tier responsibility surface. |
| **adult** | `value_scanner/adult/README.md` | "Adult in Prime Working/Parenting Years" — Sarah Parker's stage; manages household + work + community simultaneously. |
| **parent** | `value_scanner/parent/README.md` | "Parent as Primary Caregiver" — child-rearing as the central care-economy contribution; Tommy and Emma's parents' shape. |
| **single-parent** | `value_scanner/single_parent/README.md` | "Single Parent Managing Solo Household" — Maria-shape; bears full coordination load alone; substrate-as-relief most acute here. |
| **middle-aged** | `value_scanner/middle_aged/README.md` | "Middle-Aged Adult in Established Life Stage" — broadest governance_scope of any archetype (8 layers including industry_sector). |
| **caregiver** | `value_scanner/caregiver/README.md` | "Caregiver for Dependent Adults" — James caring for David (progressive condition); care work as primary economic activity. |
| **senior** | `value_scanner/senior/README.md` | "Senior in Early Retirement" — Margaret-shape; transition from earning to contributing; municipality interface heavy. |
| **grandparent** | `value_scanner/grandparent/README.md` | "Grandparent as Care Provider and Elder" — coordinates across household boundaries (own household + adult-children's households). |
| **retired** | `value_scanner/retired/README.md` | "Retired Person in Post-Employment Life" — fixed income, abundant time, community contribution opportunities. |
| **elderly** | `value_scanner/elderly/README.md` | "Elderly in Advanced Age" — independence preservation, care-receiving, dignity at end-of-life. |
| **student** | `value_scanner/student/README.md` | "Student as Active Learner" — bridges teen / young-adult; educational governance_scope is primary. |
| **worker** | `value_scanner/worker/README.md` | "Worker Balancing Employment and Care" — only archetype with industry_sector in governance_scope; food-service exploitation visible. |
| **idd-community** | `value_scanner/idd_community/README.md` | "IDD Community Member as Valued Participant" — Jordan-shape; autonomy + protection from exploitation; sensory-friendly accommodation. |
| **person-with-disabilities** | `value_scanner/person_with_disabilities/README.md` | "Person with Disabilities as Full Participant" — Alex-shape; accessible tech as workplace obligation not special treatment. |
| **vulnerable-temporary** | `value_scanner/vulnerable_temporary/README.md` | "Person Experiencing Temporary Vulnerability" — illness, stress, life transition; substrate adjusts care surface temporarily. |

**Non-archetype directories** (carry no scenarios — keep but note):
- `audio/home_initiative_for_digital_public_infrastructure/` — source-material references.
- `documents/artificial_intelligence_values_and_alignment/` — source-material references.
- `organizations/{p2p-foundation, commons-engine, kolibri, gifct, ...}/` — institutional references for narrative grounding.

**The brief listed 21 archetypes; this audit found 19.** The brief's count appears to have folded `audio`, `documents`, `organizations` (non-archetype source dirs) into the archetype count.

---

## Section 3 — Scenario statistics

### 3.1 Scenarios per archetype (descending)

| Archetype | Count |
|---|---|
| adult | 113 |
| middle-aged | 109 |
| young-adult | 103 |
| person-with-disabilities | 102 |
| student | 102 |
| single-parent | 101 |
| teen | 98 |
| parent | 96 |
| senior | 96 |
| worker | 96 |
| idd-community | 86 |
| vulnerable-temporary | 86 |
| caregiver | 85 |
| retired | 85 |
| elderly | 84 |
| grandparent | 84 |
| preteen | 71 |
| child | 51 |
| young-child | 33 |
| **TOTAL** | **1,681** |

The distribution roughly mirrors developmental governance breadth: adult / middle-aged / young-adult sit at the top because their `governance_scope` reaches into 7–8 layers; young-child sits at the bottom because its scope is just `[individual, family]`.

### 3.2 Scenarios per context (descending)

| Context | Count |
|---|---|
| family | 230 |
| individual | 222 |
| neighborhood | 216 |
| community | 211 |
| household | 180 |
| ecological-bioregional | 180 |
| educational | 177 |
| workplace-organizational | 152 |
| municipality | 62 |
| healthcare-medical | 26 |
| industry-sector | 25 |
| **TOTAL** | **1,681** |

### 3.3 2D matrix — archetypes × contexts

`·` = no scenarios in cell. Numbers around 11–15 indicate the corpus aimed for ~12 scenarios per applicable (archetype × context) cell.

| Archetype | household | family | individual | neighborhood | community | educational | workplace organizational | municipality | ecological bioregional | healthcare medical | industry sector | TOTAL |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **young-child** | · | 11 | 10 | · | · | 12 | · | · | · | · | · | **33** |
| **child** | · | 12 | 12 | 12 | · | 15 | · | · | · | · | · | **51** |
| **preteen** | · | 12 | 11 | 11 | 12 | 12 | · | · | 13 | · | · | **71** |
| **teen** | · | 12 | 11 | 12 | 12 | 12 | 13 | 13 | 13 | · | · | **98** |
| **young-adult** | 12 | 12 | 12 | 12 | 13 | 14 | 13 | · | 15 | · | · | **103** |
| **adult** | 12 | 12 | 12 | 12 | 13 | 13 | 13 | 13 | 13 | · | · | **113** |
| **parent** | 12 | 12 | 12 | 12 | 12 | 12 | 12 | · | 12 | · | · | **96** |
| **single-parent** | 12 | 13 | 12 | 12 | 13 | 13 | 13 | · | 13 | · | · | **101** |
| **middle-aged** | 12 | 12 | 12 | 12 | 12 | · | 12 | 12 | 13 | · | 12 | **109** |
| **caregiver** | 12 | 12 | 12 | 12 | 12 | · | 12 | · | · | 13 | · | **85** |
| **senior** | 12 | 12 | 12 | 12 | 12 | 12 | · | 12 | 12 | · | · | **96** |
| **grandparent** | 12 | 12 | 12 | 12 | 12 | 12 | · | · | 12 | · | · | **84** |
| **retired** | 12 | 12 | 12 | 12 | 13 | · | · | 12 | 12 | · | · | **85** |
| **elderly** | 12 | 12 | 11 | 12 | 12 | 12 | · | · | 13 | · | · | **84** |
| **student** | 12 | 13 | 12 | 12 | 13 | 13 | 13 | · | 14 | · | · | **102** |
| **worker** | 12 | 12 | 11 | 12 | 12 | · | 12 | · | 12 | · | 13 | **96** |
| **idd-community** | 12 | 12 | 12 | 12 | 12 | 13 | 13 | · | · | · | · | **86** |
| **person-with-disabilities** | 12 | 13 | 12 | 13 | 13 | · | 13 | · | 13 | 13 | · | **102** |
| **vulnerable-temporary** | 12 | 12 | 12 | 12 | 13 | 12 | 13 | · | · | · | · | **86** |
| **TOTAL** | **180** | **230** | **222** | **216** | **211** | **177** | **152** | **62** | **180** | **26** | **25** | **1,681** |

### 3.4 Density observations

**Highest-density cells** (everywhere around 12-15; the corpus was generated with a target cell count):
- The 6 anchor contexts — `family`, `individual`, `neighborhood`, `community`, `household`, `ecological-bioregional` — have near-uniform 12-scenario coverage across every adult-and-up archetype. This is the **household-and-its-immediate-rings** band the vision spec Section 4 describes as concentric.
- `educational` has dense coverage for *every developmental archetype* (young-child through young-adult, plus parent / single-parent / student) but is empty for caregiver / worker / retired / person-with-disabilities — institutional alignment, not life-stage.

**Notable gaps the matrix surfaces:**
- **`household` is empty for the four youngest archetypes** (young-child, child, preteen, teen). This is **defensible** — those archetypes' governance_scope per README does not include `household` (they participate *in* a household but are not stewards *of* one); `family` is their household-shape context.
- **`municipality` is sparse** (62 scenarios total, present only for teen / adult / middle-aged / senior / retired). This maps to vision-spec 5.9 city-hall and is appropriately civic-stage-restricted.
- **`healthcare-medical` is restricted to caregiver + person-with-disabilities only** (26 scenarios). Vision-spec 6.9 (health services) is Tier 3 / far-horizon — this matches the gospel-tier scope discipline.
- **`industry-sector` is restricted to worker + middle-aged only** (25 scenarios). Vision-spec 5.6 industry-association — appropriately worker-archetype-anchored.
- **`workplace-organizational` is empty for child / preteen / senior / grandparent / retired / elderly** — the not-currently-employed archetypes; correct.

The corpus appears generator-produced (uniform 11–15 per cell, identical timestamps, consistent stewardship records) — not hand-authored. This is a *catalog of the archetype × context space*, not a story-harvested corpus. The strength is coverage; the weakness is uniformity (each scenario is a one-paragraph beat without the canonical-character grounding that the .feature archaeology corpus carries).

---

## Section 4 — Sampled scenarios

Twelve scenarios chosen to span the matrix. Each is summarized in ~80 words from the embedded Gherkin `content` field. Filenames are relative to `/projects/elohim/genesis/data/lamad/content/`.

### 4.1 `scenario-value-scanner-young-child-scenarios-family-young child contributes to family decision making appropriately.json`

**Cell:** young-child × family. Tommy (named explicitly in the scenario body) gives input about meals through the scanner; the family_elohim solicits his preferences age-appropriately, weights his care intentions (choosing for others) alongside family needs, and the scenario asserts that "Tommy should experience having voice in family governance" — *participation should be genuine, not token*. This is the youngest-archetype expression of the Imago Dei discriminator: inherent dignity is operative at age four, not a developmental milestone.

### 4.2 `scenario-value-scanner-child-scenarios-neighborhood-child learns about local food systems through scanner.json`

**Cell:** child × neighborhood. Emma (Tommy's sister, age six in the epic) scans locally-produced items in her neighborhood. Her personal_agent explains: *"This was grown by farmers just 20 miles away."* The neighborhood_elohim makes regional care economies visible. Emma develops "sense of place-based economic relationships." This is the bridge from household scanning to bioregional economic literacy — the epic's *"At the Community Garden"* claim rendered as a learning beat for the youngest scanning-capable archetype.

### 4.3 `scenario-value-scanner-teen-scenarios-workplace-organizational-part-time work integrated with care economy understanding.json`

**Cell:** teen × workplace-organizational. Jasmine (a teen archetype name) earns income from part-time work *and* contributes unpaid household labor and community volunteering. Her personal_agent aggregates both, building "sophisticated understanding that economy is broader than paid employment" and internalizing the care-economy framework as normal economic thinking. This scenario is the substrate's answer to the canonical-economics framing that teen wages are training-for-real-economy; the protocol asserts care work *is* real-economy from the first paycheck.

### 4.4 `scenario-value-scanner-adult-scenarios-household-coordinating complex household schedules across family members.json`

**Cell:** adult × household. Sarah (canonical adult-archetype name, same Sarah as the epic) manages work, kids' schools, household tasks, appointments; her partner has a separate schedule. The household_elohim integrates all family calendars, identifies conflicts, suggests solutions. The scenario explicitly asserts: *"schedule coordination should reduce stress, not increase it."* This is the canonical 11-hours-per-week-coordination-burden problem from the epic, addressed at substrate level — coordination becomes infrastructure, not relentlessly-personal mental load.

### 4.5 `scenario-value-scanner-single-parent-scenarios-educational-before and after school care affordable and accessible for working single parents.json`

**Cell:** single-parent × educational. Maria works 8-5, school runs 8-3, and she cannot afford $400+/month per child for extended care across three children. The educational_elohim helps her access affordable before/after care; subsidies are available for single-income families. This scenario surfaces a *policy gap visible at protocol scale* — the protocol can't unilaterally fund extended care, but it can route Maria to it, make the cost visible, and assert that "Maria should be able to maintain employment through affordable school-based care" as a substrate floor.

### 4.6 `scenario-value-scanner-idd-community-scenarios-community-accessing community cultural opportunities.json`

**Cell:** idd-community × community. Jordan loves musicals; the local theater offers sensory-friendly performances. The personal_agent helps Jordan select and prepare; the theater environment adapts (lights, volume). Jordan experiences "full community cultural citizenship." The scenario explicitly names the reciprocity: *"the theater benefits from inclusive programming."* This is the Imago Dei discriminator (Section 1.5) at the IDD-community archetype: inherent dignity is non-negotiable; "accessibility allows Jordan's flourishing and joy" — flourishing, not just access.

### 4.7 `scenario-value-scanner-person-with-disabilities-scenarios-workplace-organizational-accessible technology is workplace obligation not special treatment.json`

**Cell:** person-with-disabilities × workplace-organizational. Alex requires accessible technology; the scenario asserts this as *legal requirement, not accommodation*. The workplace_elohim documents technology accessibility, tracks when barriers prevent Alex from working, holds employer accountable. The substrate "should recognize accessible tech as baseline, not luxury." This is the closest the value-scanner corpus comes to substrate-floor enforcement language — and it maps directly to the Imago Dei discriminator's *"any Qahal mechanism that violates the inherent-dignity floor of any being is refused by the substrate"* from spec Section 1.5.

### 4.8 `scenario-value-scanner-senior-scenarios-municipality-access to senior nutrition programs without stigma.json`

**Cell:** senior × municipality. Margaret (the grandparent-archetype name, also used here for senior — naming overlap noted) manages nutrition on $32k/year fixed income. The municipality_elohim connects her to Meals on Wheels or congregate dining. The scenario's discriminator: *"the framing should be senior service, not poverty relief"* — Margaret should access programs *with dignity*. Her own care contributions should remain visible "to counterbalance receiving." This is the dignity-floor expressed as ledger discipline: receiving never erases prior contribution.

### 4.9 `scenario-value-scanner-grandparent-scenarios-family-coordinating between margaret's household and sarah's household.json`

**Cell:** grandparent × family. Margaret has her own household with Robert; Sarah has hers; the family_elohim coordinates between them while respecting both autonomies. The scenario asserts: *"neither household should dominate the other"* and "multigenerational care should work across household boundaries." This is the cross-household friction-gradient applied at intimate-relationship scale — a vital substrate property the Section 4 canonical narratives carry (Sheila checking in on the Dowell household from across the continent) but most explicit here.

### 4.10 `scenario-value-scanner-worker-scenarios-industry-sector-addressing industry culture of exploitation.json`

**Cell:** worker × industry-sector. Food-service industry culture normalizes worker exploitation. The protocol supports cultural-change advocacy, helps workers articulate vision for dignified work, builds consciousness "that current conditions are unjust not inevitable," creates movement for care-economy-aligned transformation. This is the value-scanner corpus's brush with vision-spec 5.1 (ChickenMax → EAE conversion) — but at industry-association scale (5.6) rather than per-EAE. The scenario authors collective-action substrate without yet invoking the EAE pattern; it would benefit from explicit cross-reference.

### 4.11 `scenario-value-scanner-caregiver-scenarios-healthcare-medical-advanced care planning supported early and comprehensively.json`

**Cell:** caregiver × healthcare-medical. David's condition is progressive; James (his caregiver) needs to support advance-directive documentation while David can still participate. The healthcare_elohim prompts early discussion, helps document wishes, coordinates planning across providers. This is the substrate's expression of the *witness-of-harm + attestation-of-repair + ongoing-acknowledgment* triad from spec 1.5 — applied to end-of-life care rather than to reconciliation. The scenario takes seriously the proposition that dignity-of-decision-making is itself a primitive: David's voice must be captured before capability is lost.

### 4.12 `scenario-value-scanner-middle-aged-scenarios-ecological-bioregional-bioregional care economy networks reduce consumption through sharing.json`

**Cell:** middle-aged × ecological-bioregional. Robert (Margaret's spouse — naming continuity within the corpus) participates in bioregional sharing — tool libraries, car sharing, resource exchange. The bioregional_elohim tracks ecological benefits: *"Tool sharing prevented 8 redundant purchases."* The scenario closes by asserting that "care economy and ecological sustainability are aligned" and that bioregional resilience is *built through sharing*. This connects the household-scale value-scanner to vision-spec 6.17 (natural collectives / bioregion / biodiversity) — the donut-endstate ceiling expressed in micro-decisions about whether to buy or to borrow.

---

## Section 5 — Mapping value-scanner content to the Qahal catalog

The vision spec's catalog (Sections 4–6) describes 4 Tier-0 + 9 Tier 1+2 + 18 Tier 3 archetypes. The value-scanner contexts in scenario filenames map onto this catalog as follows.

### 5.1 Tier 0 (Section 4) — direct cell coverage

| Vision spec section | Catalog archetype | Value-scanner context | Cells | Confidence |
|---|---|---|---|---|
| 4.1 Dowell household | T0:household | `household` (180) + `family` (230) | 17 cells (household) × 19 (family) | HIGH — direct |
| 4.2 Faith community | T0:faith-community | — | — | **GAP** — no context |
| 4.3 Life-group | T0:life-group | — | — | **GAP** — no context |
| 4.4 Wisdom commons | T0:wisdom-commons | — | — | **GAP** — no context |

The value-scanner corpus is **richly aligned with T0:household** (the living core) but has **zero direct coverage** of T0:faith-community, T0:life-group, or T0:wisdom-commons. This is the same Tier-0 gap the .feature archaeology surfaced — the value-scanner does not fill it. Section 6 below treats this as the most important finding of this audit.

### 5.2 Tier 1+2 (Section 5) — context-to-archetype mapping

| Value-scanner context | Best vision-spec match | Coverage | Confidence |
|---|---|---|---|
| `neighborhood` (216) | 5.8 Neighborhood association | 17 cells | HIGH |
| `municipality` (62) | 5.9 City hall | 5 cells | HIGH |
| `educational` (177) | 6.15 Education K-12 (Tier 3) / partial 5.7 Library | 12 cells | MEDIUM — straddles Tier 1+2 / Tier 3 |
| `workplace-organizational` (152) | 5.1 EAE / 5.5 Factory / 5.4 Distribution / 5.6 Industry assoc | 13 cells | LOW-MEDIUM — covers all of 5.1-5.6 generically |
| `industry-sector` (25) | 5.6 Industry association | 2 cells | MEDIUM |
| `community` (211) | Mixed — 5.2 Grocery coop, 5.3 Farm CSA, 5.8 Neighborhood | 18 cells | LOW — generic |

**LOW-confidence flag:** `workplace-organizational` is a single bucket in the value-scanner corpus that vision-spec Section 5 splits into five distinct Tier 1+2 archetypes (EAE, grocery coop, farm-CSA, distribution center, factory). The scenarios within this bucket may or may not differentiate. Sprint 5 authoring should disambiguate when extending coverage.

**LOW-confidence flag:** `community` is similarly generic and could land in any of 5.2, 5.3, 5.8. The scenarios are likely written at substrate-property level (mutual aid, food cooperatives, etc.) rather than at specific-collective-archetype level.

### 5.3 Tier 3 (Section 6) — direct cell coverage

| Value-scanner context | Best vision-spec match | Coverage | Confidence |
|---|---|---|---|
| `healthcare-medical` (26) | 6.9 Health and human services | 2 cells (caregiver, person-with-disabilities) | HIGH |
| `ecological-bioregional` (180) | 6.17 Natural collectives (bioregion / biodiversity) | 17 cells | HIGH — direct |
| `educational` (177) | 6.15 Education K-12 (also 5.7 Library — see above) | 12 cells | MEDIUM |
| `industry-sector` (25) | 6.13 Mineral-rights + industrial / 6.14 Logistics freight | 2 cells | LOW — may not match |

### 5.4 Substrate currents (cross-cutting cross-reference)

The vision spec describes three substrate currents (Story, Value, Governance). Mapping value-scanner contexts onto the currents:

- **Story** — `individual` (222), `family` (230), `household` (180), `educational` (177): identity, narrative, recognition, learning. These are the **inward-facing scenarios** where the scanner generates attestation and care-ledger entries.
- **Value** — `workplace-organizational` (152), `community` (211), `industry-sector` (25): REA flows, mutual aid, commitments, restitution. These are the **economic-action scenarios** where care couples to monetary REA.
- **Governance** — `neighborhood` (216), `municipality` (62), `ecological-bioregional` (180): rubric, councils, friction-gradient, peer mediation. These are the **collective-decision scenarios** where multiple commons-elohims convene.

This is a clean alignment, defensible by inspection of the sampled scenarios. The value-scanner corpus is *substrate-currents-shaped* even though it predates the spec's three-currents framing.

### 5.5 Imago Dei discriminator (Section 1.5) — implicit coverage

The IDD-community and person-with-disabilities archetypes carry the Imago Dei discriminator most explicitly (sample 4.6, 4.7 above). The vulnerable-temporary archetype carries it in its temporal expression (the substrate adjusting during illness/stress). The .feature archaeology (Section 4.5 of that doc) flagged Imago Dei as under-articulated — only present as a secondary tag on three .feature files. The value-scanner corpus carries it through *condition-specific archetypes* but does not name it as such. Sprint 5 substrate-floor authoring should cross-reference the value-scanner IDD/disability scenarios as canonical-narrative anchors for Imago Dei feature work.

---

## Section 6 — Connection to .feature archaeology

The .feature archaeology document (`2026-05-22-scenario-archaeology-and-archetype-map.md`) inventoried 76 BDD-shaped scenarios at `genesis/a2o/features/` and identified four Tier-0 archetypes. The value-scanner corpus relates to that archaeology as follows.

### 6.1 Where the value-scanner *fills* archaeology gaps

**T0:household** — the .feature archaeology found 7 primary scenarios and flagged five missing canonical-narrative beats (commons-elohim quiet witness, member-ring as standing, care-economy ledger, Gertrude's cross-household witness, reach-extension by household choice). The value-scanner corpus provides **180 household + 230 family + 222 individual = 632 scenarios** at the household-and-immediate-rings band. Many of these scenarios are exactly the beats the archaeology surfaced as gaps:

- Sample 4.4 (adult × household — Sarah's schedule coordination) → "household runs smoothly through effective coordination" is *the care-economy ledger* beat.
- Sample 4.1 (young-child × family — Tommy in family governance) → "Tommy should experience having voice in family governance" is *the member-ring as standing* beat for the youngest archetype.
- Sample 4.9 (grandparent × family — coordinating across households) → "neither household should dominate the other" is *Gertrude's cross-household witness* beat at the grandparent-cell.

**Connection insight #1:** The value-scanner corpus is a **deep reservoir of T0:household canonical-narrative beats** that Sprint 5 can mine rather than author from scratch. Where the .feature archaeology proposed 5 new household scenarios, the value-scanner contains hundreds of equivalent beats.

### 6.2 Where the value-scanner does *not* fill archaeology gaps

**T0:faith-community** — the .feature archaeology proposed 7 scenarios for plural-eldership, congregation rubric, baptism-as-mastery-attestation, friction-gradient at congregation scale, etc. The value-scanner corpus contains **zero faith-community context**. The contexts that exist (`community`, `neighborhood`) carry general-purpose mutual-aid scenarios but no Restoration-Movement plural-stewardship beats.

**T0:life-group** — the .feature archaeology proposed 7 scenarios for sub-Qahal nesting, partially-derived standing, encrypted prayer attestations, host rotation, life-group cohesion threshold. The value-scanner corpus has **zero life-group context**. The closest beat is "small-group sharing" within `community` cells, but it does not carry the holonic-nesting architectural test.

**T0:wisdom-commons** — the .feature archaeology found 3 secondary scenarios and proposed 7 new ones for peer-council convening, witness-not-verdict, reconciliation as REA event, federation autonomy. The value-scanner corpus has **zero wisdom-commons context**. No filename pattern corresponds.

**Connection insight #2:** The Tier-0 gap surfaced by .feature archaeology is **structurally not filled** by the value-scanner corpus. Faith-community, life-group, and wisdom-commons are *categorically absent* from value-scanner filenames. These three Tier-0 archetypes need **new scenario authoring** (.feature or value-scanner-shaped or both); they cannot be harvested from existing corpus.

### 6.3 Architectural pattern visible across both corpora

**Connection insight #3:** The .feature archaeology corpus and the value-scanner corpus operate at **different abstraction layers** that the spec assumes work together:

- The **.feature corpus is substrate-mechanism-shaped** — auth-lifecycle, recovery-quorum, content-addressing, freeze-floor, SSR-capability, doorway-handoff. It tests *that the substrate works*.
- The **value-scanner corpus is lived-narrative-shaped** — Tommy scanning strawberries, Sarah coordinating schedules, Maria needing childcare. It tests *what the substrate is for*.

Both are essential. The value-scanner does not replace the .feature corpus; it grounds it in human experience. The .feature corpus does not replace the value-scanner; it verifies the protocol can carry the load the value-scanner describes. Sprint 5 (which the roadmap names as "Genesis content + canonical templates + a2o scenarios") should produce *both shapes* of artifact for each new Tier-0 archetype: value-scanner-shaped canonical-narrative beats (the lived experience) + .feature-shaped substrate-mechanism tests (the verifiable claims).

The Sprint 5 authoring task therefore divides cleanly:
1. **For T0:faith-community** — author both value-scanner-shape beats (plural eldership in a worship service, congregation rubric being ratified, baptism observed, etc.) AND .feature-shape substrate tests (`congregation-rubric.feature`, `plural-eldership.feature`, etc.).
2. **For T0:life-group** — author both shapes; value-scanner beats anchor holonic-nesting lived experience, .feature tests verify partial-standing derivation works mechanically.
3. **For T0:wisdom-commons** — author both shapes; value-scanner beats anchor peer-federation lived experience (Brother Cal writing to the Arkansas congregation), .feature tests verify horizontal-no-hierarchy substrate guarantee.

---

## Section 7 — Reorganization proposal

The .feature archaeology (Section 5 of that doc) proposes a new taxonomy under `genesis/a2o/features/`:
```
archetypes/{household, faith-community, life-group, wisdom-commons}/
cross-cutting/{reach, standing, attestation, friction-gradient, commons-elohim, rea-flow, imago-dei}/
infrastructure/{recovery, doorway, p2p, delivery, ssr, browser, deployment, resilience}/
```

The value-scanner corpus is **already structured by archetype × context** at the filename level. The question is whether it needs to MOVE into that taxonomy, get an INDEX pointing at it, or stay-and-connect.

### 7.1 Option analysis

**Option (a) Index-only — recommended.** Keep all 1,681 `.json` scenarios at `/projects/elohim/genesis/data/lamad/content/`. Keep the 19 archetype directories with README + epic.md at `/projects/elohim/genesis/docs/content/elohim-protocol/value_scanner/`. Produce a single **archetype × context index document** that maps every cell to its scenario filenames + counts, plus cross-references to vision-spec catalog sections.

Rationale:
- **The 1,681 `.json` files are generator-produced seed-data**, not story-harvested .feature files. They live in `lamad/content/` because they *are* lamad content — the lamad pillar imports them, seeds the DHT, and surfaces them through the renderer pipeline. Moving them to `a2o/features/` would break that pipeline.
- **The `.json` corpus and `.feature` corpus serve different consumers**. The `.json` corpus is consumed by the elohim-import → seed-data pipeline (lamad pillar, runtime content). The `.feature` corpus is consumed by Cucumber/Playwright (verification, CI). Mixing them confuses substrate purpose.
- **The READMEs at `value_scanner/<archetype>/README.md` are gospel-tier framing** for each archetype's developmental stage and governance scope. They belong in `docs/content/` next to the epic, not in `a2o/features/`.

**Option (b) Reorganize — not recommended.** Moving 1,681 .json files into `a2o/features/archetypes/{household, faith-community, ...}/` would require restructuring the seed-data pipeline. The filename pattern carries archetype × context already; a directory restructure would duplicate that signal without adding clarity.

**Option (c) Connect-only — partially recommended (combine with a).** Cross-references in the spec's catalog stubs (Sections 4-6) pointing at the value-scanner cells. This is a small lift and high payoff: a reader of "5.8 Neighborhood association" would see "lived-narrative coverage: value-scanner `neighborhood` cells (216 scenarios; see `/projects/elohim/genesis/docs/plans/2026-05-22-value-scanner-content-audit.md` Section 5.2)."

### 7.2 Recommendation

**Combine (a) + (c).** Produce a value-scanner index document (this audit doubles as that document) and add lightweight back-references in the vision-spec catalog stubs. Do not move the `.json` files. Do not move the archetype READMEs.

The Sprint 5 *.feature-authoring* work (faith-community, life-group, wisdom-commons) happens in `a2o/features/archetypes/`. The Sprint 5 *value-scanner-extending* work (if scope permits authoring lived-narrative beats for those same Tier-0 gaps) happens in `docs/content/elohim-protocol/value_scanner/` as new archetype directories or expanded contexts in existing archetypes. Two corpora, one shared archetype vocabulary, cross-linked.

---

## Section 8 — Operator decisions to flag

The operator needs to make the following calls before Sprint 5 authoring begins.

### 8.1 Disposition of the value-scanner corpus

**Decision:** Keep-in-place + index (recommendation 7.2) vs. relocate vs. memorialize.

**Recommended:** Keep-in-place. The corpus is generator-produced seed-data that the lamad pillar's import pipeline depends on. Moving it breaks that pipeline. This audit serves as the index. **Default to this unless there's a specific reason to do otherwise.**

### 8.2 Whether to author Tier-0 value-scanner scenarios for faith-community / life-group / wisdom-commons

**Decision:** Sprint 5 authors *value-scanner-shaped* beats (lived-narrative .json scenarios) for the three Tier-0 gaps in addition to *.feature-shaped* substrate tests — or only .feature.

**Recommended:** Author both shapes per Section 6.3 above. The two corpora are complementary, not competitive. New value-scanner directories would be `value_scanner/faith_community/`, `value_scanner/life_group/`, `value_scanner/wisdom_commons/` with READMEs + scenarios per the same archetype × context shape (though the contexts will differ — `worship_service`, `holonic_nesting`, `peer_federation` rather than `educational` or `municipality`).

### 8.3 Whether to expand value-scanner archetype set

**Decision:** Should the existing 19 archetypes get new context cells (e.g., `child × workplace-organizational` for child labor / family business; `young-child × neighborhood` for stroller-rolling-supervised first-scan)?

**Recommended:** Defer to Sprint 6+. The current 19 × 11 matrix is already richly populated where it should be. Adding new cells should be driven by specific narrative needs from canonical-narrative authoring, not by completionism.

### 8.4 Naming continuity in value-scanner scenarios vs Section 4 canonical narratives

**Decision:** The value-scanner corpus uses Tommy, Emma, Sarah, Maria, Jasmine, Margaret, Robert, David, James, Jordan, Alex as character names. Vision-spec Section 4 uses Matthew, Jessica, James, Sheila/Susan, Gertrude Dowell, Brother Cal, Hardins, Lees, Robertsons, Kim family. **The two character sets do not currently overlap.**

**Recommended:** Treat this as a deliberate naming-namespace separation. The Parker family (value-scanner) is the *generic protocol participant* — the demo family that the epic uses for general illustration. The Dowell family (Section 4 canonical) is the *operator's actual household* — the specific lived household the alpha cluster seeds from. Do not unify the namespaces; the dual frame is useful (Parker = "what the protocol does for anyone"; Dowell = "what the protocol does for us").

### 8.5 Integration with lamad rendering pipeline

**Decision:** The `.json` scenarios use `contentFormat: "markdown"` and embed Gherkin as the `content` field body. The lamad pillar's renderer map should render these as either (a) markdown (the current path; renders Gherkin as code-block-like text), (b) a new `scenario-gherkin` content format with syntax highlighting, or (c) a new `value-scanner-scenario` interactive viewer that surfaces the agent-negotiation pattern.

**Recommended:** Path (b) for Sprint 5. Path (c) is a Sprint 8+ aspiration tied to the convergent-Qahal-homepage work; it's the lived-protocol-rendered surface the vision spec gestures at but does not require for MVP. Path (b) requires a small lamad manifest update + Angular renderer; it makes the scenarios *readable as scenarios* rather than rendered as opaque markdown.

### 8.6 Cross-corpus tag standardization

**Decision:** The .feature archaeology proposes `@archetype:*`, `@cc:*`, `@inf:*`, `@tier:*` tags (Section 5.4 of that doc). The value-scanner `.json` files already carry tags (`epic:value_scanner`, `user_type:adult`, `governance_layer:household`, etc.). **Should these tag taxonomies be unified across both corpora?**

**Recommended:** Yes, lightweight unification. Map `user_type:adult` → `@archetype:value-scanner:adult` and `governance_layer:household` → `@cc:household` (or to a new `@vs-context:*` namespace). The lift is small (a tag-rename pass in the scenario generator). The payoff is that the converging Qahal-homepage can surface scenarios from either corpus under unified facets.

---

## Closing notes

The value-scanner corpus is not a finished story; it is a substrate-shaped catalog of 1,681 worked moments of household-rooted protocol participation. It pairs with the `.feature` corpus the way the canonical Section 4 narratives pair with the architectural framing in Sections 1-3: lived experience grounds and disciplines abstraction.

The most actionable finding of this audit is **the Tier-0 gap is not filled by the value-scanner**. Faith-community, life-group, and wisdom-commons are categorically absent. Sprint 5 should author into both corpora: `.feature` files for substrate-mechanism verification AND value-scanner-shaped `.json` scenarios for lived-narrative grounding, sharing the same archetype vocabulary. The household coverage (632 family/individual/household scenarios) is the model the three gap-archetypes should emulate in shape and depth.

The second most actionable finding is **the corpus is already correctly archetype-organized via filename pattern**. It does not need to move. It needs an index (this document) and lightweight cross-references in the spec's catalog stubs. Build on what exists; don't restructure for restructure's sake.

The third most actionable finding is **the Parker family and the Dowell family inhabit different namespaces by design** — the protocol's general-illustration corpus and the operator's actual-household corpus. Both should land; neither should subsume the other.

---

*Audit complete. Operator review of Section 8 decisions before Sprint 5 authoring begins.*
