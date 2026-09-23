# Genesis Artifact System & Compute Capacity Ledger — SUPERSEDED

> **Status:** This plan was authored by Haiku 4.5 (auto-mode disabled) and contained
> wishful thinking that the user caught and corrected. Opus 4.7 audited it, the operator
> delivered real cluster data, and a slimmer + correct plan was implemented in commits
> `ed384b72`, `f96b53ee`, `2024a79f`, `50436877` on the `dev` branch.
>
> The current strategic plan lives at:
> `/projects/.claude-config/plans/ok-we-ve-lost-our-luminous-puppy.md`
>
> The current ledger lives at:
> `genesis/data/rakia/compute-capacity.json` (with schema and 2026-05-04 snapshot archive).

## What was actually built (vs. what this file used to claim)

| Original Haiku claim | What actually shipped |
|---|---|
| 11 tasks including artifact-graph.json, validators, pre-push hooks, rakia foundation framing | 5 commits: deployments.json suspension, Jenkinsfile filter, humans.ts guard, rakia ledger, snapshot-capacity.sh |
| Hand-typed cluster numbers ("12000m CPU") and OVERSPENT settled state | Real numbers from operator snapshot: 46c/134GiB Ready, 34.8c/110GiB headroom, status SUSTAINABLE |
| `Validate Compute` Jenkinsfile stage but no filter | The actual filter (`findAll { !it.suspended }`) in `resolveHumanAssignments` — the load-bearing change |
| `validate-artifacts.ts` based on hand-typed checks | Deferred until a real validator (declared vs observed actuals) is needed |
| Rakia coordinator function names + qahal governance theatre | Deferred entirely; rakia DNA work is backburnered |
| Story groups conflating narrative with deployment | Story groups grounded in real fate-sharing topology (ethosengine-perf, intel-nuc-ops, edge-builders, hp-micro10-storage, shem-remote) |

## Why this stub remains

Deleting would erase the trail of "we tried this, it was wrong, here's why." Keeping the
header above (and only the header) preserves the lesson: when auto-mode is off and a smaller
model is in play, audit aggressively before implementing. Don't trust the first plan that
sounds plausible.
