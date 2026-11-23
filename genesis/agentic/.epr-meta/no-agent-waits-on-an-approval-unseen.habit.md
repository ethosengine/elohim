---
epr-habit-version: 1
id: no-agent-waits-on-an-approval-unseen
invariant: >
  A tool call pending past two minutes in any moored session is seen from outside that session, and
  announced to the steward, to the other sessions here and on sibling devices, and in `berth status`,
  without the held session's help. The watcher only reads transcripts and announces: it never approves,
  cancels or retries.
status: red
active: false
checks:
  - "python3 -m unittest genesis/agentic/held_tool_watch_test.py (from the repository root; scan, one announcement per held call and one on resolution, redaction, push-transport absence, berth status held and held?)"
  - "python3 -m unittest genesis/agentic/berth_test.py (status text and status-json@1 unchanged when nothing is held)"
refs:
  - "watcher: genesis/agentic/bin/held-tool-watch (scan | watch)"
  - "backlog: genesis/data/timeline/backlog/steward-device-arrival-open-items.md rows 34, 82, 83"
  - "measure: .claude/epr-meta/measures.yaml dialogue-blocked-seconds@1"
retire-when: >
  when a held tool call reaches the steward by a transport that does not pass through an agent, on two
  devices, supervised by the device manifest, with the observer's own liveness shown in `berth status`.
---
DELTA 2026-11-23 (BORN red): exists: `held-tool-watch scan` reports any transcript tool_use with no result past 120 s (subagents under their own id; background Bash, Agent, Task, Monitor and a Bash within its own timeout excluded), `watch` announces each held call once and its resolution once by a berth broadcast from sender `held-tool-watch`, a marker file and an optional `HELD_TOOL_PUSH_CMD`, and `berth status` (text and an additive `held` field of `status-json@1`) shows `held`, or `held?` when the marker is stale; held_tool_watch_test 13 OK, berth_test 284 OK. Does not exist: no supervised process runs it (not in the device manifest), no steward push transport is configured, and `dialogue-blocked-seconds@1` is not yet fed by it.
