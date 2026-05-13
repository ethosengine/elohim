---
name: Check helm chart status before recommending in operator runbooks
description: Bitnami has been blocking/deprecating charts lately; verify chart availability and license before naming a vendor in a runbook
type: feedback
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
When writing K8s deployment runbooks that name a specific helm chart (Bitnami, Helm Hub, vendor charts), check current status before publishing:
- Is the chart still maintained?
- Has the publisher restricted access (Bitnami in particular has been gating access to certain charts in 2025-2026)?
- Are there license or commercial-use changes since the last reference?

**Why:** On 2026-05-09 the MinIO + sccache runbook recommended `bitnami/minio` chart. The k8s operator pushed back: Bitnami has been blocking/deprecating charts. The runbook would have wasted operator cycles before discovery.

**How to apply:**
- Before writing `helm install <vendor>/<chart>` in a runbook, run a quick check: `helm repo update && helm search repo <vendor>/<chart>`, plus a web search for "<chart> deprecated" or "<chart> license" recent posts.
- Prefer official upstream charts where available (e.g. `minio/minio` from MinIO Inc. directly) over third-party redistributions.
- When a vendor chart is the only option and may be unstable, name the alternative paths in the runbook (manual manifest, operator-managed CRD, alternative vendor) so the operator can swap if the primary is unavailable.
- Don't assume past success means present availability. Charts that worked 6 months ago may now be paywalled or removed.
