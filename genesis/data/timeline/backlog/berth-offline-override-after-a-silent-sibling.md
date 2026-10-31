---
id: "backlog-berth-offline-override-after-a-silent-sibling"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "berth --offline-ok cannot converge across a silent sibling's return"
slug: "berth-offline-override-after-a-silent-sibling"
written: "2026-10-30"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "berth / steward-scoped leases / pre-push"
domain: "D5"
tags: [berth, leases, offline-ok, power-loss, pre-push]
---

# berth --offline-ok across a silent sibling's return

On 2026-10-30 a power loss at about 19:00Z restarted both of Matthew's workspaces. shem
came back first; ethosengine's device stayed silent for about 35 minutes. Pushing the
integration train needed dev-integration, alpha-fleet and jenkins-dispatch, all
steward-scoped, so each claim refused `unconfirmed: <ethosengine> last seen …`, and the push
was retried with `BERTH_OFFLINE_OK=1`.

| Row | What | State |
|---|---|---|
| 1 | Right after writing the override row, `_settle_steward` re-reads the verdict. The view had not folded the row yet, so the verdict was `pending`, which carries no `device`, and `steward_reason` raised `KeyError`. The claim exited 1, the pre-push released the lease, and every retry wrote a fresh lease and a fresh override: alpha-fleet never converged | fixed by 9a5719006 (pending names its `waiting_on`; test in `berth_test.py`) |
| 2 | A lease that carries an override stays `unconfirmed` until `override_at`, and `override_at` is computed from the sibling's last-seen. Once the sibling is back, every head it sends moves `override_at` 30 minutes later, and its acknowledgements never confirm the lease. Releasing and claiming again without `--offline-ok` was `effective` at once | open: an overridden lease should confirm on acknowledgement like any other once the sibling is heard again |
| 3 | `BERTH_OFFLINE_OK=1` stays set across retries in the pushing session's environment, so a retry made after the sibling returns still takes the override path | open: the hook could drop the flag for a resource whose sibling has a head newer than the override row |

## Done when

- A push that claimed under `--offline-ok` while a sibling was silent completes its claims
  without a manual release once the sibling returns.
