---
id: feedback-shem-down-peers-are-held-not-failed
name: feedback_shem_down_peers_are_held_not_failed
description: When shem is declared down, shem-resident peers (adam, caleb, the non-household humans) being down is EXPECTED — their e2e failures are HELD-state, not bugs; classify by nodeTypes before diagnosing
metadata:
  node_type: memory
  type: feedback
cites:
  - genesis/orchestrator/data/deployments.json
  - genesis/manifests/cluster-state.yaml
---

When diagnosing CI/e2e failures against the alpha cluster, **classify each failing peer/persona by `nodeTypes` in `deployments.json` FIRST.** Household/primary cluster = matthew, jessica, james (`nodeTypes:['operations','edge','performance']`); everything else (adam, caleb, pete, terrance, frank, gertrude, susan, daniel, emma, eve, nancy) is `nodeTypes:['remote']` = **shem-resident**. shem is the remote multi-tenant canvas; when `cluster-state.yaml` declares `shem: available: false`, those remote peers being down is **EXPECTED** — their scenarios are **HELD** (should auto-skip), NOT failures to fix and NOT "raise the memory limits."

**Why:** On the 2026-06-03 overnight e2e shift I read build #1075's 19 failures as "stack-side — raise the 1.5 GiB OOMKill limits." Three of my "blockers" (caleb conductor down, adam genesis-peer seed-wait, james OOMKill) were all the same mistake: treating shem-resident peers as primary-cluster. The topology was already in [[project_shem_is_p2p_live_canvas]]; I just didn't apply it. Most of the 19 were shem peers correctly down (held), masking the true test-layer signal.

**How to apply:** A failing scenario that depends on a `nodeTypes:['remote']` persona, while shem is declared down, is held — skip it in the count and look at the household failures for the real signal. The separation that ENFORCES this (probe→cluster-state reconcile so shem-down → `ELOHIM_REMOTE_COMPUTE_STATUS=unavailable` → remote-persona auto-skip) is the [[project_ci_reconciles_to_substrate_signal]] two-homes principle; if remote scenarios are *failing* instead of *held*, the separation isn't wired (the probe is failing-open on a blind `unknown`) — that's the bug, not the peer being down. Caveat: a household node mis-provisioned (e.g. james lacking a manifest) IS a real issue, but the fix is the manifest/declaration, never a blind limit bump.
