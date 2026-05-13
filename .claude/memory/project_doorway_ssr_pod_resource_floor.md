---
name: Doorway SSR pod resource floor
description: Doorway with SSR enabled needs ≥1Gi memory + startupProbe; 256Mi was a coin-toss OOM, no startup probe killed pod for liveness fail before V8 init completed
type: project
originSessionId: 2a998ad1-49e1-4f9d-a4ca-0cb796181cbf
---
Doorway pod resource floor when SSR_BUNDLE_PATH is set.

**Why:** doorway-ssr-deliver shift (2026-05-08) found alpha's `memory: 256Mi` + `livenessProbe.initialDelaySeconds: 10` were both too tight for SSR. Build #953 verified one render at 200 OK + 3666 bytes + ngh="0", but build #954 (same code, empty fresh-trigger) flapped 502/503/404 — same image, different luck on cold start. V8 parsing the 51MB Angular server bundle spikes working set to ~200MB; with 256Mi, the pod sometimes OOM'd. With initialDelaySeconds: 10, the liveness probe fired before doorway's HTTP server bound :8080. Build #955 with `81014c17` (memory→1Gi, CPU→1000m, startupProbe 120s budget) verified the substrate stable across two consecutive builds with a fresh-trigger between them.

**How to apply:**
- When deploying doorway with `SSR_BUNDLE_PATH=…` to any cluster, set:
  - `resources.limits.memory: 1Gi` minimum (V8 + 51MB bundle + app)
  - `resources.limits.cpu: 1000m` (cold-start parse needs headroom)
  - `startupProbe` with `failureThreshold * periodSeconds ≥ 120s` (V8 init takes 30-90s on cold start)
  - `livenessProbe` and `readinessProbe` can stay aggressive — startupProbe gates them
- `genesis/orchestrator/manifests/doorway/staging.yaml` and `prod.yaml` still need the same fix when SSR rolls to those environments.
- The Angular SSR bundle size (51MB / 171 .mjs files) drives memory floor. If the bundle gets significantly bigger, revisit.
