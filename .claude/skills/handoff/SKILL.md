---
name: handoff
description: Write or update a handoff document so the next agent with fresh context can continue this work.
---

Write or update a handoff document so the next agent with fresh context can continue this work.

Steps:

1. Check if `HANDOFF.md` already exists in the project root.
2. If it exists, read it first to understand prior context before updating (extend it; don't discard still-relevant history).
3. Create or update the document with these sections:
   - **Goal** — what we're trying to accomplish.
   - **Current Progress** — what's been done so far (cite commits/branches/files; verify against the repo, don't restate from memory).
   - **What Worked** — approaches that succeeded.
   - **What Didn't Work** — approaches that failed, so they're not repeated.
   - **Next Steps** — clear, ordered action items for continuing.

Ground every claim in real state: check `git log` / `git status`, run the relevant probe, or read the file — a handoff that restates assumptions is worse than none. Call out anything committed-but-unpushed or merged-but-not-deployed explicitly, since that's exactly what a fresh-context agent can't see.

Save as `HANDOFF.md` in the project root and tell the user the file path so they can start a fresh conversation with just that path.
