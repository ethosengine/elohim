# Sprint Retrospective — `<sprint-name>`

**Date range:** `<YYYY-MM-DD>` to `<YYYY-MM-DD>`
**Shifts included:** `<list of shift-ids>`
**Compiled by:** `<operator>`

## Top anti-patterns by frequency *(permanent section)*

Aggregated from every shift's observed anti-patterns. Rank by total
occurrences across all shifts in the sprint.

| Rank | ID | Name | Total occurrences | Shifts hit |
|------|-----|------|-------------------|------------|
| 1 | `AP-NNN` | `<name>` | `<count>` | `<n>/<total>` |

**Actions:** per entry, list the fix decided on (Jenkinsfile change, tooling
PR, defer, etc.) and the owner.

## Graduated palette entries *(permanent section)*

Wishlist items that appeared across multiple shifts; promote to
`.claude/settings.json` in one batched PR.

- `<proposed pattern>` — appeared in shifts `<list>` — absorbs `<count>` literal variants
- …

---

## Implications for brit *(migration-bridge; remove once brit Phase 2a + rakia attestation-consumption lands)*

Itemized: observed pain → proposed attestation field or shape. Each item
cites shift-result evidence.

- **Observed:** `<anti-pattern or measurement-trustworthiness event>`
  - **Shift evidence:** `<shift-id#iteration>`
  - **Proposed brit shape:** `<BuildAttestation.field | DeployAttestation.field | new type>`
  - **Value if shipped:** `<which sprints / shifts would have been cheaper>`

## Implications for rakia *(migration-bridge; remove once rakia is the primary orchestrator)*

Itemized: observed orchestrator gap → proposed rakia behavior.

- **Observed:** `<permission blocker, missing action type, opaque build state, etc.>`
  - **Shift evidence:** `<shift-id#iteration>`
  - **Proposed rakia behavior:** `<stepwise action, attestation read, reach check, etc.>`
  - **Value if shipped:** `<which shifts would have been simpler>`

---

## Implications for the agentic-developer itself *(permanent section)*

v1.1 priorities, ordered by how many shifts would have benefited.

- **Proposal:** `<change to playbook, Objective schema, tier model, etc.>`
  - **Shifts that would have benefited:** `<list>`
  - **Estimated lift:** `<S|M|L>`

## Measurement trustworthiness review *(permanent section)*

Aggregated low-confidence / regression-after-done / oracle-skepticism
events. Decide per entry whether to tighten the Objective schema,
adjust stability defaults, or rewrite a specific measurement command.
