---
name: scribe-authoring-loop-first-run
title: Scribe authoring loop — first run complete
description: scribe (Opus 4.6 primary writer) + dispatcher technical review + fresh blind-reader ran end-to-end 2026-08-15 on backlog rows 16/16a; storyteller conversion awaits operator sign-off.
metadata: 
  node_type: memory
  type: project
  originSessionId: c1280948-357d-4a93-9b91-d8d8273c0e3e
  modified: 2026-08-15T13:09:45.538Z
---

**The workflow (operator directive 2026-08-15, first run complete same day):** for prose deliverables, `scribe` (planted eprfs agent, pinned `claude-opus-4-6`, tools Read/Write/Edit) is the PRIMARY writer; the dispatching session supplies the technical spec and reviews rounds for technical coherence only — it does not rewrite prose. A fresh context-isolated [[pin-reader-agents-to-older-opus|blind-reader]] then audits legibility (path + profile only; it cannot be scoped to a diff, it cold-reads the whole document).

**First run (2026-08-15, backlog rows 16/16a — agent-capability attestation + inclusion constraint, `commons-holonic-stewardship-backlog.md`):** two rounds to approval. Round-1 draft was prose-faithful; the two drops were both technical-context items (a load-bearing gate-satisfaction clause; the cluster's anchor refrain) — the failure mode landed exactly where the design expects, on the reviewer's side of the split. SendMessage to the same agent resumed it with context intact; round 2 applied both corrections with everything else byte-identical. Scribe's judgement-call reporting (no-GAP + 4 flagged calls) made review cheap — calls were checked against file precedent instead of re-reading cold.

**Mechanics learned:** a package planted mid-session DID become dispatchable in the same session (the harness announced it after a turn boundary — earlier "next session" assumption was wrong). The `projections` block in an AgentPackage requires `frontmatter` per projection (schema-enforced), and `.claude`/`.codex` runtime files regenerate from `metadata.modelHints`, so frontmatter blocks inside the package can go stale without failing verify — refresh them when changing modelHints.

**Next:** once the operator is happy with the loop, apply the same primary-writer conversion to `storyteller`. Capability-selection-as-policy (the durable design question behind the pin) is canonized at rows 16/16a of the commons-holonic-stewardship cluster, including the lamad/avodah earnable-path + plural-DID-authorship inclusion constraint.
