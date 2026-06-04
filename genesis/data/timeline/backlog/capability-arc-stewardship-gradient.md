---
title: "Capability arc — steward↔stewardee gradient, decline, death; unexpected death as separate intervention"
created: 2026-06-04
domain: "design"
tags: [capability, decline, stewardship, guardianship, death, recovery, imagodei, mishpat]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - genesis/docs/architecture/cradle-to-grave-capability-gradient.md
---

# Capability arc (person axis, time)

Operator framing (2026-06-04): a whole design arc for capability decline — from
Gertrude (remote grandparent, edge of full agency, still independent) through the
in-laws-ADU/spare-bedroom grandparent, declining capacity, medical stewardship
(power-of-attorney-shaped delegation), to death — ideally the **full range of
steward↔stewardee scenarios**. These are **person-axis stories, NOT dwelling
stories**: they follow the person wherever they dwell; the dwelling class only
modulates context (who is co-present; whether an institution is a party).

**Unexpected death is modeled SEPARATELY** — an intervention, not a gradient
endpoint. The recovery spec already structurally separates planned/graduated paths
(IntimateQuorum, StewardshipGrant) from `NetworkWitness { purpose: Dissolution }`
(deliberately reserved Phase 2b stub: "bereavement care"). Honor that separation.

**Composes from:** graduated recovery authority (5 layers; 2 implemented, 3 stubbed);
StewardshipGrant/DevicePolicy/PolicyInheritance; the rea-compute §5 guardianship row
(capacity-conditional bounds — uninstantiated); the formation spec's sponsor+grant
shape (the SAME mechanism that bounds a child's entry unwinds for an aging parent —
one mechanism, both ends of life; supersession chain, never reset).

**Missing:** capacity-conditional bounds validation (no validator enforces capacity
checks); authority-transfer choreography on decline; temporary incapacity handoff
without key rotation; estate/succession mechanics (Dissolution stub only); the
value-scanner personas (Helen/dementia, James-caregiver/David, Jasmine teen, Aisha
young-adult) have narrative but no substrate instantiation; the ADU/co-located-elder
persona slot is unnamed (between gertrude-remote and Helen-in-care) — add to the
persona roster when this work starts.
