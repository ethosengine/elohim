---
epr-habit-version: 1
id: epr-atom-home
invariant: >
  Every reachable EPR renders at /epr/{id} in one shell-owned frame — arrival where you
  actually came from, identity, a focal render shaped by format, the four legs in household
  words, and the commons around it — with no pillar chrome; an unreachable EPR renders the
  designed gate, never a wall and never the controls of a thing that cannot be seen.
status: red
active: false
checks:
  - "a2o @concern:epr-atom-home (genesis/a2o/features/content/epr-atom-home.feature — @browser-only @act:i, @wip until the shell component and step defs land; runnable locally: cd genesis/a2o && npx cucumber-js --tags '@concern:epr-atom-home')"
  - "pnpm look https://alpha.elohim.host/epr/evolution-of-trust (genesis/a2o) — the shot carries data-testid=epr-home and no viewer-back-home"
first_move: >
  Slice 1 of the spec: extract the renderer host from content-delivery into a shell-owned
  EprFocalComponent (count-neutral under the import ratchet), build EprHomeComponent with the
  frame and the four legs, switch the epr/:resourceId route, and land the step definitions so
  scenarios 1–6 drop @wip and this habit flips unwired → red → green on evidence.
refs:
  - "genesis/docs/superpowers/specs/2026-09-02-epr-atom-home-shell-component-design.md (the frame, the legs, the seams)"
  - "https://claude.ai/code/artifact/50e6f942-e332-43c4-b619-66f0a5d2ccb0 (design canvas, v2 community layer)"
  - "genesis/a2o/reports/look/epr-evolution-of-trust (the 2026-09-02 render of the misrouted surface)"
retire-when: >
  when the doorway's /epr resolver serves the same frame server-side (SSR frame with the same
  legs and gate) so that the shell component and the runtime path are one rendering path — the
  habit then describes the runtime, not an Angular component.
---
DELTA 2026-09-02: declared RED on measured violation — `pnpm look https://alpha.elohim.host/epr/evolution-of-trust`
(genesis/a2o/reports/look/epr-evolution-of-trust) renders lamad's ContentViewer at the universal address:
viewer-back-home present, affinity/mastery controls above the content, tabs hiding the legs, and
/epr/concept-bidirectional-trust renders the full learner chrome for a node no peer holds. The a2o
concern is written (10 scenarios, all @wip); the shell component and step defs are Slice 1 of the spec.
