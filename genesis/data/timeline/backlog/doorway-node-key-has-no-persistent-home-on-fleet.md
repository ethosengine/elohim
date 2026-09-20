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
already in the tree: `genesis/manifests/doorway-pkarr-resolver.yaml:17-42` — a `doorway-pkarr-cache`
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

**Links.** Landed code: commit `a9910d11c`. Plan:
`genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md` story
5.1 → 5.3. Habit: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (retire-path).
