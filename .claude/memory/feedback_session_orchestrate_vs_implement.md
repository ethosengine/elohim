---
name: Sessions split into orchestrating vs implementing
description: When classifying or resuming Claude sessions, distinguish orchestrating threads (high-level planning/strategy/delegation) from implementing threads (hands-on code edits). Resume prompts differ in shape.
type: feedback
originSessionId: a3da8e7a-0d13-473c-9e43-f45494bf2dde
---
Sessions fall into two modes and should be classified/resumed differently:

**Orchestrating threads** — setting direction, evaluating options, drafting plans, deciding what to delegate to subagents, reviewing work, making architecture calls. No code edits, or only light scaffolding.

**Implementing threads** — hands-on edits to specific files, running tests, debugging, wiring services. The work product is the diff.

**Why:** The user works in both modes and got interrupted across both. He thinks of them as distinct kinds of threads with distinct resumption needs.

**How to apply:**
- When summarizing sessions, label each as orchestrating or implementing (or mixed — note the transition point).
- Orchestrating resume prompt: restate the problem space, list options considered, note the open question/decision waiting. Example hook: "We were deciding between X and Y for the replication gate — last turn I recommended Z, you hadn't confirmed."
- Implementing resume prompt: name the specific files, the last passing/failing test, the pending TODO. Example hook: "Last edit was at `path/to/file.rs:L123`, test `foo::bar` was failing on X."
- When producing a set of resume prompts, keep them in their native mode — don't force an orchestrating thread into an implementation checklist, and don't bloat an implementation thread with strategy restating.
