````markdown
# Sprint Result — `<shift-id>`

**Objective:** `<objective-name>` — `<description>`
**Status:** `<done|bailed|interrupted>`
**Iterations run:** `<n>` of `<budget.iterations>`
**Wall clock:** `<elapsed-minutes>` of `<budget.wall_clock_min>`

## Outcome

### If done

- Final measurement: `<value>`
- Stability evidence: passing measurements at iteration `<n>` and iteration `<m>`, with at least one on a fresh trigger (`<build-id>`)
- Landing commit: `<sha>` (if any)
- Files changed: `<list>`

### If bailed

**Bail reason:** `<one-paragraph reason from Opus>`
**Question for operator:** `<explicit question Opus needs answered>`
**Proposed next step:** `<what Opus recommends once the question is answered>`
**Last measurement:** `<value>` at iteration `<n>`

### If interrupted

**Interruption type:** `<stop|budget-exhausted|tool-interruption>`
**State at interruption:** `<what was in flight>`

---

## Dialogue stasis

Read `epr session stasis status --shift-current` before the closing marker.
Absent, retained-away or unfolded evidence is **unmeasured**, never zero.

- Shift/session: `<shift-id>` / `<session incarnation>`
- Away ruling: **unmeasured** — `<reference, authorization source, scope, UTC expiry>`
- `shift-marker` edges (same shift id): open **unmeasured** / close **unmeasured** — `<event references and timestamps>`
- `dialogue-blocked-seconds@1`: **unmeasured** against `dialogue-blocked-ceiling@1` = **1800 s**; rail outcome **unmeasured**.

| Blocked seconds by class / presence | Collaborative | Automated | Unknown |
|---|---|---|---|
| Authorization | unmeasured | unmeasured | unmeasured |
| Question | unmeasured | unmeasured | unmeasured |
| Silence | unmeasured | unmeasured | unmeasured |

Use folded interval unions; overlapping classes must not inflate the shift total.

- Incidents by resolving rung (0–5): **unmeasured** — `<counts, incident references; unresolved separately>`
- `harness-exceptions@1`: **unmeasured** — `<counts by hook and evidence>`
- Observation gaps / unavailable status / pending markers: `<evidence or unmeasured>`

## Proposed palette additions

Ordered by priority (blockers first). Next shift's kickoff should review and
approve or reject each.

### Blockers *(approve before next shift)*

- `<narrow literal>` → proposed generalization: `<broader pattern>`
  - Purpose: `<why Opus / Sonnet needed this>`
  - Iterations where it arose: `<list>`
  - Safety taxonomy: `<broadly_safe|subcommand_scoped|never_wildcard>`

### Wishlist *(low priority, approve when convenient)*

- `<entry>` as above

---

## Proposed pipeline legibility improvements

Aggregated anti-patterns Haiku observed. Addressing these between sprints
reduces cost and increases signal quality for future shifts.

| ID | Name | Occurrences | Attestation maps to |
|----|------|-------------|----------------------|
| `AP-NNN` | `<name>` | `<n>` | `<brit-attestation-field>` |

## Judgment calls log

Iterations where Opus bailed, dispatched Sonnet for verification, or
distrusted a measurement. Feeds retrospective analysis of Objective
schema and playbook improvements.

- Iteration `<n>`: `<one-line summary of the call and its outcome>`

## Measurement trustworthiness notes

Low-confidence Haiku findings, done-regressions, oracle-skepticism events.

- Iteration `<n>`: `<event>`
````
