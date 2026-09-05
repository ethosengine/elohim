---
epr-habit-version: 1
id: hosted-human-lifecycle
invariant: >
  A hosted human's account at a doorway is their own from the portal forward and gone when
  they close it: registering through the portal yields the display name they typed and a
  cell of their own on a pool conductor (never a shared one), and closing through the portal
  leaves no session, no cell on any pool conductor, and no active account row — so the
  whole hosted stage can be walked end to end from the UI and leave the deployment as it
  was found.
status: red
active: false
checks:
  - "a2o @concern:hosted-human-lifecycle (genesis/a2o/features/auth/hosted-human/05-leaving.feature — @browser-only @act:i; authority is the household lane: `just test mesh features/auth/hosted-human/05-leaving.feature`; on a deployed doorway run with ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml A2O_ALLOW_DESTRUCTIVE=0 — the story creates and removes its own human, so it is safe on a shared fleet)"
first_move: >
  Land the step definitions and the close-account route so the @wip scenarios execute and
  this habit measures red on a real run, then move the register/login provisioning
  gate off `dev_mode` so a registrant gets their own cell (the red that the 2026-09-04
  baseline observed: a fresh registration on alpha answered with the operator's profile).
refs:
  - "genesis/docs/superpowers/plans/2026-09-04-hosted-human-lifecycle-e2e-plan.md (the plan; P2P gate output in §2)"
  - "genesis/a2o/features/browser/doorway-portal-login.feature (the sign-in station this story reuses)"
  - "genesis/a2o/features/auth/agency-pipeline-coherence.feature (the account-page agency pipeline this story asserts at 'Hosted')"
  - "doorway/doorway-service/src/routes/auth_routes.rs (register/login; provisioning gated on dev_mode today)"
  - "doorway/doorway-service/src/conductor/provisioner.rs (provision_agent / deprovision_agent — the reclaim primitive already exists)"
retire-when: >
  when account closure is a notarized, peer-witnessed governance action whose effect on the
  hosting doorway is reconciled by the substrate (the doorway reads "closed" from the DHT and
  reclaims by construction) — at that point the doorway cannot keep hosting a closed human,
  and the practice under watch has become a property of the substrate.
---
DELTA 2026-09-04 (DECLARED red — the check exists and measures nothing passing): baseline against https://doorway-alpha.elohim.host — a
fresh POST /auth/register answered with the operator's own Human profile (display name, bio,
affinities), because every deployed doorway runs DEV_MODE=true and the hosted branch skips
provisioning under it, recovering the singleton conductor's existing Human. No self-service
close exists (only an admin soft-delete of the credential row; no cell reclaim, no session
end). The feature file is written and parses (cucumber --dry-run 2026-09-04); every scenario is @wip until the step definitions land, so the check currently reports 0 passed — red on evidence, not intention.
