---
name: ""
metadata: 
  node_type: memory
  originSessionId: b265419a-8b6b-4814-acc3-9bb106b13095
---

`alpha.elohim.host` tracks **dev**; `elohim.host` (prod) tracks the **main** release line. As of 2026-06-23, `origin/main` is **~5018 commits behind `origin/dev`** — main has not had a release cut in a long time. So a frontend/CSS fix that landed on dev (and renders correctly on alpha) will NOT appear on prod until a `dev → main` release + redeploy.

**Diagnostic signature:** a UI bug reported on `elohim.host` but NOT on `alpha.elohim.host` is almost always prod-stale, not a code bug. Confirm by fetching each host's hashed app CSS (`/` → `styles-*.css`) and diffing for the expected fix marker (e.g. `color-scheme: dark`) — don't re-derive the bug from source.

Worked example: the resilience hypercard "light text on light background" on the EPR bar (2026-06-23). Fix `05827e5ed` (2026-06-12, on dev/alpha — paired `--lamad-*` tokens + shell `styles.css` `color-scheme` contract + lamad `_chrome-binding` hypercard binding) was correct and live on alpha; prod's CSS lacked the `color-scheme` contract → the C2 split (Canvas system colors stay light while `--lamad-*` palette goes dark). Resolution is a release (targeted hotfix onto main, or full dev→main), which is operator-owned — never trigger a prod deploy. Sibling to [[feedback_che_devworkspaces_direct_to_main]] (release discipline) and [[feedback_frontend_review_eyes_first]] (verify on live render).
