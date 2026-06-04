---
title: "Household mobility seams — lifecycle choreography + institutional authority regimes"
created: 2026-06-04
domain: "design"
tags: [household, dwelling, mobility, lifecycle, guardianship, institutional, qahal, mishpat]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
---

# Mobility seams (household×dwelling edges over time)

Where are the seams that let a family move location, a member move dwelling-to-dwelling,
and dwelling-situation classes vary (apartments, dorms, ADUs, state-wards, retirement
communities, nursing homes, temporary housing/shelters)? Held thesis on the household
lattice (umbrella §6).

**2026-06-04 discovery verdict: the gap is NOT missing primitives — it is missing
lifecycle choreography.** Membership is already multi-collective (no exclusivity;
Kenji-the-student lives the dorm+family case); guardian-of/ward-of exist in the
relationship vocabulary; the rea-compute §5 guardianship row (act-on-behalf,
age-bounded, capacity-conditional) is the uninstantiated bounds template; revocation
exists for collective membership; NodeRegistration re-registers with household binding.

**Confirmed zero-prior-art seams:** household relocation ceremony (dwelling-binding
update); household split/merge (divorce/separation — atomic membership+asset
redistribution); aging-out (ward→adult capacity transitions; supersession chain, no
reset); institution-as-governance-actor (facility/state as bounded-authority party);
multi-membership precedence/privacy/conflict; concurrent guardianship; temporary
incapacity handoff that is NOT key rotation (power-of-attorney-shaped);
node-hardware replacement preserving household identity ("migration/lineage" named in
CLAUDE.md, designed nowhere).

**Design key:** each dwelling class is an authority-regime TEMPLATE on the
household×dwelling edges — a rubric configuration (umbrella §3), not a new entity.
Affirmation (relationship-given: ADU grandparent) vs graduation (standing-accrued:
new roommate) selects per the rubric.
