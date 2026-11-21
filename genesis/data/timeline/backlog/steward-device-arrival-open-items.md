

## Row 58, delta (2026-11-21): the keeper ladder climbed on after the steward approved the prompt

Incident `824f0d7e…:145`. A command met an ask rule at 17:32:44Z and the steward's dialog stayed
open for eight and a half minutes, so the ladder's first rung ran while the wait was real. The
steward approved at 17:41:20Z and the sensor recorded `prompt-closed, closedBy: approved`. The
coordinator then ran rung 2 (a Sonnet keeper) and rung 3 (an Opus keeper), kept telling the
session not to retry a command that had already run, asked for a push notification, and blocked
the session's stop.

Two causes. The native fold keeps the first closer: the first rung's Permit closes the incident
as `parked`, and the later `approved` is ignored (`elohim/eprfs/device-runtime/src/session_stasis.rs`,
`close()` is first-wins and Parked is not Closed). And the ladder in
`genesis/agentic/bin/stasis-coordinator` treats `parked` as still open and takes the next rung.

Fixed in the ladder: before a rung or a delivery it reads the sensor's own close record for the
incident from the journal it already holds locked, and if the dialog is over it ends, writes one
`ladder-ended` row, withdraws the parked ask and delivers nothing more. Five tests. Still owed:
the native fold should let an approval close a parked incident, which is a change in
`device-runtime` and so waits for a deliberate roll; and the coordinator daemon on shem has run
since 2026-11-10 and takes the fix only when restarted, which is the steward's to do because it
serves every session on the device.
