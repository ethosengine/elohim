---
name: Use GET, not HEAD, when probing /blob/<hash> on elohim-storage
description: HEAD on /blob/<hash> returns 404 even when GET returns 200 — route registration is GET-only and HEAD falls through to a 404 catch-all; using `curl -sI` for blob existence checks gives false negatives
type: feedback
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
`curl -sI` (HEAD) on `http://<peer>:8090/blob/<hash>` returns 404 even when `curl -s` (GET) on the same URL returns 200 with full bytes. This is because the route is registered GET-only in elohim-storage's http.rs (line ~533) and HEAD requests fall through to a 404 catch-all.

**Why:** Cost the better part of an hour during 2026-04-30 alpha debugging. I treated `curl -sI` results as ground truth for "Adam doesn't have the blob," concluded the substrate had a deep bug, and worked through four hypotheses that were all chasing a non-existent problem. Switching to GET probe immediately revealed that Adam serves the blob fine.

**How to apply:**
- For blob existence probes, use `curl -s -o /dev/null -w '%{http_code}\n'` (GET with status only), not `curl -sI`.
- For size/header-only checks, use a `Range: bytes=0-0` GET — same headers, single byte transfer.
- The HEAD-on-/blob/ behaviour is a small bug to fix in elohim-storage too (HEAD should mirror GET for content-addressed routes), but until then, don't trust HEAD.
- This generalizes: any time a probe contradicts what the user reports, sanity-check whether the probe method itself is sound. The route's lack of HEAD support was invisible until the GET worked.
