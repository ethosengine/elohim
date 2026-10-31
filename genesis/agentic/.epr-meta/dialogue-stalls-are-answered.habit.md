---
epr-habit-version: 1
id: dialogue-stalls-are-answered
invariant: >
  A stalled session dialogue — an open permission prompt, a turn ending in a question, or the
  steward waiting on silence — is answered by the cheapest sufficient capability level before
  the human is interrupted, and nothing approves on the human's behalf.
status: unwired
active: false
checks: []
refs:
  - "spec: genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md (dialogue scope, presence, independent coordinator and authority boundary)"
  - "plan slices: genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md §Slices A.3–5 (observe-only adapter and native fold), B.6–7 (policy and keeper), C.8–9 (coordinator and control), D.10–11 (silence and audited learning)"
retire-when: >
  when native session runtimes provide this dialogue-response invariant without stasis-specific
  adapters or a separate coordinator, and their own runnable checks cover all three stall classes
  and the prohibition on approving for the human.
---
DELTA 2026-10-30 (BORN unwired): No sensor exists yet; the slice-3 observe-only adapter provides the first evidence, before the native fold and control can prove the invariant.
DELTA 2026-10-30 (slice 1, still unwired): spec genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md reached READY for slices 2–5 on the sixth fresh blind read, after gpt-6-astra reviews in rounds 1–4. genesis/agentic/bin/stasis-probe on Claude Code 2.1.314: 27/27 headless checks yes on stricter predicates, plus a negative control (report probe/20261030T145038Z), all four model aliases bound, and agent frontmatter effort supported (20261030T155306Z). genesis/agentic/stasis_probe_test.py: 9 OK. Six interactive capabilities stay unsupported until the steward-assisted probe runs. The measure rows read skipped (no fold), as they should.
DELTA 2026-10-30 (interactive probe, still unwired): driven in-session under a pty with stasis-probe --interactive-drive (no person needed), reproduced twice (probe/20261030T162945Z, probe/20261030T163031Z): 7/7 yes. Findings: dialog-displayed fires 6.1–6.2 s after PermissionRequest; a human deny emits no event at all; idle_prompt fires 60.1–60.2 s after Stop and never while a dialog is open; asyncRewake wakes an idle session, and the wake arrives as a UserPromptSubmit beginning with <task-notification>, which the presence witness must exclude. Remote Control push and the channel deny-relay still need the steward.
