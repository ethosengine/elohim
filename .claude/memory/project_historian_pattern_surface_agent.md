---
name: Historian — pattern-aware un-archive agent
description: The inverse of cleanup. Indexes the archive, recognizes when current work matches an archived pattern, surfaces the precedent as a risk/precedent annotation into the active plan or sprint. Performs the `surface` primitive named in the lifecycle spec.
type: project
originSessionId: b5ef4833-2583-4482-b36e-b595da75dafe
---
Cleanup archives stale memory. Historian *re-activates* archived memory when the present trajectory matches an archived pattern. Archive isn't a graveyard; it's a library. Historian is the librarian.

**Operational shape:**
- Cleanup writes to `.claude/archive/<date>/` (existing)
- Historian builds a pattern-signature index over the archive: failure shapes, success shapes, decision frames, recurring stack traces
- During planning (e.g., before `/shift`) and during execution (sense-and-respond layer), historian compares current trajectory against the pattern index
- On match, historian *surfaces* the archived entry as an inline annotation: "this is shaped like ARCHIVE entry X, which went Y" — risk if Y was failure, precedent if Y was success
- The annotation enters the live plan/sprint as context, not as a notification — it pivots planning, not interrupts execution

**Why this matters:** Our lifecycle spec names `surface` as a primitive but doesn't develop it. Historian is the agent that performs `surface` continuously, not on demand. Without it, the archive is write-only and the same mistakes recur. With it, the archive becomes a load-bearing safety + opportunity surface.

**How to apply:**
- Don't expand cleanup to do this; historian is a distinct service (different cadence, different intent)
- Historian's annotations belong in the present tense: in active plans, in sprint pre-flights, in `/shift` Objective drafts
- Historian operates on the past-perspective slice of the temporal triad (see project_three_temporal_perspectives.md). It does not author roadmap (future-perspective) or perform hygiene (development cycle present-perspective)
- Build it AFTER memory-kit realignment lands — needs archive index + pattern-signature schema; not blocking near-term work
