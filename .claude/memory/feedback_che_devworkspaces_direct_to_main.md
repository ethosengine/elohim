---
index: false
name: feedback_che_devworkspaces_direct_to_main
title: "Push policy — che-devworkspaces main yes, elohim main no"
description: "che-devworkspaces (CI/image infra) pushes straight to main, inert-by-default; elohim monorepo main is reviewed dev→main only — surface classifier blocks."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: efcfeef6-e1eb-4f9a-bc62-4a561c22b8b9
---

When I edited the `elohim-edgenode` pipeline (`jenkins/Jenkinsfile-elohim-edgenode`) in the **che-devworkspaces** submodule, I pushed to a branch and opened a PR. The operator rejected the PR: **"no push to main.. there's no dev → main pipeline workflow on that repo."** Push the commit straight to `main`.

**Why:** the elohim ecosystem is low-PR by design — feature branches land on `dev` via local fast-forward (no PR), and release review is only at `dev → main` of the **monorepo**. Infra/image repos like che-devworkspaces have no merge automation and the multibranch Jenkins jobs build `main` directly, so a PR just sits there as dead weight.

**The operator's explicit split (2026-06-18):** _"push to main on elohim (no), push to main on che-devworkspaces (fine)."_ So:
- **che-devworkspaces (and similar CI/image infra repos):** commit + push CI/Jenkinsfile changes **directly to `main`** (fast-forward) — no PR, no branch. Multibranch jobs build `main`, so a PR just sits.
- **elohim MONOREPO `main`:** do **NOT** push directly — that is the reviewed `dev → main` release branch. (Monorepo dev-flow = fast-forward onto `dev`, no PR.)

**How to apply:** make infra-main changes **inert-by-default** (a new param defaulting to current behavior, like the `HC_FEATURES` default) so a direct-to-main push can't destabilize. Don't default to branch-first + PR for infra repos.

⚠ **The auto-mode classifier may block an infra-main push** if "no push to main" was said earlier in the session (it reads it literally / globally). That phrasing usually means "skip the PR, push straight to main" — surface the block to the operator and let them grant the push (they did, via /permissions). Don't route around the denial.

Distinct from [[feedback_commit_only_integrator_pushes]] (autonomous/shift mode on the monorepo = commit-only, integrator pushes): that rule is for unattended monorepo work; when the operator **explicitly directs a push** (as here, and the `ethosengine/holochain@elohim-0.6` jemalloc-prof fork push), I push. Related leak-hunt context: [[project_storage_metrics_surface_and_leak_verdict]].
