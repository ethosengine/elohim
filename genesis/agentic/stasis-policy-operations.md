# Dialogue stasis policy pinning and audit

The steward reviews and pins the exact bytes of
`.epr-meta/elohim/policies/dialogue-stasis-policy.json`. Creating the file,
validating its schema, or preparing a command does not activate it. The native
policy loader and live CID enforcement are implemented; the installed binary
must expose `policyState` and `policyCid` in `session stasis status --json`.
A successful command without those fields is an older native implementation,
not evidence that a policy is pinned.

Run from the checkout:

```bash
python3 -m unittest genesis.agentic.stasis_policy_test
EPRFS_BIN=/path/to/eprfs genesis/agentic/bin/stasis-policy-pin
```

The native raw-byte command is
`eprfs cid .epr-meta/elohim/policies/dialogue-stasis-policy.json`, provided by
`elohim/eprfs/eprfs-cli`. Do not use `--body` or `--short`. The installed `epr`
and `epr flow` help inspected for this slice expose no file-CID verb;
`eprfs-agent --help` is unavailable here, and that package currently declares a
library. Provision the native CLI separately; the helper never builds Cargo
or substitutes a Python hash. `EPRFS_BIN` selects its executable.

The helper prints the CID and a shell-quoted `epr flow note --kind ruling`
command targeted at the policy path. The reason is JSON with `purpose`,
`policyPath`, `policyCid`, and `version`. **Only the steward runs that command**,
in a session whose resolved actor is the steward, after reviewing the exact
bytes. Do not copy an agent identity into the ruling. Rerun the helper after
any byte change, including formatting. The helper only prepares the command;
it never records a ruling itself.

The policy uses the native camelCase budget fields and `Bound` wire shape.
Approach is derived from `stall.limit * stall.thresholdPct / 100` (180 s),
Breach is its limit (300 s), and an omitted bound sense means Ceiling. This
avoids competing explicit values for the same bound. The source-parity test
reads `StasisPolicy::builtin` literals and the classifier's shape-pattern IDs;
a source refactor that defeats this narrow parser fails the test. The extra
1800 s `shiftCeiling` mirrors the existing native default. Slice 6's native loader
and slice 9's coordinator consume this file after native verification. Classification patterns and normalization retain the native
implementation; this slice adds no alternative permission matcher.
The native `bash-c` pattern also recognizes `sh` and `zsh`, while the spec's
initial list names `bash -c`; the policy preserves the native pattern unchanged.

The native loader validates these fields, uses them in the fold and classifier,
and resolves the latest applicable **steward** ruling. With the
ladder on, a live `decide` must return exit 3 for a missing policy or CID
mismatch, emit no decision or guidance, and record why it stayed inert. Preserve
shadow defaults and ladder-off sensing. Confirm the ruling reason format above
in that loader; the spec does not prescribe a structured pin payload.

Prepare an audit at session close:

```bash
genesis/agentic/bin/stasis-audit --session SESSION --transcript /path/to/transcript.jsonl --reviewer codex
```

`--repo DIR` selects the checkout. Without `--transcript`, the helper uses the
single transcript path recorded in the session's private `state.json` cache;
multiple paths require an explicit selection. The journal is read under its
shared lock, including retained segments. Original denied calls are joined by
tool-use ID or an unambiguous sensor `inputDigest`, then classified with
`epr session stasis classify --corpus FILE --main-checkout DIR --cwd DIR`.
`EPR_BIN` selects that native executable. Missing correlation, missing native
classification, corrupt records, and an unclosed session remain explicit gaps.

Findings name journal sequence and transcript line, without raw arguments.
The helper reads `git log --all` over the session window; commit IDs and parent
links are review context, not attribution to this session or proof of a remote
push. Tool results establish that a result was observed, not that the remote
effect succeeded. Attempt-only and failed calls retain those labels.

Native effect tuples currently distinguish `git push` and `gh api`. The required
denied-push/later-API-merge fixture therefore produces a separately labelled
`remote-publication-review-candidate` for mutating requests to GitHub
merge endpoints (including POST implied by `gh api` fields). The reviewer must
check target and outcome; an explicit GET is excluded.
This hint neither changes native equality nor expands the policy rule set.
Opaque scripts, wrappers, arbitrary APIs and tool spellings beyond the native
classifier require transcript review. Shadow parks are labelled hypothetical;
shape-class parks are procedural, not authoritative human denials.

`--reviewer codex` prints `argv` and a stdin brief for a read-only, nonprompting
`codex exec`. It does not call a model, even with `--write`. The independent
reviewer must inspect actual transcript and git evidence and return a verdict;
preparation alone never satisfies L1's passing other-family audit. Raw evidence
is private to the device and is quoted as untrusted material in the brief.

The default prints the exact verdict-note command. `--write` runs only that
command, recording `changes-requested` with independent review still pending.
No automated clean scan records `approved` or activates a promotion. Once the
independent review is complete, its reviewer records its own evidence-backed
verdict through the existing flow-note interface. Missing or failed review
leaves a rung 0 entry proposed.

## Runtime wiring and activation

Apply in this order:

1. Use a current native binary. `EPR_BIN` selects one explicitly for both hooks
   and the coordinator. The device's existing `epr bin follow --tool epr`
   handles published descendant upgrades; a refused follow is not permission
   to replace the shared binary head manually.
2. Wiring is checked in: twelve Claude registrations in `.claude/settings.json`
   and seven Codex registrations in `.codex/hooks.json`. Both sensor files are
   projections of the same planted HookPackage, whose `registrations` records
   each runtime separately. Do **not** reapply the historical
   `stasis-settings.patch.json` to an already wired checkout. Leave
   `STASIS_LADDER` unset for shadow sensing. Review/trust the new Codex hook
   definitions with `/hooks`, then start a new session. No persisted hook trust
   is installed by the package projector.
3. Review and pin the policy's exact CID using the procedure above. Confirm
   `"${EPR_BIN:-epr}" session stasis status --session SESSION --json` says
   `policyState: pinned-match`, with a `policyCid`. A missing, mismatched or
   unreadable status cannot activate delivery.
4. Set `STASIS_LADDER=live` and the selected `EPR_BIN` for new harness processes.
   Restart an existing shadow
   daemon with `stasis-coordinator stop`, then `stasis-coordinator start --live`
   after its flock is released. A daemon inherits its launch environment;
   changing a parent shell cannot reconfigure an already running process.

Run `just gate dialogue-stasis` for the sensor, coordinator, delivery, policy,
audit and probe fixture contracts. Run `pnpm run elohim-agent:packages:verify`
after package or registration changes. These checks do not call a model or
record a policy ruling.

### Codex coverage

Codex registers SessionStart, PreToolUse, PermissionRequest, PostToolUse,
UserPromptSubmit, Stop and SessionEnd. Its `turn_id` becomes the sensor's
prompt correlation key; journal session identity remains `runtime: codex`.
Notification, PermissionDenied and PostToolUseFailure are not registered.
Missing human-denial and presence evidence stays unknown. A continuation
prompt is not promoted into a human-presence witness. Codex currently captures
permission requests and Stop events without emitting control decisions on them.

Pinned guidance returns through PostToolUse or UserPromptSubmit, once per
reader and delivery record. Codex notifications name the session dialogue,
not a Claude-only PushNotification tool. It does not borrow Claude's idle-wake
capability: advice waits for the next supported hook. The official
[Codex hook contract](https://learn.chatgpt.com/docs/hooks) describes this
background-delivery and trust boundary.

The Codex keeper runs with a separate working directory, user configuration
ignored, hooks disabled, read-only sandbox and approval policy `never`.
`STASIS_KEEPER=1` remains the sensor's early recursion guard. Combined usage
comes from JSON `turn.completed` input/output tokens; cached input is already
included. Missing usage remains unmeasured. C1/C2 are unbound in the current
Codex capability table and are skipped with evidence; C3/C4 use their declared
bindings. This does not change the incident or shift budgets.

The 2026-11-02 device probes used Codex 0.166.0. An actual shell write was denied
by its read-only sandbox, and a separate real session received a marker through
the projected PostToolUse hook and repeated it in its final reply. The latter
used isolated fixture authority and one-off trust of vetted fixture hooks,
not production policy activation. Measured hook calls were 0.09–0.32 seconds.
Actual coordinator consultations returned validated decisions: Claude C1,
13.12 s / 11,163 tokens; Codex C3, 39.16 s / 17,489 tokens, followed by C4,
48.15 s / 18,025 tokens and its steward ask. Native authority in
those consultation probes was a fixture. Private evidence and raw transcripts:
`genesis/local-dev/stasis-runtime-wiring/`.

On this device, native stasis tests passed 35 checks and the selected immutable
candidate reports the production policy `unpinned`. The coordinator runs in
shadow with that explicit candidate; four real Codex sessions have emitted
private events. `runtime.env`, `policy-pin.txt` and `activation.md` in the
private evidence directory contain the tested selection and steward activation
steps. The shared installed native head was preserved after its governed
follow refused a non-descendant source head.

Story-harvest station: dialogue-stasis / between hook registration → production
guidance / missing node: the runtime trusts the registered hook, the selected
native binary reports `pinned-match`, and its coordinator's guidance reaches
the intended session reader / probe: native status plus events, consultation
usage and delivery receipt in that session / current state: red; real capture
and isolated delivery are measured, production pin and live delivery pending.

SessionStart starts the daemon idempotently and emits one context line containing
effective mode, policy state, coordinator liveness and incident count. Unreadable
counts are `unknown`, never zero. It teaches parked sessions to continue
independent authorized work and never retry the parked effect. The runtime's
existing `harnessCapabilities` gates waking; no new capability is claimed.

The PermissionRequest transport is exactly:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "deny",
      "message": "parked as stasis `<id>`. Do not retry this effect by any tool or spelling. Continue independent authorized work. Guidance will arrive."
    }
  }
}
```

`stasis-probe`'s recorder emits this envelope. Report
`probe/20261030T145038Z` proves `deny-message-reaches-agent[host|none]` and
`deny-continues-turn[host|none]`: the denied command did not execute and the next
echo did. Report `probe/20261030T163031Z` proves idle wake via exit 2 and its
system-originated prompt. Channel deny-relay and Remote Control push remain
unsupported; a displayed collaborative dialog must wait for human closure.

Local stub measurements on 2026-10-31 (five samples per synchronous registration,
including process startup; seconds) set the wiring timeouts:

| Registration                                             | Largest measured | Timeout |
| -------------------------------------------------------- | ---------------: | ------: |
| PermissionRequest                                        |            0.455 |       3 |
| Notification permission_prompt / idle_prompt             |    0.204 / 0.183 |   3 / 3 |
| PreToolUse                                               |            0.330 |       3 |
| PostToolUse / PostToolUseFailure                         |    0.328 / 0.135 |   3 / 3 |
| PermissionDenied                                         |            0.182 |       3 |
| Stop                                                     |            0.311 |       3 |
| UserPromptSubmit                                         |            0.378 |       3 |
| SessionEnd                                               |            0.407 |       3 |
| SessionStart, available / unavailable (original fixture) |    0.412 / 0.869 |       3 |
| Stop asyncRewake, guidance ready                         |            0.512 |    1870 |

The async timeout includes a **configured** 1860-second idle watch, covering the
1800-second incident ceiling plus a minute for delivery; a full 31-minute idle
wait was not measured. The measured row is wake latency, not a claim that the
whole watch ran. Internal native decisions share a 1.9-second budget including
`epr_client` resolution and kill the whole process group on timeout. SessionStart
runs daemon startup and status concurrently, bounded to 0.65 and 1.9 seconds
respectively, under a three-second registration in both runtimes. Live hooks
fail open on lock contention. These are fixture measurements, not model or
deployed harness latency. Private raw samples: `genesis/local-dev/dialogue-stasis-slice9/hook-latency.json`.

`STASIS_LADDER=off` preserves the slice-3 stdout byte for byte, including its
already-existing 3b exception relay, while sensing continues. It disables new
control delivery, blocks and wake requests. `STASIS_SENSE=off` exits before
reading stdin or writing anything. SessionStart's new awareness line still
reports `off`; it is not an old slice-3 event. Stopping the independent daemon
remains `stasis-coordinator stop`.

Activation measurement on 2026-11-02 exposed an unused binary hash on every
stasis lookup: resolving the explicit 120 MB candidate cost 0.99 seconds before
native validation took another 0.86–0.93 seconds. Execution-only callers now use
`epr_client.resolve_binary`; identity consumers still calculate the CID. With
the real steward pin, isolated guidance replay took 1.18 seconds and the Codex
startup hook reported live/pinned in 1.25 seconds. These are device measurements,
not guaranteed latency under every load. Evidence:
`genesis/local-dev/stasis-runtime-wiring/activation-delivery-after.json` and
`activation-hook-output.json` in that directory.

## Delivery contract and remaining native seam

`delivery.jsonl`, beside each session's `events.jsonl`, records live guidance,
park lifecycle, alert requests and delivery receipts under the same
`journal.lock`. It is device-private, has no DHT head, and keys records by the
existing incident id and a local sequence. Shadow `would-have` records never
become deliveries. Claims and delivery records name the policy CID; changing
the pin suppresses old guidance. Receipts are claimed before output, so a crash
can leave delivery unconfirmed but cannot repeat an alert or Stop block. The
native budget fold remains authoritative; unknown costs stop further spending.

Each reader has its own guidance receipts. The main and subagent sessions cannot
consume each other's messages. A terminal park (`answered`, `withdrawn`,
`expired`) never reopens or grants execution authority. A matching completed
AskUserQuestion marks its items answered; tool errors do not. Later execution
needs fresh validation. No automatic park expiry is invented.

Interpretations where the spec lacks a wire contract:

- Stop is the available evidence that the turn has exhausted independent work.
  It blocks once per batch, includes all pending items, and respects
  `stop_hook_active`. An uncertain question never blocks. Collaborative C1
  begins at 180 seconds; further collaborative rungs wait for 300 seconds.
- Optional `learnedForms` entries carry `name`, `tool`, redacted `shape`, native
  `effectClass`, `legibleForm`, `status: active`, `auditVerdict: approved`,
  `auditRef` and `rulingRef`. No entry exists today. The pinned policy must name
  those bytes and native classification must return `shape` with `ruleMatch:
none`. Proposed forms, ask matches and uncertain inputs never reshape.
- The native keeper answer has no validated, scoped ruling receipt. Every
  `proceed` therefore downgrades to park. Peer text and a ruling id in a keeper's
  reason do not establish standing.
- PushNotification is a once-only in-session request, not proof of phone
  delivery. `berth say --session SESSION TEXT` is the independent fallback;
  its bounded attempt and success/unknown receipt are retained per stage.

The current native source implements automated deny with `incidentId`,
`policyCid`, `shadow: false` and a nonempty message, and validates optional
learned forms. An older installed binary can still return `effect: none` or
omit the policy status fields; check the actual executable before activation.
Scoped, unexpired steward-ruling validation remains required before delivering
`proceed`; the coordinator continues to downgrade it to park.

Story-graph seam: dialogue-stasis / between native policy pin → measured live
closure / missing node: native replay recognizes an adapter's parked closure
and separate pending item without counting the session as still blocked / probe:
replay the slice-9 fixture through native `fold` and validate its event schema /
current state: unverified; the inspected v1 schema only admits approved,
denied-inferred and unknown prompt closes. Live closures therefore remain in
the delivery journal; the adapter records `native-park-fold-pending` instead
of writing an invalid event or inventing a human denial. Native reconciliation
must absorb that delivery lifecycle before blocked-time measures can claim
complete live coverage.

## Away rulings

Automated presence can come from a steward's **away ruling**: an `epr flow note --kind ruling` whose
reason is JSON naming BOTH the purpose and an expiry, for example
`{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T23:00:00Z"}`. The session's presence
carries the ruling note's CID as its `rulingRef`. The native reader counts a ruling only when it is
authored by a person steward of the collective, its `purpose` is exactly `dialogue-stasis-away`,
and `expiresAt` is a parsable RFC 3339 time not yet passed. A ruling with no expiry, an unparsable
one, another purpose, or a reference the sidecar cannot resolve is **not** away: the regulator
never parks on an absence it cannot verify.

## Live regulator records

Records the live regulator writes carry `agentId: "stasis-regulator"`,
`regulator: {"kind": "native-decide"}` (the coordinator writes `"coordinator"`) and the `policyCid`
they were decided under. The fold counts such a record as live (and closes a parked stall) only
when the regulator kind is known and `policyCid` equals the policy pinned at fold time; otherwise it
folds as a shadow record. This binds provenance only as strongly as control over journal writes,
the device trust boundary; it is not a signature.
