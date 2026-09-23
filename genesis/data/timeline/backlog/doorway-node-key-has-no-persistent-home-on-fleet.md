---
id: "backlog-doorway-node-key-has-no-persistent-home-on-fleet"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway's Ed25519 node key can now persist across boots (story 5.1), but the fleet manifest gives it nowhere to live"
slug: "doorway-node-key-has-no-persistent-home-on-fleet"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [doorway, manifest, node-identity, tls, operator-owned]
---

**The fact.** Commit `a9910d11c` (story 5.1, 2026-09-20) landed `DOORWAY_NODE_KEY_FILE`: set it and the
doorway loads or generates+persists a 32-byte Ed25519 key with first-writer-wins semantics; unset, it
stays ephemeral as before. `genesis/orchestrator/manifests/doorway/alpha.yaml` sets no
`DOORWAY_NODE_KEY_FILE` env var anywhere (confirmed by full-file grep), and the doorway container's only
declared volume is `ssr-render-bundle` (`emptyDir: {}`, `:508-510`), used for the SSR bundle — not for a
key. There is no volume of any kind, ephemeral or persistent, for a node key on the deployed manifest.

**Evidence.** `genesis/orchestrator/manifests/doorway/alpha.yaml` (no `DOORWAY_NODE_KEY_FILE` match;
`:508-510` the sole `volumes:` stanza for the doorway pod). Precedent for a small persistent volume
already in the tree: `genesis/orchestrator/manifests/ci-infra/doorway-pkarr-resolver.yaml:17-42` — a `doorway-pkarr-cache`
`PersistentVolumeClaim` (`ReadWriteOnce`, 1Gi, `openebs-jiva-csi-default`) mounted at
`/var/lib/doorway/pkarr`, not currently referenced by any Jenkinsfile/groovy/CI dispatch script
(confirmed by repo-wide grep 2026-09-20). Precedent for a mint-once Secret already in the tree: `JWT_SECRET`
(`alpha.yaml:346-350`, `secretKeyRef` into `elohim-doorway-alpha-secrets`).

**Why it matters.** Story 5.1's persistence mechanism is a precondition for story 5.3 (a shared-name
certificate proven through the beacon's owner-stamped TXT record) and for ever flipping
`DOORWAY_JWT_SIGN_ALG` to `eddsa` — both need a stable node identity across restarts. Without a manifest
change, the fleet doorway still mints a fresh key every boot even though the code path to avoid that now
exists. This is operator-owned manifest work, not a code gap.

**Smallest next step.** Pick one of the two precedent shapes: a small PVC (mirroring
`doorway-pkarr-cache`, sized for one 32-byte file) or a mint-once k8s Secret (mirroring `JWT_SECRET`,
minted once and read by the pod at boot). Note for whichever is chosen: a rolling deploy briefly runs two
pods, and the key file's first-writer-wins publish (`a9910d11c`) already covers a shared volume correctly
— a Secret needs the mint-once step done outside the pod's own boot path so both replicas read the same
value instead of racing to mint one each.

**2026-09-22 — repo-side cure written, NOT yet measured on the fleet.** The PVC shape was chosen because
it reuses the generate-on-boot implementation that already landed in `a9910d11c` without introducing a
secret-provisioning path. The stored bytes are a PRIVATE Ed25519 signing key, so the thing to avoid is
committing private key material to this repo — not the Secret mechanism itself: an **externally
provisioned** Secret (minted outside the repo, injected by the operator or a secret manager) remains a
legitimate future option, and is the right shape if several pods ever have to share one identity. Each of
the four singleton doorways now declares its OWN `ReadWriteOnce` / `openebs-hostpath` / 64Mi claim
(`elohim-doorway-{alpha,alpha-b,staging,prod}-node-key`), mounted at `/var/lib/doorway/node` with
`DOORWAY_NODE_KEY_FILE=/var/lib/doorway/node/doorway-node.key` and `fsGroup: 1000` (the image runs as
`USER doorway`, uid/gid 1000) — the same node-local class and `fsGroup` shape the doorway's own mongodb
archive already uses. Doorway-A and doorway-B hold DIFFERENT claims on purpose: they validate each
other's JWKS, so one shared key would make that verification vacuous. Each Deployment now pins
`RollingUpdate maxUnavailable: 0 / maxSurge: 1` explicitly (at `replicas: 1` the defaults already
computed to this) — that is the invariant the ROLLOUT path rests on: `node_identity::load_or_generate`
exits(1) on an unwritable or corrupt file, so a bad mount surfaces as a Pending-or-NOT-Ready surge pod
and a red `waitForRolloutWithEvidence` while the healthy outgoing pod keeps serving; readiness is
`/health`, which never consults the key. It says nothing about eviction, node loss, or a pod that turns
Ready and fails later. Note also what constrains placement: the affinity pins a node CLASS
(`node-type=operations` / `remote`), and it is the BOUND PV that supplies the specific-node constraint —
a bound PV whose node conflicts with the required affinity, or a node without room for a second pod, is
what would leave a rollout stalled. `staging-read.yaml` is deliberately excluded to preserve the
placement flexibility its two soft-affinity replicas are given, and to avoid adding an identity
dependency a `PROJECTION_WRITER=false` reader does not need — the reason is written into that file so
the omission is not read as an oversight.
**What would close this — scoped to alpha A + B:** both alpha doorways logging `Doorway node identity
resolved (persisted)` with `mode: generated` on the first roll and `mode: loaded` with the SAME
`fingerprint` on the next. That is the reading the `dev` branch's edge deploy produces (it is where
2026-09-21's `generated-ephemeral` on every doorway boot was read — habit `doorway-failover` delta
09-21c). Staging and prod carry the same change but deploy only from their own branches, so their
equivalent reading comes with the next `staging` / `main` edge deploy and is NOT claimed here.
Unverifiable from the dev environment either way — it needs an edge deploy.
**Complementary work, still missing:** no a2o scenario asserts this. The household read it BY HAND on
2026-09-20 (habit `doorway-failover` delta 09-20b), which is evidence of the code path, not a standing
check. The owed node is a scenario that restarts a doorway and asserts `mode: loaded` with the same
fingerprint across the restart — the check a `checks:` line could name.

**Links.** Landed code: commit `a9910d11c`. Plan:
`genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md` story
5.1 → 5.3. Habit: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (retire-path).
