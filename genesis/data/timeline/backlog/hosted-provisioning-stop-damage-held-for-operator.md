---
id: "backlog-hosted-provisioning-stop-damage-held-for-operator"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The hosted-provisioning stop-damage branch is reviewed and gated but held: landing it moves alpha into the `wiped` verdict, which refuses registrations until an operator runs recover"
slug: "hosted-provisioning-stop-damage-held-for-operator"
written: "2026-10-19"
author: "2026-10-19 overnight shift (finish integration-2026-10-18)"
status: "backlog"
priority: "high"
jobs: [elohim-edge]
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - doorway/doorway-service/src/routes/auth_routes.rs
  - genesis/data/timeline/backlog/doorway-apex-only-hosted-provisioning-policy.md
tags: [doorway, hosted, provisioning, security, operator-decision, held]
---

# Hosted provisioning stop-damage: held for the operator

`fix/hosted-provisioning-stop-damage` (the same branch as `proof/recovery-gate`) was rebased onto dev on 2026-10-19 in the worktree `.worktrees/land-hosted-1019`. Its doorway gate passed, but it has not been landed.

| commit | what it holds | gate |
|---|---|---|
| `0a7c3f1a5` | the merge onto dev: an intent log for hosted provisioning, and the store verdict at boot | — |
| `7d46bc6b1` | the takeover guard alone (`auth_routes.rs` only); it cherry-picks cleanly onto dev | 1927 passed, 0 failed |
| `e674f52e8` | codex review findings 2–4, plus a login exemption that checks the provisioning record | 1930 passed, 0 failed |

The first codex review rated the branch UNSAFE with 3 findings. All three are fixed in the commits above.

## Why it is held

Two operator decisions stand between this branch and dev.

1. **Landing the full branch puts the fleet into a state that needs recovery.**
   - At boot, the doorway reads an empty intent log while its conductors still host apps. It records that as the `wiped` verdict.
   - Under that verdict the doorway refuses every new registration until someone runs `POST /admin/provisioning/recover`.
   - On alpha this has to be run on both doorway-alpha and alpha-b.
2. **The guard commit alone (`7d46bc6b1`) can lock people out at login.**
   - A node or device account row that carries the same key as a hosted record gets `403 ACCOUNT_KEY_HOSTED_HERE`.
   - On the fleet that may include James's or Jessica's rows, if they were written before 7ba6f2e0a.
   - The guard closes a likely live takeover path on dev, so landing it quickly has real value. But the operator should choose when, knowing which rows it affects.

An open design choice for the operator: should the guard also look up the provisioning intent by key?

## Residuals after landing (none fixed by this branch)

- A pod key that has no account here yet can still be claimed. Proof that the claimant holds the key is item 5 in `doorway-identity-recovery-gate-residuals-2026-10-04` (on the branch).
- JWTs minted before the guard landed are not re-checked.
- `GET /admin/hosted-users` requires no authentication.
- Two doorways that share one pool are not serialized against each other.
- `POST /admin/hosted-users` ignores the account's agency phase.
- The evidence for a recover call is the shared `API_KEY_ADMIN`, which every admin holds.
- When deprovisioning fails during graduation, the doorway still forgets the record.
- The account-row half of the guard is untested, because testing it needs Mongo.

## Cure

The operator chooses one of three paths:

- land `7d46bc6b1` alone;
- land the whole branch and run recover on doorway-alpha and alpha-b;
- re-scope the branch.

Whichever path is chosen, each landing needs a household receipt for every story pre-push names. Pre-push's own `t2-receipt.sh` lists those stories.
