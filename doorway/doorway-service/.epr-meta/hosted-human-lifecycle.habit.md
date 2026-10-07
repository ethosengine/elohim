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
status: green
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
DELTA 2026-09-11d (RED -> GREEN on measured evidence): run 20260911T065537Z-d48f3b69 — 05-leaving.feature under
`just test mesh-browser`, **6/6 scenarios, 70/70 steps, EXIT=0**, on the household mesh at d48f3b690 (doorway
eff30a245 build, storage fb4d10c7d build, REAL hosted provisioning: HAPP_BUNDLE_PATH + POOL_COMPUTE_* wired,
dev_mode singleton path retired 60fb28a39, canonical uhCAk keys 36b0e053a/b870af164, closed identifiers re-register
eff30a245, RFC3339 timestamps, account page survives a bad instant dfd2c8932, portal opened before sign-in 60c3b4a36).
Every clause of the invariant walked from the UI: own display name, own cell on a pool conductor, close leaves no
session, no cell, no active row; closing twice harmless. Earlier same-day runs: 4/6 (crashed render), 5/6 (story
gap), 6/6-with-RAM-shed — all named and cured. The notarized-promise extension (07-hosted-by-a-household,
doorway-humans-served) is NOT this invariant: it moved to its own habit `hosted-cell-promised` (born red,
convergence-window + conductor-contention causes named there). Fleet confirmation: the next edge+app deploy carrying
these commits; the flip authority is the household lane per the check.

DELTA 2026-09-11c (REAL HOSTING MEASURED; RED with ONE named cause left): mesh at faca0d95e, doorway 3c7ee8d89, cold
recast 04:02 (hc-mesh wired HAPP_BUNDLE_PATH + POOL_COMPUTE_* + ELOHIM_COMPUTE_LOCAL_API, 163b3e1eb; archive wiped on
cold start, bceeb0424; hosted cast allow-listed to 14 lane personas, 5cf5f8f38). Prologue: 13 hosted registrants +
3 prologue-hosted-* with DISTINCT canonical uhCAk keys and POPULATED hostedCellGrantCid; humansServed=13 on A, 0 on B.
Lanes (04:12–04:15): 07 mesh 0/3, 07 mesh-browser 0/2, humans-served mesh 1/4 + browser 0/1, 05-leaving browser 4/6.
THE CAUSE: every hosted red is `GET /api/v1/commitments/<cid>` on jessica's storage (not the pool) → 404 — the
commitment is minted (cid in the roster) but not readable off the provider peer: the route serves the local projection
and post_commit signals are cell-local, so a non-authoring peer never projects it until a fetch by cid. Secondary:
hostedByHousehold null on the account page (resolver returns None — diagnosing); re-register of a CLOSED identifier
answers exists with no grant (grant leg runs on first registration only); 05-leaving: agency pipeline has no
"Hosted" step label. Measured cost: 786 MB conductor heap per hosted human (scale-risk row 7); 13 humans = 10.0 GB.
Reports: genesis/a2o/reports/sprint-report-household-20260911T041{2,3,5}*-faca0d95.json.

DELTA 2026-09-11b (FIRST HOUSEHOLD MEASURE of the S1+S2 work — RED with a NAMED cause; run 20260911T0240{31,53}Z /
024102Z / 024112Z-b6c7947b, mesh at e2dcabebc+, imagodei coordinator repacked, integrity wasm untouched):
@concern:hosted-compute-contracted 0/3 (mesh) + 0/2 (mesh-browser) — every scenario fails at registration with
`503 PROVISIONING_FAILED … install app on conductor-0 … NotFound /app/elohim.happ`; Prologue seed-humans and the
three prologue-hosted-* casts fail the same way (agentPubKey `-`, grant cid `-`). CAUSE: the household mesh never
provisioned a hosted human — until 60fb28a39 dev_mode routed every registration to the singleton-Human recovery (one
shared key), and hc-mesh.sh sets neither HAPP_BUNDLE_PATH (default /app/elohim.happ, a container path) nor
POOL_COMPUTE_URL/TOKEN/PERFORMER. The cure exposed the gap; the mesh was relying on the bug. Wiring in flight.
@concern:humans-served 3/4 — humansServed is LIVE (present, `0` not `—`, sibling isolation holds); the close
scenario has no live registrant to close. 05-leaving measured 0 scenarios under `mesh` (file-level @browser-only;
rerun under mesh-browser). Reports: genesis/a2o/reports/sprint-report-household-20260911T0240*-b6c7947b.json.

DELTA 2026-09-11 (S1+S2 of plan 2026-09-10-doorway-federation-three-reds-to-green LANDED LOCALLY; stays RED —
nothing measured on a mesh yet). Stories: 07-hosted-by-a-household (@concern:hosted-compute-contracted, 7 blind-reader
rounds, 3× READY, steward binding declared self-asserted) and dataplane/doorway-humans-served (@concern:humans-served,
READY); both attached as checks; glue for all 77 steps defined (279398f0e, 26d0f0d43); 05-leaving's 69 steps intact.
Prologue casts 3 hosted registrants through POST /auth/register (7e0c75130). Doorway: provisioning no longer keys on
dev_mode (60fb28a39 `should_provision`; synthetic fallback Simulacra-only), signal subscriber keys on projection_writer
(f64d8c5bf), POST /auth/close-account (4a7145814), hosted cell notarized as a delegates-compute commitment scope
hosted-cell — issued on register, revoked on close, account-closed self-revocation coordinator-only (b71b6d66e),
humansServed derived from live hosted-cell rows (a33e3876a; backlog doorway-landing-humans-served-source CLOSED).
Storage: hosted-cell scope + provider-side revoke (c9318d121, 9957e6369; gate 3712 tests). doorway-app: close surface
+ hosting strip, eyes-on render (9cf94c6c8). Harness cleanup through the product path (cb5fbeae6, 3d54c89af).
Gates: doorway 1230 tests EXIT=0 ×2, elohim-app 4596, doorway-app 52. OPEN before a flip: hostedByHousehold still
names the arranger (Task 13b in flight), schema codegen, DNA gate for the coordinator change, then the household run
(Task 17) — a run id, not this note.

DELTA 2026-09-04 (DECLARED red — the check exists and measures nothing passing): baseline against https://doorway-alpha.elohim.host — a
fresh POST /auth/register answered with the operator's own Human profile (display name, bio,
affinities), because every deployed doorway runs DEV_MODE=true and the hosted branch skips
provisioning under it, recovering the singleton conductor's existing Human. No self-service
close exists (only an admin soft-delete of the credential row; no cell reclaim, no session
end). The feature file is written and parses (cucumber --dry-run 2026-09-04); every scenario is @wip until the step definitions land, so the check currently reports 0 passed — red on evidence, not intention.

DELTA 2026-10-07 (NO status change; the device-join path is proven on an isolated network, the
hosted path is not): registering a Che workspace through alpha took six things from the person the
system could have done itself (`genesis/docs/superpowers/specs/2026-10-07-one-command-device-join-design.md`),
and alpha's portal sent him through sign-in to `consent_unavailable` because the doorway serves the
page and mounts no `/auth/consent/*`. Built across four lanes and proven live on the i1006 network
on one machine: a new conductor+storage device (generated from the repacked happ, uhCAkFbHc…XU0hn2)
ran `epr device join --label fresh-device-3`; the node that speaks for Matthew (i1006 workspace,
uhCAkRpkc…V5RRzP) ran `epr device approve '<link>' --yes` once ("Done. The device will finish
joining on its own; nothing to copy"); the join collected the consent by its PKCE verifier
(`POST /auth/consent/collect`, the code never shown), checked it, and enrolled: "This device is
enrolled. Joining record: uhCkkAh97F_w5OijD61Wy_EOm0fC-nCFZW3yWOH5Gz-gRBhKIvDR6"; the new device's
standing reads hasIdentity=true. Nothing was carried by hand; no address, port or code was typed on
the new device (`--node` was needed only because three nodes answer on this one machine, which the
discovery refuses to guess). Found and fixed on the way: a terminal on the approving node's own
machine was refused `consent_not_signed_in` on a node nobody had signed in to (dial 1 now governs:
a local caller on a node that holds a person is signed in for agree); an enroll failure lost the
collected consent and needed a second yes (the consent is now kept for its window and the next
`join` enrolls without asking). Found and NOT a code gap: a device whose packed happ carries stale
mishpat/imagodei coordinators fails `sign_device_enrollment` / `get_human_root_evidence` ("zome
function doesn't exist"); and a device with no bootstrap/relay cannot read the identity commitment
("identity commitment unavailable") — the join needs the network, and says what it waits on. Portal
states rendered (genesis/a2o/reports/look/join-refused, join-held). Crate 117, epr-cli 314,
storage focused 52 tests; elohim-imagodei consent specs 223 (29 pre-existing failures in two
untouched files), doorway-app 115, imagodei-portal 87. Read at 16:00Z: the approver's own device list names both devices
(uhCAkRpkc…V5RRzP, uhCAkFbHc…XU0hn2), so the record travelled; the a2o scenarios stay @wip (step definitions
follow); the hosted path (a doorway mounting `/auth/consent/*` and relaying between two Che
workspaces) is the named next slice. Blind-reader loop on device-provisioning-paths.feature: 2
rounds (0/7/2 → 0/5/3, READY). Second cycle on the final bytes (16:22Z, device uhCAkMIUu…, after the second-model
review's fixes): the first enroll came before the new conductor could read the identity
commitment, the consent was kept on the device, and `join` run again with the same flags enrolled
"without asking again" (joining record uhCkkhW8BUGqNfdUoKvqaa_6Dn6kN28NENkPOK27O-fZYfnSp_TsG) — the
recovery path is proven live, one yes total.

DELTA 2026-10-07c (ON THE FLEET; NO status change): pushed e87ee7c69 (operator-granted, pre-push bypassed because
the household could not mint the serving receipt — see dataplane-reanchor-dead-remaining-rekeyed-peer.md
2026-10-07); orchestrator #1996 dispatched eprfs #49 (SUCCESS) and edge #1572 (SUCCESS, Dataplane Validation
owed to the sibling run). Alpha doorway restarted on the new image and, rendered headless against the
morning's own approval link, now answers "Can't approve here" with the `epr device approve '<link>'`
command BEFORE sign-in (probe POST /auth/consent/view -> 404, no redirect to /threshold/login;
genesis/a2o/reports/look/alpha-consent-after-roll). The hosted path (a doorway that takes approvals) is
still the named next slice; the household receipt is owed on this tip.

DELTA 2026-10-07d (receipt minted; NO status change): the serving receipt owed on the morning's push is
minted on 683fb16a7 — `sprint-report-household-20261007T232705Z-683fb16a` (epr-app-deliverability 5/5
scenarios, 102 steps) on a clean recast household whose prologue ran every leg green (`PROLOGUE_EXIT=0`),
after the dead-anchor Held wedge and the identity seeder's self-conflict were fixed the same evening
(operator ruling: fix what you find). The pre-push T2 leg reads it: "every epr-app-deliverability.feature
station passed on current source".
