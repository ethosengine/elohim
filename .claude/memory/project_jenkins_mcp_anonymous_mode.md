---
name: Jenkins MCP runs as anonymous (OIDC constraint)
description: Jenkins is OIDC-protected; MCP must omit Authorization header to avoid redirect-loop. Reads work, writes don't. Builds dispatch via webhook + [build:*] commit tags.
type: project
originSessionId: 76c5f4ce-df54-4e3e-b98c-5d1028949072
---
Jenkins MCP at `https://jenkins.ethosengine.com/mcp-server/mcp` is registered with `claude mcp add jenkins ... --transport http` — **no auth header**. Adding `Authorization: Basic <token>` triggers the OIDC plugin's interactive login flow and gets you a 50-redirect loop to `/securityRealm/commenceLogin`. Anonymous works because the MCP plugin doesn't require auth and Jenkins's anon role has Overall.Read + Job.Read.

**Why:** The repo's Jenkins is behind oic-auth without `allowTokenAccessWithoutOicSession`. Enabling that flag would let API tokens bypass OIDC, but the user opted out — anonymous-read covers all read-side needs and `triggerBuild` was never essential because the orchestrator's webhook + graph-walking strategy is the canonical dispatch path.

**How to apply:**
- All `mcp__jenkins__*` read tools work freely (`getBuild`, `getBuildLog`, `searchBuildLog`, `getJob`, `getJobs`, `getBuildChangeSets`, `getTestResults`, `getFlakyFailures`, `getStatus`, `getBuildScm`, `getJobScm`).
- `mcp__jenkins__triggerBuild` and `mcp__jenkins__updateBuild` will fail with permission denied — **don't call them**.
- To trigger/retrigger a build: `git commit --allow-empty -m "ci: retrigger [build:<pipeline>]" && git push`. Tags: `[build:edge|dna|app|genesis|sophia|steward|all]`, comma-separated forms work.
- For agentic-developer "fresh trigger" stability requirement: a fresh `git push` is the only fresh-trigger path; the previous wording allowing `mcp__jenkins__triggerBuild` is now removed from the skill.
- `genesis/agentic/scripts/jenkins-measure*.sh` send a custom `Jenkins-Token` header that Jenkins ignores — those scripts run as anon and "work" because anon-read covers `lastCompletedBuild/api/json`. The token is decorative.
- If MCP ever shows `Needs authentication` in `claude mcp list`, do NOT re-add an Authorization header — check that the URL still returns 200 to anonymous POST first (`/mcp-server/mcp` should respond with mcp-session-id and CSP cookie containing base64-decoded "anonymous").
