---
name: project-jenkins-granted-peer-lane
title: "Jenkins granted peer lane — rulings 2026-11-02"
description: Jenkins runs owned-substrate a2o features on adam under a weekly steward-approved grant; /proc features labelled then rewritten; verdicts count measured scenarios only
metadata:
  node_type: memory
  title: Jenkins granted peer lane — rulings 2026-11-02
  type: project
  originSessionId: 8f5e32ab-ad2f-4f78-9aaa-b08642f0770a
  modified: 2026-11-02T15:31:12.096Z
---

Goal (steward human:matthew, 2026-11-02): "jenkins can run a peer with device grant.. so that those features can run and report honestly", and local-mesh results must equal what Jenkins shows (truth in reporting).

Rulings given 2026-11-02 (AskUserQuestion answers):
- /proc-reading or process-controlling features: NOT labelled (the interim label was withdrawn the same day: "I don't want to label stuff, I want to really resolve that what jenkins is doing is exercising the real mesh.. per the scripts"). Ruling: provider-native execution — "jenkins watching, adam execute a stage natively". adam's worker runs the same `just test mesh <feature>` from its own pinned checkout as the mesh owner, for an allowlisted feature Jenkins names; Jenkins sends no code and verifies the returned report/receipt. Giving the runner's guest payload (UID 65534) owner rights stays excluded.
- Both shapes are supported (steward, 2026-11-02): adam-native execution first, as a step toward Jenkins retiring to rakia (peer compute subsuming CI); Jenkins as a granted peer/device of human:matthew alongside. Either way Jenkins reports REAL owned-mesh results.
- Peer-stage verdict: keep pass|fail|skip, computed over measured scenarios only; held scenarios get no brit ref and are reported "held, not measured" beside it; the inventory compares against the profile-selected scenarios (so @wip/@browser no longer refuse a feature).
- Grant G2 (adam -> Jenkins runner, measure-stage, commons, 4/h, rotation 7 d) is renewed weekly by the steward; read the current validUntil in genesis/orchestrator/data/adam-compute-worker.json (2026-11-23T16:00Z as of origin/dev 2026-11-20) rather than trusting a date written here. There is no automatic renewal; an expired declaration refuses the adam render and worker boot. Renewal is a weekly steward approval; never extend validUntil or rotation_ttl_days on the agent's own authority.

**Why:** the A2O report showed 3/151 features because the cucumber plugin counted skipped as failed, and ~510 owned-substrate scenarios can only be measured on a mesh the run owns.

**How to apply:** before the next expiry, ask the steward for the replacement window (only the approval is theirs; the agent edits genesis/orchestrator/data/adam-compute-worker.json and operations.md). Delegable today: only peer-mesh.feature and app-delivery-refuses-fast; check `build-stage-task.mjs check-delegable` before proposing features. Any change under the source-under-test trees (genesis/a2o, scripts/ci, sdk, storage, doorway, DNA, lockfile) needs a fresh `[compute-worker:build]` image and re-activation before the peer stage runs again. Full grounding: genesis/local-dev/landing-queue-2026-10-31/peer-lane-plan.md. Related: [[project_ci_build_infra]], [[feedback_human_loop_not_terminal_authority]].
