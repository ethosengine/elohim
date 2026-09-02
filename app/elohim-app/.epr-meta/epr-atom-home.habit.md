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
  - "a2o @concern:epr-atom-home (genesis/a2o/features/content/epr-atom-home.feature — @browser-only @act:i; 7 frame scenarios live, 3 commons scenarios @wip for the commons plan; runnable locally against the shell on alpha data: cd app/elohim-app && pnpm start:alpha, then cd genesis/a2o && ELOHIM_CAP_OWNED_SUBSTRATE_STATUS=available E2E_DEVICE_MODE=playwright E2E_APP_URL=http://localhost:4200 E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host npx cucumber-js --tags '@concern:epr-atom-home and not @wip')"
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
DELTA 2026-09-02b (LOCAL PROOF, stays RED until an app deploy renders it on alpha):
Slice 1 landed on dev (fb0117114 … cc9cbe385, plus the a2o retargets in flight):
EprHomeComponent owns /epr/{id}; EprFocalComponent extracted count-neutral (the
import ratchet's content-viewer edge went 2 → 1); the four legs, the designed gate,
arrival from the nav stack, "Your mark", "Open in Lamad" from generated all-bundle
claims, title + nav-stack self-recording, a claimed card for lens atoms. a2o
@concern:epr-atom-home: 7 passed / 0 failed against the local shell on live alpha
data (two consecutive runs; b8b30686a carries the tail). `just gate elohim-app`
green (AOT build + 4656 tests). Renders: genesis/a2o/reports/look/epr-home-{t4b,
t6c-path,light,phone,t3-gate}. The wave-2 app build (elohim/dev #1682) died on an
agent-pod flake before deploying; the flip to green needs a fleet render — a build
number, not this note.
---
DELTA 2026-09-02: declared RED on measured violation — `pnpm look https://alpha.elohim.host/epr/evolution-of-trust`
(genesis/a2o/reports/look/epr-evolution-of-trust) renders lamad's ContentViewer at the universal address:
viewer-back-home present, affinity/mastery controls above the content, tabs hiding the legs, and
/epr/concept-bidirectional-trust renders the full learner chrome for a node no peer holds. The a2o
concern is written (10 scenarios, all @wip); the shell component and step defs are Slice 1 of the spec.
