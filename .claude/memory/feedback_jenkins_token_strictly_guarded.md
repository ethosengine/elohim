---
name: JENKINS_TOKEN — orchestrator-autonomous, but only on verified Jenkins state
description: The shift Opus orchestrator may use JENKINS_TOKEN autonomously to trigger parameterized builds, but only after it has verified (not guessed) that doing so won't cause pipeline interruptions or build storms
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
The shift Opus orchestrator can authenticate to Jenkins via `$JENKINS_USERNAME` + `$JENKINS_TOKEN` (env: `JENKINS_USERNAME`, `JENKINS_TOKEN`, `JENKINS_URL`) to trigger parameterized rebuilds (e.g. `RESET_STORAGE=true` for genesis schema-drift recovery). User clarified: **per-occurrence user confirmation is NOT the gate** — the orchestrator decides autonomously. The actual gate is verified Jenkins state.

**Why:** "you don't need a hard ok from me the user to trigger the build. You need to push the decision to the opus orchestrator, and it needs to know, (not guess) to avoid pipeline interruptions, or build storms." Auto mode covers the routine call; punting to the user is the wrong default. The risk being managed isn't user-consent-of-token-use — it's stomping concurrent builds or kicking off cascading retries.

**How to apply:**
- Orchestrator may issue authenticated triggers without a per-use user prompt, IF AND ONLY IF it has verified preconditions from actual Jenkins reads (not guesses or assumptions).
- "Verified" means: read `mcp__jenkins__getJob` on every alpha-cluster-touching pipeline (`elohim-orchestrator`, `elohim-genesis`, `elohim-edge`, `elohim-holochain`, `elohim`) and confirm `lastBuild.building: false`. Also check no recent trigger of the same target pipeline within build-cycle-time minutes (build-storm prevention).
- "Guessed" = MCP unavailable, ambiguous response, partial reads, "should be fine." If the orchestrator cannot verify, it does NOT proceed — it either re-checks after a wait or bails with an explicit question to the user.
- Subagents (ci-observer, ci-investigator) never invoke the trigger — they feed evidence (queue state, build correlations) back to the orchestrator, which decides.
- Token, username, URL never appear in logs, journals, transcripts, commit messages, or skill files. Reference only as `$JENKINS_*` placeholders.
- Don't trigger pipelines that cascade (e.g. orchestrator with parameters would dispatch downstreams). Stick to leaf pipelines (genesis is the canonical leaf for this pattern).
- Destructive parameters (`RESET_STORAGE=true` does `kubectl exec rm content.db && kubectl delete pod`) get the in-flight check at strictest tolerance — anything alpha-touching = defer.
