````markdown
# Shift Journal — `<shift-id>`

**Objective:** `<objective-name>` — `<one-line description>`
**Kicked off:** `<ISO timestamp>`
**Budget:** `<iterations>` iterations, `<wall_clock_min>` minutes
**Operator:** `<git user.name>`

## Stability Tracker

- Consecutive passing measurements: `<counter>`
- Required for done: `<consecutive>`
- Fresh-trigger measurement captured: `<yes|no>`

## Trajectory Summary *(last 3 iterations)*

`<auto-maintained header; Opus refreshes at start of each iteration>`

---

## Iteration stanza shape

Each iteration appends one stanza of this shape:

### Iteration `N` — `<iteration type>` — `<timestamp>`

**Measurement:** `<value>` (delta `<+X|-X|0>` from iteration `N-1`)
**Context:** build `<id>`, status `<passed|failed|running>`, first failing stage `<name|none>`

**Observation (Haiku):**

```yaml
primary_failure:
  error_class: <short tag>
  evidence: |
    <5-10 lines>
  files_mentioned:
    - <path>
confidence: <low|medium|high>
```

**Anti-patterns observed this iteration:**

- `AP-<NNN>` — `<name>`: `<one-line evidence>`

**Verification pass (Sonnet):** `<dispatched|skipped>` — `<directive if dispatched>`

**Decision (Opus):** `<progress|stall|novel|done-candidate|bail>`

**Rationale:** `<one paragraph — why this decision from Haiku's finding and trajectory>`

**Action taken:** `<none|edit <file>|commit+push <sha>|retrigger build|dispatch Sonnet|bail>`

**Next iteration:** `<observe-only|act-on-hypothesis|verify-done-candidate|-|terminal>`

---

## Permission wishlist (accumulated across iterations)

- **Blocker:** `<pattern>` — reason: `<why needed>` — iterations: `<list>`
- **Wishlist:** `<pattern>` — reason: `<why convenient>` — iterations: `<list>`
- **Redirect (resolved):** `<pattern>` — redirected to `<approved alternative>` — iteration: `<n>`

## Observed anti-patterns (accumulated across iterations)

| ID | Name | Occurrences | Evidence snippet |
|----|------|-------------|------------------|
| `AP-NNN` | `<name>` | `<count>` | `<excerpt>` |
````
