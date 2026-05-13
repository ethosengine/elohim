---
name: REA prefix is redundant — drop it long-term
description: REA (Resource-Event-Agent) is the conceptual pattern, not a namespace; tables/columns shouldn't carry a `rea_` prefix. The substrate has an asymmetry — `rea_commitments` (with prefix) vs `economic_events` (without) — and the long-term cleanup is to drop the prefix from `rea_commitments`, not add it to `economic_events`.
type: project
originSessionId: 155036b0-387a-441c-91c5-7a1333fb2f07
---
The substrate currently has an asymmetric naming convention:
- `rea_commitments` (with prefix) at `db/diesel_schema.rs:810`
- `economic_events` (no prefix) at `db/diesel_schema.rs:163`

Both follow REA (Resource-Event-Agent) accounting semantics. The asymmetry is a schema-evolution artifact, not two competing tables.

**Why:** REA is the *pattern* the protocol's accounting follows — it's not a vendor or namespace label. Adding `rea_` to every accounting table is redundant the same way adding `relational_` to every SQL table would be redundant. The original prefix was probably defensive ("disambiguate from a hypothetical non-REA accounting layer"), but no such alternative layer exists or is planned. The model is simply Resource-Event-Agent throughout.

**How to apply:**
- When dispatching new view-service or aggregator work, point implementers at the actual table names (`rea_commitments`, `economic_events`) without framing the asymmetry as a bug or asking the implementer to fix it.
- Do NOT propose adding `rea_` to `economic_events` to "fix" the asymmetry. The direction is the opposite.
- Long-term cleanup: a separate migration sprint renames `rea_commitments` → `commitments` (and any other `rea_*` table). Not blocking; cosmetic.
- Plan documents that anticipate this cleanup may use either prefixed or unprefixed names — check the actual schema before writing SQL.
- Same logic applies to columns or types named `rea_*` — strip the prefix when refactoring.
