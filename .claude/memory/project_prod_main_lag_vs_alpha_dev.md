---
name: ""
metadata: 
  node_type: memory
  originSessionId: b265419a-8b6b-4814-acc3-9bb106b13095
---

The doorway hosts deploy from DIFFERENT branches/legs (edge pipeline `elohim/holochain/Jenkinsfile`, stages `Deploy Edge Node - {Alpha,Staging,Prod}`):

- **`doorway-alpha.elohim.host`** ← `dev`, alpha env, doorway-A (`alpha.yaml`).
- **`elohim.host`** (bare apex) ← `dev`, alpha env, **doorway-B / alpha-b federation peer** (`alpha-b.yaml`, deploy `elohim-doorway-alpha-b`). NOT a main/prod host. This is the dual-doorway federation being proven.
- **`doorway.elohim.host`** ← `main`, prod env (`prod.yaml`) — the legacy prod doorway. `main` runs ~5018 commits behind `dev` (no release cut in months), so anything on `doorway.elohim.host` is far stale.

**Why `elohim.host` lagged while `doorway-alpha` was current** (both deploy from dev in the SAME `deployDoorwaysWithTestShape('alpha', …)` with the same `alpha.env` tags, so when both succeed they're identical): the alpha-b leg is wrapped in `catchError(buildResult: 'UNSTABLE')` (~L798-807) — it **silently swallows alpha-b deploy failures** (common cause: ingress hostname conflict on `elohim.host`), so `elohim.host` stays on its last-good image while doorway-A keeps updating. Fixed the stale header comment + pushed dev 2026-06-23 (commit `69607bd04`); a clean dev edge build where the alpha-b leg succeeds is what brings `elohim.host` current.

**Diagnostic signature:** a UI bug on one doorway host but not another is a per-host deploy lag, not a code bug. Confirm by fetching each host's hashed app CSS (`/` → `styles-*.css`) and diffing for the expected fix marker (e.g. `color-scheme: dark`) — don't re-derive the bug from source. Worked example: resilience hypercard "light text on light background" on the EPR bar (2026-06-23) — fix `05827e5ed` was live on alpha, missing on elohim.host. Deploys are operator-owned (never run kubectl); the repo manifests/pipeline are the cleanup surface. Sibling to [[feedback_che_devworkspaces_direct_to_main]] and [[feedback_frontend_review_eyes_first]].
