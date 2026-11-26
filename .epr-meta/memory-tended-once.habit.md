---
epr-habit-version: 1
id: memory-tended-once
invariant: >
  A memory space shared by several devices is tended once per head. The device that tends it
  records an attestation over what it read, and each other holder either accepts that tending
  in place of its own or says why it did not. The count that says a tending is due is derived
  from records every holder has, so it reads the same on each of them at the same head.
status: unwired
active: false
checks: []
refs:
  - "planned check: a2o @concern:memory-tended-once (genesis/a2o/features/devflow/, not yet written; built from the probes of stations GS-2 and GS-3 in genesis/docs/superpowers/plans/2026-11-26-governed-space-collective-memory-plan.md)"
  - "design: genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md sections 9 and 14"
retire-when: >
  when the governed-space shape serves a second kind of content beside memory and this invariant
  is restated for spaces in general, under a habit that names no one kind of content.
---
DELTA 2026-11-26: declared unwired and inactive on the steward's choice of this date (session 96b46344). No evidence yet: the tending attestation, the closure it is keyed on and the shared pressure count are not built. Today a memory ceremony is run separately on each of the steward's three devices.
DELTA 2026-11-26b: status stays unwired. On this device only, session 0a0687e8 tended placement pressure from 63 to 36 (6594c3aa8): 21 CLAIMED-ONLY docs cleared on gate attestations, CI #217 and direct runs; index fold lag 109 to 0. It recorded no tending attestation, so the steward's other two devices have nothing to accept and would tend again. That is the gap this habit names. The stasis composite stays at 0.844 because `traceability` is 0 by construction: no doc carries `traces:` yet.
