---
description: Start, resume, inspect, stop, or recover a bounded shared Claude/Codex delivery shift.
---

# /shift

Apply the `agentic-developer` skill. Use the same host launcher in either runtime:

```bash
just dev shift start <contract.json|yaml> --runtime codex
just dev shift resume <shift-id>
just dev shift resume <shift-id> --runtime claude
just dev shift status <shift-id>
just dev shift stop <shift-id>
just dev shift recover
```

Read `genesis/agentic/SHIFT.md` before preparing a declaration. Resolve missing
objective details from current native records and the user's existing authorization.
Do not repeat an authorization interview when the conversation already supplies it.
Freeze scope, original intent/commitment/scope CID, and the absolute deadline before
execution. Resume never renews the deadline; runtime switches start a fresh
conversation over the same durable evidence. Never resume a "latest" session.

The host owns publication and household lifecycle. Native appointed independent
acceptance, exact-revision gate/household evidence and authorized publication are
the completion authority. A journal or runtime's final message cannot close work.
A persistent stop marker prevents automatic recovery. Legacy journals remain
inspection-only; missing or corrupt execution authority refuses a launch.
