---
epr-habit-version: 1
id: pin-attestation
invariant: >
  A submodule pin move is gated by the pinned commit's own attestation and re-gates
  the pin's direct consumers; the component's suite is never re-run from the monorepo.
  A red, cancelled, pending or absent attestation refuses the pin; an unreachable read
  passes on the floor and says `claimed`.
status: red
active: false
checks:
  - "a2o @concern:pin-attestation (genesis/a2o/features/devflow/pin-attestation.feature — seven scenarios over a scratch superproject with one gitlink and a fake `gh`: green passes, red refuses, absent refuses, still-running refuses, a commit the forge never saw refuses, unreachable passes as claimed, and a pin move selects the one-hop consumer and nothing deeper; default profile: cd genesis/a2o && npx cucumber-js --tags '@concern:pin-attestation')"
  - "node --test genesis/orchestrator/gate-attest.test.mjs genesis/orchestrator/gate-oracle.test.mjs (the four outcomes, latest-run-wins, reads-the-pin-not-the-checkout, depth one)"
  - "epr flow note observations on pin-attestation@1 exist for every pin-moving push (GATE_ORACLE and the attested dispatch are live in gate-runner.mjs)"
guard: >
  Regression risks: (1) a matcher that special-cases gitlinks instead of the manifest
  listing the pin path — both oracles must agree by declaration, not by code;
  (2) widening `gateAttestation` past its three fields (thresholds, signers) before
  rung 2 exists to read them; (3) greening this habit by trusting a green badge
  rather than reading it — `tier=claimed` is a pass, never evidence.
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md"
  - "plan: genesis/docs/superpowers/plans/2026-09-23-submodule-pin-attestation-gate-plan.md"
  - "ladder: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (rung 3 of the component ladder named 2026-09-23)"
  - "oracle: elohim/rakia/docs/specs/2026-04-12-rakia-design.md §4 open question 4 (shadow, then switch)"
retire-when: >
  when attestations are read from the dataplane rather than a forge — rung 2 landing makes
  `github-checks` one provider among peers — and this habit describes a product, not a practice.
---
DELTA 2026-09-23 (oracle flipped; RED preserved until two pin-moving pushes hold): the shadow
selection over the pin-moving commit 691b28cdf printed `[gate] oracle-diff: +elohim-storage
+elohim-app` — exactly the depth-one consumers of the rakia and sophia pins (brit's consumer,
build-edge-image, has no gate project) — and over the full push range vs origin/dev the two
oracles agreed (no line). GATE_ORACLE now defaults to rakia. Evidence for the line was taken
by running the same selection the hook runs over that commit's file list before the push
(the push itself bypasses the hook on this host). The habit flips green when two further
pin-moving pushes select by the oracle with no override and every pin read is tier=witnessed.
DELTA 2026-09-23 (rung 3 landed in shadow mode; RED preserved): schema lifted and widened
(rakia 2b2cedb), brit/rakia/sophia declare attested gates with the pin as the step input
(brit 92f5faf, rakia d3329b2, sophia 631f7f4 — all three upstream checks concluded success at
those pins), consumer edges on cargo-build-storage and build-edge-image, gate-runner asks
`rakia affected` in shadow mode (GATE_ORACLE=shadow default) and dispatches attested projects
to gate-attest.mjs. Evidence: node --test gate-attest/gate-oracle/gate-runner + orchestrator
suite 301/301 EXIT=0; a2o @concern:pin-attestation 6/6 (46 steps) on the default profile, two
blind-reader passes READY; live: `gate-runner --target rakia` read `test success`,
`--target sophia` read `build success`; a sophia pin path prints `[gate] oracle-diff:
+elohim-app` in shadow mode. Not yet: the oracle flip (commit 4) and a real pin-moving push.
DELTA 2026-09-23 (born RED): spec and plan authored; the codegen gate was found red at the
pinned rakia schema and the two oracles disagree on a bare gitlink path. No landing yet.
