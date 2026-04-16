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
