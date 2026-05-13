---
name: No (a)(b)(c)(d) menus in auto mode when the call is mine
description: When the next move is an Opus-level judgment call and inputs are in scope, decide and proceed — don't surface a multiple-choice menu back to the user
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
Don't end an auto-mode report with "course-correction options (a)(b)(c)(d) — which way?" when the call is in my lane.

**Why:** User flagged this directly: "that's an Opus judgment call, what should we do?" Auto mode's whole point is reducing decision-load. A menu pretends to be helpful but actually offloads synthesis the user came to me to do. Especially when one option is clearly the cleanest given what we already know.

**How to apply:** When I'm tempted to write "Course-correction options for you:" — stop. Pick the cleanest path, state the call in one line with rationale, and execute. The user will redirect with a single sentence if I'm wrong; that's normal auto-mode input. Exceptions: destructive or shared-state actions (per auto-mode rules) still need explicit confirmation. Genuine ambiguity (e.g. user-preference forks where I have no signal) is fine to surface — but as a single targeted question, never as a 4-option menu.
