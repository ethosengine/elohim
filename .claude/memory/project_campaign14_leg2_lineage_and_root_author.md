---
index: false
name: project_campaign14_leg2_lineage_and_root_author
title: "Campaign 1.4 leg 2 — DNA lineages and the missing grantor"
description: "2026-10-02: Che = live alpha lineage; CI pack and worktree pack differ; no alpha root authored by a non-Che Matthew key, so the delegation has no grantor"
id: project-campaign14-leg2-lineage-and-root-author
metadata:
  node_type: memory
  title: Campaign 1.4 leg 2 — DNA lineages and the missing grantor
  type: project
  originSessionId: 4d194ddf-58e5-469d-8f12-0228dfb11783
  modified: 2026-10-03T00:36:01.515Z
cites:
  - genesis/a2o/scripts/lib/steward-credential.ts
  - genesis/orchestrator/manifests/doorway/alpha.yaml
---

**Three DNA lineages, one of them live (2026-10-02).** Che's five cells carry exactly the live alpha fleet's DNA hashes (lamad `uhC0kZezl4…`, mishpat `uhC0kQwOEwm…`, imagodei `uhC0kRGwtz…`, infrastructure `uhC0k5a385…`, node_registry `uhC0k51dR…`). The DNA pipeline (`elohim-holochain/dev #1480` at `dd1b55ac2`) packs a different lineage on four of five roles (only infrastructure matches) and its DNA-HASH-GUARD passes because it compares to its own committed baseline, not the fleet. A hApp packed from a worktree (`just build` per DNA, then pack) is a third lineage. `POST /admin/coordinators/sync?apply=false` on Che refused every role with `dnaHashMismatch` — correct behaviour; never apply a CI or worktree bundle to Che or the fleet without the reinstall/migration path. Coordinator parity Che↔fleet cannot be read without the doorway Admin selector; Che's coordinators came from `che/deployed-bundles/coordinator-canonical-3746e773e` on 10-01.

**Leg 2 had no grantor.** The closing scenario (`commons-path-steward-publish.feature`, "Che publishes two consecutive updates…", steps in the `fct-hosting-native-reconstruction` worktree) asserts `che !== matthew` and root_author == matthew. On alpha every Matthew-resolved root (35 items incl. `fct-module-01-church-dilemma`) was authored by Che itself; the hosted operator controller (conductor-4, account matthew.operator.dev@alpha) and fixture-matthew (conductor-6) authored none of the 94 roots (`genesis/local-dev/device-ceremony/native-fct-fixtures/canonical-roots-94.pending.json`). Resolved 2026-10-03 by the operator publishing one root from the hosted controller cell (below).

**Credentials that are dev fixtures vs real secrets.** On disk: lair passphrase (launcher literal, `hc-start.sh` ~line 628), operator account password (`device-ceremony/operator/registration-intent.json`), household admin key `mesh-admin-dev-key`, AND the alpha doorway admin keys inline in `genesis/orchestrator/manifests/doorway/alpha.yaml:41` (`dev-elohim-admin-2024`, alpha-B `dev-elohim-admin-b-2024`; apex `elohim.host` is served by alpha-B). Verified live with a WebSocket handshake (101). Hosted sessions expire hourly; re-login with `fleet/operator-login.mjs`.

**Resuming Che in place works from a Claude shell** with `setsid nohup`, passphrase on stdin via `--piped`, file sink (the 10-01 instance's stdout pipe died 19h before the process did), stale `ks/pid_file` removed only when its pid is dead; the auto-mode classifier denies the launch and credential probes until the operator grants them via /permissions. Receipt: `genesis/a2o/reports/recovery/campaign-1.4-restart-20261001/che-resume-20261002T2144Z.json`.

**Doorway↔a2o encoding drift (fixed in worktree, uncommitted).** The chaperone's `/hc/connect` returns `agentPubKey` and `cellIds` as STANDARD base64 (`chaperone.rs`, `BASE64 = STANDARD`); `device-ceremony.ts` canonicalizes them but `refreshHostedCredential` in `genesis/a2o/scripts/lib/steward-credential.ts` (added e9e0b3b63) did not, so `refresh-hosted` could never pass against the real doorway. Fixed by canonicalizing before the identity checks. Also: `AdminWebsocket.getDnaDefinition` from @holochain/client 0.21 fails with "Failed to deserialize request" against both the fork conductor directly and via the proxy — coordinator parity cannot be read that way.

**Device authorization established 2026-10-03 00:16Z; the next gap is projection, not authority.** Operator root `fct-leg2-operator-root` (`uhCkkppgLq…`) created on the hosted controller cell via `fleet/create-operator-root.mts`; owner credential + Che writer credential issued; `grant_head_delegation` operator→Che issued (`fleet/owner-grantor-<expiry>/delegations.json`). What failed to appear: the root row — 404 on Che's storage and both doorways 15 minutes later. Cause by code: a storage projects rows only for writes it performs through its own conductor's native cell, and `POST /db/content` on the doorway writes as the storage node's agent; a *hosted* cell has no storage behind it, so a direct-to-cell create reaches only the DHT, and the content reconcile arm never creates rows from the DHT. The household never hit this because its owner was a native peer with its own storage. Remedies: (a) Che adopts the root under the delegation — `declare_earned_canonical_head` with `adopt_before_author: true` AND `carried_record` (both required; content_store lib.rs ~2806-2870, ~5820) so Che's own storage projects it; (b) give the operator runtime a storage of its own (steward tier), which is the target shape anyway. The auto-mode classifier does not honor `Bash(node *)` allow rules for these identity scripts; every on-chain step needs a /permissions grant.

**Why:** two agents and an hour went into rediscovering these; the lineage split explains why "fix landed but didn't take" questions keep recurring, and the projection gap is the real frontier of leg 2.
**How to apply:** compare DNA hashes against the LIVE fleet, never against a pipeline baseline; for leg 2, start the next round at the adopt-before-author declaration from Che, with Adam's read credential still to be created; see [[project_pipeline_dispatch_ordering]], [[project_steward_agency_publish_from_own_device]], [[feedback_two_portals_only_apps_share_sso]].
