---
title: Dialogue stasis loop — a lab-neutral hierarchy of concern for session stalls
id: dialogue-stasis-loop-design
status: Draft
class: architecture
serves: dialogue-stalls-are-answered
date: 2026-10-30
context-tier: disclosed
steward: human:matthew
graduation-trigger: Two shifts, including one unattended shift, remain under the dialogue-blocked-seconds ceiling with a falling share of incidents reaching the human and zero independent-audit faults.
cites:
  - genesis/docs/retrospectives/2026-10-30-overnight-integration-pass.md
  - "elohim-operator-loop-wedged-peer-design | The elohim-operator loop when a peer wedges | sha256:8329c39fe67980a6 | path: genesis/docs/superpowers/specs/2026-10-09-elohim-operator-loop-wedged-peer-design.md"
  - "requisite-variety-guidestar-epr-family-composition | Requisite variety as the guidestar | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
  - "algedonic-feedback-signal | Algedonic Feedback Signal | sha256:cf902beac8dd4aec | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - elohim/epr/src/algedonic.rs
  - elohim/epr/src/verdict.rs
  - elohim/epr-rea/src/model.rs
  - elohim/eprfs/device-runtime/src/session.rs
  - .claude/skills/agentic-developer/SKILL.md
  - genesis/agentic/agent-runtimes.json
  - elohim/sdk/domains/elohim-agent/schemas/hook-package.schema.json
  - elohim/sdk/domains/elohim-agent/schemas/session-event.schema.json
  - genesis/agentic/.epr-meta/dialogue-stalls-are-answered.habit.md
  - .claude/epr-meta/measures.yaml
---

# Dialogue stasis loop

## Problem

The [2026-10-27..30 overnight integration retrospective](../../retrospectives/2026-10-30-overnight-integration-pass.md)
describes a steward pressing keys to unstick prompts, six "stuck?" pings, and agent
turns ending with "should I retry?" The retrospective is on another branch. These
are stalls in the session dialogue itself: the human supplies the missing loop by
noticing silence, reopening a decision, or releasing a waiting turn.

The loop covers three classes. `authorization` means a permission dialog is open;
`question` means the agent ended its turn asking something; `silence` means the
steward is waiting on a quiet agent. It tries the cheapest sufficient capability
before interrupting the human. When the human is not answering, it parks the item
and lets independently authorized work continue. It never approves on the human's
behalf. Incident opening and harvesting follow the timing rules in Terms below,
which also define Approach and Breach.

The retrospective cross-references used by this design are:

| Row | Concern |
|---|---|
| 63 | Dev-stall handoff when a peer waits on a lease. |
| 64 | Waiting discipline and gate progress notes. |
| 67 | Host RAM/load budget for gates. |
| 68 | The pause ledger and allow-rule ratchet, which this loop absorbs. |
| 69 | This dialogue stasis loop. |

## Terms and reused primitives

**EPR** means Elohim Protocol Record; **`epr`** is its CLI. **eprfs** is the
device-local, content-addressed store and runtime that the `epr` CLI operates on.
The **steward** is the human who holds authority for this workspace's sessions,
here `human:matthew`; the steward's rulings are `--kind ruling` notes. A
**standing ruling** is a steward `--kind ruling` note that is unexpired and whose
declared scope covers the case at hand. Presence rulings select handling and
grant no execution authority.

A **berth** is the device's shared lease and note channel
(`genesis/agentic/bin/berth`). A berth note is written with `berth say`, read
with `berth inbox`, and crosses to the steward's other devices through
`epr peer serve`.

A **session incarnation** is the device-runtime `SessionIdentity`: runtime,
session id, and owner pid plus start time where known. It distinguishes restarts
and helps avoid confusing process incarnations; it is not authentication.

A **shift** is the accounting window for dialogue measures and per-shift budgets.
Inside an /agentic-developer shift (`/shift`), it runs from kickoff to close;
the skill writes a `shift-marker` journal event at each edge (slice 10 wires
this). Everywhere else, it is the session incarnation's window, from its first
journal event to `session-ended`, split into consecutive 24-hour windows if the
session runs longer. `fold --shift-current` resolves the latest open
`shift-marker` window for the session if there is one, otherwise the current
session window. The event shape is defined in Journal contract and measures.

A **projection** generates runtime files from their source package.
A **projection fixture** is a checked-in expected projection output under
`.epr-meta/elohim/projections/{claude,codex,antigravity}/`, compared by
`package-projections.mjs verify`.

A **fold** is a deterministic replay of journal events into computed state:
incidents and measure readings. **The unmeasured rule**: missing, retained-away
or unfolded evidence is reported as unmeasured or skipped, never as a measured
zero.

**Opening an incident** is the fold recognizing a stall from its first journal
event. **Harvesting** is the coordinator taking an incident's unresolved work
into the ladder: it applies rung 0, then writes the first rung-1 consultation claim.

**Timing rules:** in the collaborative path, the incident opens when the prompt
opens and harvesting begins at Approach (3 minutes); Breach follows at 5 minutes.
In the automated path, harvesting begins when the incident opens. Authorization
interval evidence is specified under Folding and measures.

The Viable System Model (VSM) separates five jobs: **System 1 (S1)** performs the
work; **System 2 (S2)** coordinates it and prevents repeated, conflicting reactions;
**System 3 (S3)** manages the current operation within delegated authority;
**System 4 (S4)** learns and proposes adaptations; **System 5 (S5)** holds policy
and the authority boundary. **S3*** ("S3-star") is VSM's sporadic audit channel:
an independent audit of what actually happened, separate from the operation's
own account.

A **`Bound`** declares a limit through `limit`, `unit`, `threshold_pct` and
`sense` (`Ceiling|Floor`); an absent sense means Ceiling. `threshold_pct` is a
percentage, not a fraction. Reuse this type for declared limits
(`Bound`, `elohim/epr-rea/src/model.rs:115`; `Sense` at line 145).

An **algedonic** signal reports that a system is approaching or crossing a limit
on its ability to keep working. **Approach** means care must increase before the
limit; **Breach** means intervention is needed. **Recovered** is a transition of
the algedonic latch, never an incident state; its contract is in
AlgedonicSignal and its latch below. **Dwell** is the time recovery must persist
before that latch resets. The Incident model separately defines stall closure.

A **lab** is an AI-model provider; **lab-neutral** means independent of any
provider's model names. A **capability level** is an ordinal reasoning capacity:
C1 light, C2 standard, C3 deep and C4 frontier.

A **presence disposition** is the handling context derived from the
human's last witnessed interaction, a Breach witnessing absence, or an expiring
away ruling.

A **judge** is the /agentic-developer skill's progress evaluator. Its `stall`
means no measured change over two or more iterations, distinct from this loop's
`silence` stall class. A **rail** is a declared bound a shift is held to; the
dialogue rail sits alongside the CI floor and ceiling rails.

A **CID** is a content-derived identifier: the hash of the bytes, using
multihash/CIDv1 as across eprfs. **`sensitive-file-protection.py`** is a PreToolUse
hook that warns on sensitive paths; path matching does not block an edit.

A **keeper** is a headless, read-only subprocess at one capability level that
advises on one harvested incident. Its only output is a `Decision` with a pivot.

Keeper output reuses the protocol **`Decision`**: **Permit** means the proposed pivot
is admissible within existing authority; **Refer** advances to the next rung,
ending at rung 5, the steward. Permit is not tool permission. The keeper never
returns `Refuse` on the steward's behalf. An unanswered Refer stays pending.

A **pivot** is the operational advice beneath that decision: `reshape`, `park`,
`proceed`, or `notify`.

Reuse `SessionIdentity` from device-runtime, `Bound` from epr-rea, `Decision`
from epr, and `epr flow note --kind correction|ruling|verdict`. The existing
`AlgedonicSignal` and `should_emit` provide caller-held emission memory without
reset or a production caller; slice 4 adds the shared recovery behavior. The
wedged-peer design numbers its design elements E1, E2 and so on (its §4). Its **E1** is
the algedonic-state element (Approach, Breach, Recovered and persisted dedup
keys), and **E4** is the born-narrow operator seat:
Permit only moves that add and remove nothing, everything else Refer, and an
unanswered Refer stays pending. The scope of reuse is specified in Current state
below.

## Current state of what this extends

### Agent packages

A native package is structured JSON at
`.epr-meta/elohim/packages/{agents,skills,hooks}/<id>.json`, with
`apiVersion: elohim-agent/v1alpha1` and `kind: AgentPackage`, `SkillPackage` or
`HookPackage` (the corresponding schemas under
`elohim/sdk/domains/elohim-agent/schemas/`, each at lines 7–10).
Projection dispatches by kind and mechanically writes runtime copies to their
declared paths (`projectedTextFor`, `package-projections.mjs:1168`, and
`writeRuntimeProjections`, `package-projections.mjs:1200`, both in
`elohim/sdk/domains/elohim-agent/scripts/`). A hook such as
`.claude/hooks/<id>.py` is byte-identical to `source.body`
(`projectHook`, `scripts/hook-package.mjs:155` in that domain). An agent such as
`.claude/agents/<id>.md` combines projected frontmatter with its instructions.
`AgentPackage.metadata.modelHints` is free-form
(`elohim/sdk/domains/elohim-agent/schemas/agent-package.schema.json:31`).
The algedonic-designer package carries `claudeModel: opus` and
`claudeColor: darkred` (`.epr-meta/elohim/packages/agents/algedonic-designer.json:10`).
These are Claude-only model/color hints today, even though the projector copies
`claudeModel` into Codex frontmatter too; they do not express a lab-neutral
capability (`codexFrontmatterFromPackage`, `package-projections.mjs:724`).
Repository claude-code agents currently omit effort; Capability verification
table below establishes that the frontmatter field is supported.
Slice 2 adds `capabilityProfile`, runtime bindings and their projection.

### HookPackage registration

The schema defines a single-object-or-null `registration`; its `registrations`
object maps each runtime to that same single-object shape, not a list of event
registrations (`elohim/sdk/domains/elohim-agent/schemas/hook-package.schema.json:56`
and line 67). `registrationFromSettings` returns only the **first** matching
settings entry (`registrationFromSettings`, `elohim/sdk/domains/elohim-agent/scripts/hook-package.mjs:56`).
`normalizeRegistration` compares event, matcher, command, timeout and type, but
not `async` (runs the hook without blocking) or `asyncRewake` (exit 2 wakes an
idle session) (`normalizeRegistration`, `hook-package.mjs:89`). Reconciliation is
read-only with respect to runtime settings: it checks drift and can lodge a
finding, but never writes `settings.json` (`verifyHookPackage`, `hook-package.mjs:165`).
Slice 2 changes the single registration to a list, records and reconciles
**every** entry, and compares both async flags; settings wiring remains the
steward's act. These illustrative fragments show that change for one runtime,
before:

```json
{"registration":{"event":"PostToolUse","matcher":null,"command":"python3 .claude/hooks/dialogue-stasis-sense.py","timeout":5,"type":"command"}}
```

After slice 2:

```json
{"registration":[
  {"event":"PostToolUse","matcher":null,"command":"python3 .claude/hooks/dialogue-stasis-sense.py","timeout":5,"type":"command","async":false,"asyncRewake":false},
  {"event":"Stop","matcher":null,"command":"python3 .claude/hooks/dialogue-stasis-sense.py","timeout":120,"type":"command","async":true,"asyncRewake":true}
]}
```

### AlgedonicSignal and its latch

The current kinds are `AlgedonicKind::{Approach, Breach}`
(`AlgedonicKind`, `elohim/epr/src/algedonic.rs:96`). `AlgedonicEvidence` requires stock, limit
and bound reference, plus `threshold_pct` for Approach (`AlgedonicEvidence`, `algedonic.rs:140`).
`Severity` is Info/Warn/Critical, with Warn as default (`Severity`, `algedonic.rs:119`).
`AlgedonicSignal` carries target, declarer, evidence, severity and signed time;
its kind is derived from evidence (`AlgedonicSignal`, `algedonic.rs:233`).
`OpenSignalKey` is `(declarer, target, kind)` (`OpenSignalKey`, `algedonic.rs:319`).
The caller-held `should_emit` latch emits only after the evidence crosses its
threshold and suppresses an already-open key; a Breach can still follow an
Approach because their keys differ (`should_emit`, `algedonic.rs:381`).
It has no reset threshold, dwell time or persistence.

Only the algedonic latch module in `elohim/epr/src/algedonic.rs` is shared with
wedged-peer E1. Slice 4 extends that file with `Hysteresis`, `RecoveryEvidence`,
and a persisted latch with `observe`. The change is additive: existing public
items are unchanged and `should_emit` stays.
The wedged-peer DHT link, `CommitmentByState` in mishpat, is not part of this
loop. The following slice-4 contract is complete for the shared module; a
wedged-peer consumer calls `observe` with its own stocks. The shapes name the
required data and behavior; final Rust naming belongs to the implementer.

- `AlgedonicKind` gains `Recovered`.
- `Hysteresis { threshold_pct, reset_pct, dwell_s }` declares entry, reset and
  dwell. For a Ceiling bound, `reset_pct` must be below `threshold_pct`; for a
  Floor bound, it must be above it. Recovery requires the stock to remain on
  the safe side of the reset line (`limit × reset_pct / 100`, since percentages
  use whole-percent units) for `dwell_s` before the open key is cleared: below
  the line for Ceiling, above it for Floor.
- `RecoveryEvidence { stock, limit, bound_ref, reset_pct, crossed_back_at,
  dwell_s, cause_ref }` records that recovery. `cause_ref` names the event that
  moved the stock back; here it is the incident's closing event.
- The new persisted latch provides an alternative to caller-held memory. It keeps open keys with
  `{kind, since, safe_since}`, and `observe(stock, now)` returns at most one
  transition: Approach, Breach or Recovered. Re-crossing the reset line during
  dwell cancels `safe_since`; a later safe crossing starts a new dwell. The
  latch serializes its state to a caller-supplied path so a restart does not
  re-emit.

Slice 4 also adds
`elohim/sdk/schemas/v1/feedback-signals/algedonic-recovered.schema.json` beside
the existing `algedonic-approach.schema.json` and `algedonic-breach.schema.json`.
It follows their signal envelope and carries the recovery evidence above.

For a dialogue incident, stock is the open stall's current duration in seconds.
The limit is the Breach bound, 300 s; Approach is at 60%, or 180 s. Closing the
stall sets stock to 0 and supplies the close event as `cause_ref`. For this
consumer, recovery requires `dwell_s` with no new stall on the same key. A new
stall on that key during dwell cancels recovery and resumes the same signal,
even before the new stall reaches Approach; it does not emit a fresh Approach.
This is the anti-oscillation contract:

```text
Clear --Approach or Breach--> Signal open
Signal open --incident closes; stock = 0--> Dwelling
Dwelling --new stall on same key--> Signal open (resume)
Dwelling --dwell_s without a new stall--> Clear (emit Recovered)
```

Shift-level blocked seconds never decrease: their ceiling is per shift and
resets only when a new shift begins.

### Inherited wedged-peer provisions

The existing design at
`genesis/docs/superpowers/specs/2026-10-09-elohim-operator-loop-wedged-peer-design.md`
provides three contracts this loop inherits; they are design provisions, not a
claim that their runtime is already implemented.

- Approach and Breach are two consumer contracts: Approach raises care and
  prepares consent before anything moves; Breach executes the agreed plan, or
  Refers with evidence when there is no Approach-time plan (§2, line 93).
- E1 reuse is limited to the shared latch contract above (§4 E1, line 180;
  lines 197–199); this journal remains private.
- The E4 authority boundary defined in Terms applies to keeper advice without
  granting tool permission (§4 E4, line 305; lines 321–327).

### gate-live-inject

The hook checks whether the session's run log has grown since a cursor keyed by
both session and reader/agent (`has_news`, `.claude/hooks/gate-live-inject.py:47`).
`unseen` asks `gate-live.mjs` for new entries and advances that reader's cursor
(`unseen`, `gate-live-inject.py:61`). The PostToolUse hook selects `agent_id` (or
`main`) and returns the new lines as `hookSpecificOutput.additionalContext`,
so the agent sees them at its next tool call (`main`, `gate-live-inject.py:139`
and `gate-live-inject.py:152`). This design reuses that delivery pattern for guidance.

### ram-guard

The daemon exposes `start`, `stop`, `status [--brief]`, `plan`, `run` and
`once [--dry-run]` (`main`, `genesis/agentic/bin/ram-guard:729`).
Its store is `$RAM_GUARD_DIR`, otherwise
`${CLAUDE_CONFIG_DIR:-/projects/.claude-config}/ram-guard`, containing
`state.json`, `events.jsonl`, `guard.log` and `guard.pid` (`store_dir`, `ram-guard:401`).
`run` holds a flock (a POSIX advisory file lock) on `guard.pid`; idempotent
`start` double-forks and uses `setsid` to detach (`Daemon.run`, `ram-guard:598`,
and `cmd_start`, `ram-guard:648`).
SessionStart registers `.claude/hooks/ram-guard.py --event session`
(`.claude/settings.json:37`). The hook calls `ram-guard start` and emits status
(`ensure_daemon`, `.claude/hooks/ram-guard.py:289`, and `main`, `ram-guard.py:402`).
Its `<store>/seen/<session_id>` cursor records the number of event lines seen,
independently for each session (`_cursor_path`, `ram-guard.py:219`). This daemon, ownership,
state and cursor pattern is the coordinator's template in slice 8.

### /agentic-developer

The existing rules live in `.claude/skills/agentic-developer/SKILL.md`:

- Before bailing, research the governing spec/plan, trajectory, prior art and
  findings in order; evidence permits proceeding only within the rule's
  reversible, in-scope action boundary: P3 (from the /agentic-developer skill), line 22.
- Check the palette before every Bash call; a command with no match is **not
  attempted**, and goes to the wishlist discipline or a workaround: P5 (from the /agentic-developer skill), line 32.
- Do not edit the running shift's judge, with only the stated narrow, declared
  exception for restoring a structurally unparseable fixture: P6 (from the /agentic-developer skill), line 33.

Wishlist categories are **Redirect** (an approved alternative), **Wishlist**
(convenience with a workaround), and **Blocker** (progress stopped)
(`SKILL.md`, lines 353–361). The judge's `stall` means no measured delta over
two or more iterations (`SKILL.md:228`).
Slice 10 composes dialogue handling with these rules; rung 0 preserves P5 and
adds learned legible forms on top. These are two real entries from
`.claude/settings.json`'s `permissions.allow` list:

```json
["Bash(python3 .claude/scripts/habits-project.py *)", "Bash(node genesis/orchestrator/gate-runner.mjs *)"]
```

`matchesPalette` normalizes `:*` to `*` and globs allow entries with picomatch,
rejecting any command containing control characters or newlines
(`matchesPalette`, `genesis/agentic/palette.mjs:18`; `loadPalette`, `palette.mjs:43`).
It is not the permission engine; the risk classifier's ask/deny matcher is
specified under Classification.

### Measures, lenses and epr flow sense

A versioned measure row is an observation-only record of family, unit, procedure,
environment sensitivity and subsumption (`.claude/epr-meta/measures.yaml:15`).
A **lens row** binds one or more measures to a context, bounds and an enforcement
class (`deny|ask|inject|measure`); enforcement belongs to that binding
(`measures.yaml`, lines 36–64). **`epr flow sense`** is the native command that
runs a measure's declared `probe` and folds its result through the observation
writer (`sense`, `elohim/eprfs/epr-cli/src/flow/sense.rs:444`; registry header lines 66–73).
Lens readings follow the unmeasured rule in Terms (registry header lines 55–59;
`OutcomeStatus::Skipped`, `elohim/eprfs/epr-cli/src/flow/report.rs:191`).
An explicit reset with no later readings is evidence that can establish zero.
This design adds dialogue measures
and their ceiling, then wires the journal fold in slice 5 and the health lens in
slice 11; declarations alone provide no reading.

## P2P design gate

| Question | Answer |
|---|---|
| Classification? | **Private (B)**: the journal belongs to the device's session; peers do not validate its raw dialogue evidence. The authoritative local journal is `.eprfs/status/stasis/<session>/events.jsonl` and its retained segments. |
| Existing DHT entry type? | No DHT (distributed hash table) entry type is used or added. This journal does not require network notarization. |
| Head-plane cost? | None: zero new replicated heads at birth or at one year, no per-item reconciliation work, and no added quiesce cost. Local journal retention is bounded. |
| Identity? | Session incarnation plus atomic sequence number. Runtime and session id are required; owner pid and owner start time, when known, guard against pid reuse. Missing process evidence remains absent or null. This is a local composite identity. |
| Coordinator zome and projection signal? | No zome, the Holochain module that authors network entries, and no post-commit signal. Local folds reconstruct incidents, rung state and measures from the journal. No HTTP route or network sync projection is added. |

Only measures leave the device. Raw events and redacted shapes stay private.

## Capability vocabulary and bindings

`AgentPackage` gains a `capabilityProfile`. Its dimensions are:

| Dimension | Values | Meaning |
|---|---|---|
| `level` | C1 light, C2 standard, C3 deep, C4 frontier | Ordinal reasoning capacity. |
| `effort` | low, medium, high, xhigh | Reasoning budget within a level. |
| `authority` | read-only-nonprompting for every keeper | The keeper reads and advises without writing or raising permission prompts. |
| `independence` | same-family, other-family | Other-family is required for S3* audits to preserve plurality of judgement. |

Each runtime in `genesis/agentic/agent-runtimes.json` gains `capabilityBindings`.
These are declared placeholders, subject to probing. **fable** is a Claude model
alias, bound in the table to `claude-fable-5-1`:

| Runtime | C1 | C2 | C3 | C4 |
|---|---|---|---|---|
| claude-code | haiku | sonnet | opus | fable → claude-fable-5-1 |
| codex | unbound | unbound | gpt-6-astra, high | gpt-6-astra, xhigh |
| gemini | unbound | unbound | unbound | unbound |
| antigravity | unbound | unbound | unbound | unbound |

At consultation time an unbound level is skipped and the skip is recorded. Slice 2's
projector takes `capabilityProfile`, the target runtime and that runtime's
`capabilityBindings` as inputs. For claude-code it emits agent frontmatter
`model: <alias>` and `effort: <level>`, using the verified frontmatter support
in the capability table below. For codex it emits `model` and
`model_reasoning_effort`. An unbound level
omits the model field, records `capability-unbound` in the projection fixture, and is
reported by verify as a warning. During migration, a package with no
`capabilityProfile` falls back to `modelHints.claudeModel` unchanged. Those
free-form hints remain migration debt; they do not define capability. The
protocol-native model-fit layer is a frontier described below, outside this
loop's implementation.

## Capability verification table

The reported headless result from `genesis/agentic/bin/stasis-probe` on Claude Code
2.1.314 is **30 checks `yes`**, with behaviour model haiku. The probe runs
`claude -p` in a scratch project outside any checkout with a recorder hook on
every event. The completed `genesis/agentic/bin/stasis-probe --only effort`
probe writes a scratch `.claude/agents/probe-effort.md` with `effort: low`, has
the parent dispatch it, and reads `effort.level` from subagent hook payloads
identified by `agent_id`. On 2.1.314 the subagent's events read `low` and the
parent's read `medium`: claude-code agent frontmatter supports `effort:`.
The reported effort evidence is
`genesis/local-dev/dialogue-stasis/probe/20261030T155306Z/report.json` (not present
in this worktree). The six *interactive* capabilities
listed below are outside that headless result and remain **`unsupported`** until
the steward-assisted `stasis-probe --interactive-setup` / `--interactive-read`
probe proves them. Every other capability not marked `yes` also stays unsupported
for later adapters. The steward's declaration that Remote Control is on does not
establish any of these behaviours.

| Claude Code dependency | Status (2.1.314) |
|---|---|
| PermissionRequest fires for a call that matches an `ask` rule, after that call's PreToolUse | yes |
| A PermissionRequest `deny` pre-empts the prompt (with `--permission-prompts none` and `host`) | yes |
| PermissionRequest carries no `tool_use_id`; it does carry `prompt_id`, `tool_name`, `tool_input`, `permission_mode` | yes |
| Events in one user turn share `prompt_id`; PreToolUse and PostToolUse carry `tool_use_id`. Matching a PermissionRequest to the preceding PreToolUse of the same tool and `prompt_id` holds only for serial calls. A parallel batch or identical inputs must yield low confidence or an observation gap, never a guessed match. | yes (observed serial ordering only) |
| A `deny` with a `message` reaches the agent verbatim, and the turn continues (the next call ran) | yes |
| PostToolUse carries `duration_ms` | yes |
| Stop carries `last_assistant_message`, `background_tasks`, `session_crons`, `stop_hook_active` | yes |
| `claude -p --output-format json` reports `permission_denials` with `tool_use_id` | yes |
| Model aliases bind: haiku → claude-haiku-5-5, sonnet → claude-sonnet-5-5, opus → claude-opus-5-5, fable → claude-fable-5-1 | yes |
| Agent frontmatter `effort:` sets the subagent's effort | yes |
| The keeper shape `claude -p --restricted --permission-prompts none --tools Read,Grep,Glob --settings <file> --model <bound>`, with `STASIS_KEEPER=1` in the environment, has no Bash, raises no PermissionRequest, ignores project hooks and runs only the explicit settings' hooks | yes |
| `STASIS_KEEPER=1` is visible to hooks, so an adapter can ignore keeper events (`hook-sees-keeper-env`) | yes |
| Notification `permission_prompt` fires about 6 s after a dialog displays | yes (6.2 s after PermissionRequest; it does not fire for a dialog answered within that window) |
| A human deny emits no event at all: no PostToolUse, PostToolUseFailure, PermissionDenied or Stop for the denied call. The close is visible only through the agent's next event | yes |
| An approved dialog closes at the matching PostToolUse time minus `duration_ms` | yes |
| Notification `idle_prompt` fires about 60 s after a turn ends | yes (60.2 s after Stop) |
| `idle_prompt` does not fire while a permission dialog is open (held about 70 s) | yes |
| An `async` plus `asyncRewake` Stop hook exiting 2 wakes an idle session | yes (woke 30 s after Stop; the woken turn ran and stopped with `stop_hook_active: true`) |
| A rewake (and any background-task notification) reaches hooks as a UserPromptSubmit whose prompt begins with `<task-notification>` | yes |
| Remote Control pushes a displayed `AskUserQuestion` | unsupported — needs the steward's phone |
| A channel can relay a deny to close an already displayed dialog | unsupported — research preview, not probed; never an approval |

The interactive rows were driven from inside a session with `stasis-probe --interactive-drive`. That runs a real interactive `claude` under a pseudo-terminal in a throwaway project and presses the keys itself: deny one dialog after about 70 s, approve one at once, then sit idle. Report: `genesis/local-dev/dialogue-stasis/probe/20261030T162945Z/report.json`. Three design consequences follow:

- **Presence witness.** A UserPromptSubmit whose prompt begins with `<task-notification>` is system-originated (a rewake, a background-task completion). It is never a witnessed interaction. Without this rule, every wake would look like the steward at the keyboard.
- **Denied close.** A human deny is invisible until the agent's next event, so `denied-inferred` closes at that event with low confidence, and the interval can overstate the steward's actual answer time. An `idle_prompt` that arrives with no tool event since the dialog opened shows the dialog had closed. The fold may use that as an upper bound, marked as an inference.
- **Dialog without notification.** A dialog answered within about 6 s produces no `dialog-displayed`. The fold falls back to `permission-requested` (see Folding and measures).
| Auto mode falls back to prompting after 3 consecutive or 20 total classifier denials, session-wide | documented, not probed |
| Subagent `dontAsk` mode | not needed: keepers run headless with the verified keeper shape above |

## Presence disposition

Witnessed interaction is a UserPromptSubmit, a dialog resolved by the human, or an
answered question. In the absence of a stronger absence observation or an away
ruling, `collaborative` means the last witness is within **the collaborative
window (T_c)**, the passive bound of 10 minutes by default. `automated` applies
when that bound has passed, a declared, expiring away ruling applies, or the
Breach rule below applies. `/shift`, `/loop` and unattended kickoff record the
away ruling. Unknown evidence is recorded as `unknown`, without inventing a
witness or execution authority.

Every event and every decision records the disposition and its basis: the age of
the last witnessed interaction and the ruling reference, each nullable when
unavailable. A 5-minute Breach on an unanswered displayed dialog is **witnessed
absence**: it flips the **session's** disposition to automated, including later
incidents, and holds until the next witnessed interaction resets it. That Breach
is an active observation of absence and takes precedence over the passive
10-minute `T_c` bound; it is not merely an incident-local timeout. A witnessed
interaction returns the session to collaborative unless an away ruling is still
in force. Expiry of that ruling removes its override; it does not invent a new
witness or clear an unanswered-dialog Breach.

```text
                  T_c expires (10 min since last witness)
                  OR unanswered displayed dialog reaches Breach
                  OR away ruling begins (holds until expiry)
  collaborative ------------------------------------------------> automated
                <------------------------------------------------
                  witnessed interaction, unless away ruling in force
```

## Ladder and independent coordinator

| Rung | Actor | Task |
|---|---|---|
| rung 0 | Deterministic regulator | Preserve P5: a Bash command that does not match the palette is not attempted. On calls that reach PreToolUse, recognize a learned shape with a more legible form, deny submission and name that form before permission handling. Learned forms still pass the palette and permission checks. |
| rung 1 | `stasis-keeper` at capability level C1 | Seek a simpler, safer, more legible step that is independently authorized, or other authorized work. |
| rung 2 | `stasis-keeper` at capability level C2 | Reconsider an unresolved incident with greater reasoning capacity. |
| rung 3 | `stasis-keeper` at capability level C3 | Reconsider an unresolved incident with deeper reasoning. |
| rung 4 | `stasis-keeper` at capability level C4 | Reconsider the incident and prepare a one-line ask with a recommendation. |
| rung 5 | Steward | Answer the prepared, genuinely blocking ask. |

Rungs 1–4 are keeper consultations at capability levels C1–C4 respectively.
"The human rung" always means rung 5.

The coordinator is the independent agent that comes in after a stall. One daemon
per device at `genesis/agentic/bin/stasis-coordinator` owns every incident of that
device's sessions. It follows ram-guard's daemon pattern: start/stop/status, a
flock-held pid file enforcing a single instance, per-session cursors and
SessionStart launch. Ownership, cursors and heartbeat are journal records.
Later it is declared under `epr device supervise`.

Consultations run in rung order. The ladder advances only when a consultation's
result is recorded or its deadline expires; the working agent's retries do not
advance it. The Incident model and Budgets below define the folded state;
slice 8 owns the claim protocol. The coordinator
writes a heartbeat. If it dies, its state reads unknown, never green.

Keepers run headless with read-only tools through the verified binding shape:
`claude -p --restricted --permission-prompts none --tools Read,Grep,Glob --settings <file> --model <bound>`.
A codex other-family consultation uses `codex exec --sandbox read-only` only
after a runtime-specific probe establishes the same read-only, nonprompting
boundary. Their processes carry `STASIS_KEEPER=1` in the environment. A hook
process inherits the environment of the runtime process that spawned it, so
the adapter reads `os.environ` to ignore keeper events. The reported
`hook-sees-keeper-env` probe check verified this inheritance. A keeper opens no
incident and never recurses into the ladder. The coordinator's only writes are
the journal and berth notes; guidance
and notification requests are delivered through session hooks.

### Classification

`stall_class` names what is waiting: authorization, question or silence.
The classifier is native code in slice 5's `session_stasis.rs`, driven by the
policy's rule set of shape patterns and effect normalization rules. Slice 5
ships built-in defaults identical to the policy's initial rule set. Slice-3
adapters record only the raw inputs to this classification (`tool`, redacted
`shape`, and `source`); they never set `concernClass`. The fold classifies those
inputs into `concern_class`, which determines which pivots are admissible:

- `risk`: the call matches an explicit `ask` or `deny` rule in the active
  Claude Code settings, using the rule-matching subset below.
- `shape`: a PermissionRequest matches no `ask` or `deny` rule **and** matches
  a declared shape pattern: shell compounds `&&`, `;` or `|` around `cd`;
  heredocs `<<`; `bash -c`; or multi-line commands.
- `uncertain`: everything else, treated as risk. Absence of a matching rule
  alone never establishes shape.

The listed shape patterns are the initial rule set, not an exhaustive law.

The authority for rule syntax is the
[Claude Code permissions documentation, Permission rule syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax).
The native risk classifier implements this subset:

- A bare tool name matches every call of that tool.
- `Bash(<pattern>)`: `*` matches any run of characters. A trailing `:*` is the
  legacy prefix form, matching any suffix; normalize it to `*`, as
  `genesis/agentic/palette.mjs` does in `toGlob` for `matchesPalette`, but match
  against ask and deny rules rather than allow entries.
- `Read|Edit|Write(<path>)`: the path is a gitignore-style pattern.
- `mcp__<server>__<tool>` names accept `*` wildcards.

Precedence is deny > ask > allow; an allow match cannot override an ask or deny.
A rule the classifier cannot parse, or a specifier form outside this subset,
makes the call `uncertain`, which is treated as risk. When unsure, the classifier
errs toward risk. Slice 5's differential verification below checks this subset
against the permission harness.

`concern_class` is computed only for `authorization` stalls. A `question` or
`silence` stall always has `concern_class: uncertain`, so no same-effect reshape is
ever offered for it (reshape is shape-class only).

`effect_class` is a normalized tuple. For Bash it is the first command word
plus subcommand (for example, `git rebase` or `cargo test`), plus a target kind
decided by these rules, applied in order:

1. `remote`: the command is `git push|fetch|pull|clone|ls-remote` with a remote
   name or URL, or a network client (`curl`, `wget`, `ssh`, `scp`).
2. `worktree`: a path argument, or a `git -C` / `--manifest-path` / `cd` target,
   resolves under `<main checkout>/.worktrees/`.
3. `main-checkout`: such a path resolves under the main checkout root, outside
   `.worktrees/`. A command with no path argument runs in the session's working
   directory and takes that directory's kind.
4. `other`: everything else.

For Edit/Write it is the tool plus a path-class glob. For other tools it is the
tool name. Two effects are the same class exactly when their tuples are equal.

`shape` is the redacted command pattern. Arguments are replaced by a finite set
of placeholders: `<path>`, `<ref>` (a git ref or revision), `<url>`, `<num>`,
`<glob>` and `<str>` (any other argument). Flags are kept, because they carry the
risk. Secrets and argument values are never stored. For example:

| Command | Shape |
|---|---|
| `git -C /projects/elohim/.worktrees/x rebase --onto main feat` | `git -C <path> rebase --onto <ref> <ref>` |
| `cd app && pnpm exec vitest run foo.spec.ts` | `cd <path> && pnpm exec vitest run <path>` |
| `curl -s https://doorway-alpha.elohim.host/p2p/status` | `curl -s <url>` |

`inputDigest` (in `state.json` and the journal) is the SHA-256 of the hook event's
`tool_input` serialized as canonical JSON: sorted keys, no insignificant
whitespace. Two open prompts with an equal digest are "identical inputs" and are
correlated with low confidence.
The classification rules, including normalization and shape patterns, live in
the pinned policy file; the loop cannot widen them.

### Pivots

Permit admits a pivot within existing authority. Refer advances the ladder
without granting tool permission.

| Pivot or decision | Effect on the stall | What it writes | Precondition | What happens next |
|---|---|---|---|---|
| `reshape` | Offers ONE more-legible command with the same effect; no interpreters, `eval` or new tools. | The journal records the pivot and consumption of the incident's one reshape allowance, retaining only the redacted pattern. | Shape class only; the allowance has not been used by any tool or before any restart. | The command is delivered as guidance, the agent issues it, and it passes the unchanged permission checks. A second failure goes to Refer; no second reshape is offered. |
| `park` | Closes the stall with `closed_by: parked`; the session continues independent authorized work. | A separate pending item in the journal: `{incident id, one-line summary of the step, one-line ask, recommendation}`, without secrets or argument values. | The step remains unresolved and its ask can be stated; parking grants no authority to execute it. | The item surfaces in the batched `AskUserQuestion`. Its own lifecycle is `pending → answered \| withdrawn \| expired`. Expiry leaves the Refer unanswered, never a Refuse, without reopening the incident. Running the step later needs fresh validation. |
| `proceed` | Resolves a question under existing authority; it cannot release a permission prompt. | The consultation records the cited standing ruling id and validation result. | Question class only. The keeper cites a standing ruling id; the regulator verifies that it exists, is a `--kind ruling` from the steward, is unexpired and covers this question's scope. | Only after validation is "proceed under ruling R" delivered. Otherwise the pivot is downgraded to `park`. |
| `notify` | Calls attention to the stall; the incident stays open. | One berth note plus one `PushNotification` request per incident per stage (Approach, Breach), recorded for deduplication. | That stage has been reached and its notification has not already been recorded. | The incident remains open and the ladder continues; no acknowledgement or execution authority is inferred. |
| Refer | Advances to the next rung. | The journal records the reason and rung transition. | The current rung cannot resolve the stall within its authority, a consultation deadline expires, or a reshape fails. | Consult the next bound rung; at rung 5, the steward. An unanswered Refer remains pending. |

### Incident model

Slice 5's `session_stasis.rs` folds the journal into this shape:

```text
Incident {
  id: session incarnation + seq of the opening event
  stall_class: authorization | question | silence
  concern_class: shape | risk | uncertain
  effect_class: normalized tuple
  opened_at: { wall, mono }
  presence_at_open
  state: Open | Approach | Breach | Pending | Closed
  rung: 0..5
  consultations: list of {
    rung, level, binding
    decision: Permit | Refer
    pivot, reason
    started_at, finished_at | deadline_at
    claimed_by
  }
  blocked_intervals: list of { from, to | null }
  closed_by: approved | denied-inferred | answered | reshaped | proceeded |
             parked | withdrawn | expired | session-ended
  budget_used: { keeper_calls, elapsed_s, tokens }
}
```

The incident has only these states. The bracketed times are absolute, measured
from the incident's opening, not added together:

```text
Open [0 s] → Approach [180 s] → Breach [300 s] → Pending [rung 5 reached]
Any open state → Pending        (rung 5 reached early, e.g. a budget exhausted)
Open / Approach / Breach / Pending → Closed        (the stall is resolved)
```

Pending means rung 5 has been reached and the incident awaits the steward.
Any state goes to Closed the moment the stall is resolved, with the attribution
listed in `closed_by`; no dwell applies to closing an incident. Latch recovery
is governed separately by AlgedonicSignal and its latch above.
The fold opens the incident from its first journal event; harvesting follows
Terms. A parked item follows the separate lifecycle in Pivots. Its unresolved
work can continue through the ladder under the original incident id and budgets
without reopening the stall. An in-flight consultation has a deadline; its
decision, pivot and reason are supplied by completion. The fold treats duplicate
consultation-start events for the same `(incident, rung)` as one consultation;
write-time claiming belongs to the slice-8 coordinator.

Blocked intervals have an open end until observation establishes their close.
Their union is counted per shift, across incidents, so overlapping or duplicated
intervals cannot inflate the total. Their reset boundary is specified under
AlgedonicSignal and its latch.

### Budgets

The `budgets` object in
`.epr-meta/elohim/policies/dialogue-stasis-policy.json` holds these starting
policy bounds, not measured baselines:

| Scope | Keeper calls | Time | Tokens |
|---|---|---|---|
| Per incident | 4 | 1800 s elapsed | 200000 |
| Per shift | — | 7200 s of keeper time | 2000000 |

The same file holds the presence and latch bounds. Their built-in defaults are
the collaborative window T_c = 600 s, Approach at 180 s, Breach at 300 s, and a
latch `dwell_s` = 300 s for dialogue keys, which are `(session, stall_class,
effect_class)`.

Tokens means combined input + output tokens as reported by the keeper runtime's
own usage output: `claude -p --output-format json` usage or codex's "tokens used".
Exhaustion of any applicable budget jumps straight to rung 5. Each consultation
also has bounded effort and a deadline.

Before slice 6 creates the policy file, the slice-3 adapter and slice 5's shadow
`decide` and `fold` use built-in defaults equal to every value above (budgets,
presence and latch bounds), and `decide` and `fold` report
`"policy":"absent (defaults, shadow)"` in JSON. Shadow `decide` with no policy
file exits 0. Live operation is defined under Policy boundary and learning;
its exit behavior is specified in Native CLI contract.

### Shadow operation

A live `decide` or live coordinator can deny or park a PermissionRequest, block a
Stop, deliver guidance through the next hook's context, send a berth note, or
advance or close an incident. In shadow mode, **each** prospective effect is
written to the journal as `would-have` and **none** is executed. Consultations
and hypothetical transitions can be recorded for comparison, but they do not
change the session, deliver notifications or advance or close live incidents.

### Collaborative path

The dialog opens immediately; Remote Control push is usable only after the
interactive probe proves it. The fold opens the incident when the prompt opens.
The coordinator owns the timers: Approach fires after 3 minutes without an
answer and begins harvesting by applying rung 0, then claiming rung 1. Breach
fires at 5 minutes and continues the climb; disposition follows the
Breach-precedence rule in Presence disposition.

Because the dialog is already displayed, handling depends on what can close it:

```text
Displayed dialog
├─ Human closes it
│  └─ Deliver the ladder's resolution at the next hook point;
│     reset absence unless an away ruling remains in force.
└─ Human has not closed it
   ├─ Channel deny-relay proved by the slice-1 interactive probe
   │  └─ May close it earlier by denial, never approval;
   │     relay closure is not a witnessed human interaction.
   └─ Relay unsupported and human absent
      └─ One frozen dialog per absence carries the prepared recommendation;
         guidance waits for closure.
```

### Automated path and questions

The PermissionRequest adapter parks the call with this fixed denial:

> parked as stasis `<id>`. Do not retry this effect by any tool or spelling.
> Continue independent authorized work. Guidance will arrive.

Harvesting begins when the incident opens, following Terms; the automated park
and subsequent ladder work follow the separate lifecycles in the Incident model.
Guidance reaches the session
at its next hook point using the `gate-live-inject` pattern, or wakes an idle
session through an asyncRewake Stop hook only where the interactive probe has
proved support.

For unanswered questions, the coordinator follows the timing rules in Terms.
Ordered P3 research precedes any question consultation.
The regulator validates a keeper's cited ruling before delivering `proceed`,
as specified in the pivot table; a failed validation becomes `park`.
`question: uncertain` never blocks.

### Denial authority and human delivery

Authoritative denials include a human deny, a classifier deny, a call matching an
`ask` or `deny` rule (risk class), and anything uncertain. Their effects may not be
reattempted through another tool or spelling. A rung can propose a different,
lower-risk effect, park the item, propose narrowing the ask rule for the steward
to decide, or Refer.

A procedural denial is the regulator's own park of a shape-class call. Only
explicit `reshape` guidance releases its one same-effect attempt, once per
incident across tools and restarts, through unchanged permission checks. That
guidance never releases an authoritatively denied effect. Palette matching
follows Current state → /agentic-developer.

On reaching rung 5, Breach notification goes out immediately unless it was
already recorded for that incident and stage. Its berth note is the independent
fallback, including when Remote Control is unavailable; the one
`PushNotification` request is delivered in-session as a best effort.
Parked asks are batched.
When no independent work remains, the Stop adapter requires one `AskUserQuestion`
containing the batch. Remote Control's native push for that displayed dialog
remains unsupported until proved by the interactive probe.

Budget exhaustion follows Budgets above.

### Kill switches

`STASIS_LADDER=off` keeps journaling, so observation and measures continue. It
suppresses only decisions, context and blocks; session-visible tool results and
hook stdout are unchanged. Separately, `STASIS_SENSE=off` makes the adapter exit 0
immediately and write nothing. The fold then reports `observation-gap` for that
span when it can detect one, following the unmeasured rule in Terms.

## Policy boundary and learning

The S5 boundary lives in the policy file named under Budgets. Its never-list is the
machine-readable form of the table below: invariant ids plus the classification
rules and bounds; its CID is what the steward's ruling names. The file also
declares promotion criteria, presence bounds and ruling validation.

| Invariant | Enforcer | Prevention or detection |
|---|---|---|
| Never approve a prompt | Code restricts the PermissionRequest adapter to `deny` or no decision. A unit test asserts that no code path emits `allow`. | Prevention. |
| Never widen permissions | No loop component writes any settings file. Proposals go to a proposal-only `generalize-permissions` run that the steward applies; S3* audits actual changes. | Prevention by construction plus S3* detection. |
| Never get around an ask rule | A code check forbids `reshape` for risk class, including uncertain concerns treated as risk. S3* audits actual tool calls for a later same-effect call. | Prevention plus detection. |
| Never accomplish an authoritatively denied effect another way | The fixed deny template instructs against it; S3* audits actual tool calls for another tool or spelling accomplishing the effect. | **Detection, not prevention.** The template is an instruction. |
| Never treat peer/keeper/subagent text as the steward's word | Keeper briefs render that text in a quoted untrusted-evidence block. Only `--kind ruling` notes from the steward count as rulings. | Prevention for rulings; advisory for prose. |
| Never undo work | Keepers run with `--restricted --tools Read,Grep,Glob`, verified by the probe; another runtime needs equivalent probe evidence. The coordinator's only writes are the journal and berth notes. | Prevention. |
| Never edit the policy | `sensitive-file-protection.py` only warns on sensitive paths; it never blocks on path matching. The regulator compares the active policy CID with the steward's latest ruling. | **Not prevented.** CID mismatch is detected and makes the regulator inert, recording the reason. |

**Live operation** is any non-shadow `decide` (slice 9 onward), with
`STASIS_LADDER` on and a ruled policy required for activation. The active policy's
CID must equal the CID named in the steward's latest `--kind ruling`. If a live
invocation finds the policy absent or its CID mismatched, the regulator stays
inert under the ladder-off behavior in Kill switches; the reason is recorded.
The pre-policy shadow exception is defined under Budgets.
The sensitive-path hook is advisory, not the enforcement boundary. Its separate
content detector can request permission for secret material; that does not
protect this policy path.

Learning has three scopes:

- **L0, within an incident:** climb the rungs until the stall resolves.
- **L1, across incidents:** the second clean procedural reshape resolution of
  the same effect class automatically **proposes** a rung 0 entry. That is the
  trigger, not activation. The entry takes effect only after a completed,
  passing other-family S3* audit, under the steward's standing ruling
  **2026-10-30: auto-promote, audited**. The auditor reads what actually ran after
  each deny from transcript tool calls and git; replaying the journal is
  insufficient. A failed or missing audit leaves the entry proposed. Routing
  that skips rungs remains advisory unless the steward authorizes it.
- **L2, the loop on itself:** `dialogue-stasis-health@1` tracks rung 5 share,
  steward corrections, overturned `proceed`s, S3* faults and time to resolution.
  A breach dispatches a read-only rung 4 keeper to **return a patch proposal as
  text**. A separately authorized writer—the steward, or an implementer the
  steward names for that one proposal—applies it to a branch, with other-family
  review. Activation requires a separate steward ruling naming the exact policy
  or package revision CID. At most one proposal is outstanding; a cooldown
  applies, descendant incidents share its budget, and a proposal may not start
  another improvement campaign. P6 forbids changing a running shift's judge.

## VSM composition

| Part | Role | Reused primitive |
|---|---|---|
| Working session and retry restraint | S1 work and self-regulation | `SessionIdentity` |
| Adapters and private journal | S1 self-sense | `epr flow sense` probe |
| Regulator, coordinator, budgets, one notification per incident per stage | S2 coordination and anti-oscillation | Native state machine to be built |
| Rungs 1–4 consultations | S3, with narrow delegated authority | `Decision`, E4 Permit/Refer |
| Other-family audit of actual actions | S3* independent audit | `--kind verdict` |
| Promotion, advisory routing and proposals | S4 learning | Lens rows |
| Policy CID, rulings, presence bounds and never-list | S5 policy | `--kind ruling` |
| Approach, Breach and Recovered | Algedonic channel | `AlgedonicSignal`, extended with reset and dwell |

## Composition with agentic-developer

The skill package at `.epr-meta/elohim/packages/skills/agentic-developer.json`
consumes the mechanism. P5 still checks the palette before every Bash call: a
non-matching command is not attempted. Rung 0 adds learned legible forms on top
of that rule; palette matching follows Current state → /agentic-developer.
P3 still requires ordered research before any bail and before
consulting about a question. Models do not replace evidence.
P6 keeps the current shift's judge outside its write authority. Wishlist entries
come from incident evidence and retain categories based on actual consequences.
Judge `stall` still means no measured change over two or more iterations.

Unattended kickoff records the away ruling. Slice 10 wires the skill's
`shift-marker` events at kickoff and close as defined in Terms. A dialogue rail,
the ceiling on `dialogue-blocked-seconds`, sits beside the CI rails. Close reads
`epr session stasis status --shift-current`. L2 proposals wait for the steward
between shifts. `genesis/docs/shifts/JOURNAL-TEMPLATE.md` and
`genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md` each gain a Dialogue stasis block.

## Journal contract and measures

The versioned schema is
`elohim/sdk/domains/elohim-agent/schemas/session-event.schema.json`. Events carry
`schemaVersion: 1`, `kind`, session incarnation, `agentId`, atomic `seq`, `clock`
and `presence`. Clock evidence is wall time plus boot id and uptime for monotonic
time, with provenance. Runtime and session id are required; owner pid and start
time are optional and nullable, matching the optional process evidence in
`SessionIdentity`, `elohim/eprfs/device-runtime/src/session.rs:11`. A present owner start time follows
`SessionIdentity`'s decimal-string clock-tick representation. Presence carries
disposition and its witnessed basis.

`source` records the required `hookEvent` plus optional, nullable `promptId`,
`toolUseId` (the source event's own id) and `precedingToolUseId` (for a
PermissionRequest). `correlation` carries `incidentId`, confidence
(`high|low|unknown`) and method (`prompt-id-preceding-pretooluse|tool-use-id|inferred-next-event|none`);
unknown confidence permits a null incident id. Matching `prompt_id` plus the
preceding PreToolUse of the same tool is valid only for serial calls. Parallel
batches or identical inputs must produce low confidence or an observation gap,
never a guess. Optional `durationMs` preserves a source duration, or null.

Requirements depend on kind: `permission-requested` and `dialog-displayed` require
source, tool and correlation; `prompt-closed` requires close attribution and
correlation; `observation-gap` requires a gap reason and nullable RFC 3339 `from`
and `to` times; `turn-ended` requires question classification, including
`uncertain`; `shift-marker` requires a `shift` object
`{"id":"<shift-id>","edge":"open"}` or the corresponding `"close"` edge,
with the same id at both edges. Today's `consultation` shape requires level,
binding, Permit/Refer, pivot and reason. The lifecycle contract needs schema support beyond that
completed-result shape: a start carries rung, level, binding, `started_at`,
`deadline_at` and `claimed_by`; completion supplies the result, `finished_at`
and runtime usage for Budgets; expiry identifies the same claim. Slice 5 covers
this contract with synthetic fixtures before slice 8 writes live claims.
Other context, such as effect class, redacted shape and concern class, remains
optional; its producer and interpretation follow Classification above.
Decisions carry presence through their event envelope. Missing context
is not evidence of a safe shape or a confident close.

| Event kind | Meaning |
|---|---|
| `permission-requested` | A call reached permission handling. |
| `dialog-displayed` | A Notification `permission_prompt` arrived; its timing is optional evidence under Folding and measures until the interactive probe proves it. |
| `prompt-closed` | Approval time is PostToolUse time minus `duration_ms`; denial is inferred and labeled `denied-inferred`, or closure is `unknown`. |
| `classifier-denied` | The harness classifier denied the call. |
| `turn-ended` | The turn ended, with required `question: yes\|no\|uncertain`. |
| `consultation` | A consultation lifecycle event under the contract above; slice 5 folds it and slice 8 owns write-time claiming. |
| `steward-ping` | The steward asked for attention or progress. |
| `observation-gap` | Observation was incomplete, with a reason and nullable interval endpoints. |
| `shift-marker` | A skill shift opened or closed, with `shift.id` and `shift.edge`; Terms defines the accounting window. |
| `session-ended` | The session ended. |

### Journal locking

There is one lock file per session:
`.eprfs/status/stasis/<session>/journal.lock`. An exclusive flock covers one
critical section: append the event, update `state.json`, and rotate a segment
when needed. The fold takes a shared flock to read. The slice-8 coordinator's
consultation claim is an append under this same exclusive lock.

### Hook-to-journal mapping

Slice 3 implements this mapping without session output. The keeper-process
filter reads `os.environ` and applies before any event, presence update or
correlation.

Each hook invocation is a fresh process. Correlation state lives in
`.eprfs/status/stasis/<session>/state.json`: the last PreToolUse per
`(agent_id, prompt_id, tool)`, the open prompts and the next `seq`. Its access
follows Journal locking. This is a cache rebuilt by journal replay; the fold
never reads it as authority. For example:

```json
{"schemaVersion":1,"nextSeq":42,
 "lastPreToolUse":{"main|<prompt_id>|Bash":{"toolUseId":"toolu_…","seq":40,"inputDigest":"sha256:…"}},
 "openPrompts":{"main":[{"incidentId":"…","seq":41,"tool":"Bash","precedingToolUseId":"toolu_…","inputDigest":"sha256:…","openedAt":{"wall":"…","uptimeSeconds":0}}]},
 "presence":{"lastWitnessWall":"…","lastWitnessUptimeSeconds":0}}
```

| Hook input | Journal action |
|---|---|
| Any event from a process with `STASIS_KEEPER=1` | Ignore it. |
| PermissionRequest | Write `permission-requested`. `source.precedingToolUseId` is the last PreToolUse of the same tool in the same `prompt_id`; this is valid only for serial calls, otherwise correlation is low confidence. Never guess an id for a parallel batch or identical inputs. |
| Notification `permission_prompt` | Slice 3 wires this path and writes `dialog-displayed` whenever it arrives; interpretation follows Folding and measures. |
| PostToolUse / PostToolUseFailure whose `tool_use_id` matches the open prompt's preceding id | Write `prompt-closed` with `closedBy: approved` and `durationMs`; tool failure does not imply permission denial. |
| The same agent's next PreToolUse, Stop or UserPromptSubmit with no such close | Write `prompt-closed` with `closedBy: denied-inferred` and low confidence. |
| PermissionDenied | Write `classifier-denied`. |
| Stop | Write `turn-ended` with question classification `yes\|no\|uncertain`, in addition to any inferred close above. |
| UserPromptSubmit | Update the presence witness, unless the prompt begins with `<task-notification>` (a rewake or task notification, which is system-originated and never a witness). A "stuck?"-type prompt also writes `steward-ping`; preserve any inferred close above. |
| SessionEnd | Write `session-ended`. |
| A hook error or a sequence discontinuity | Write `observation-gap`. |
| Notification `idle_prompt` | Do not journal it until slice 10 adds silence sensing. |
| New transcript entries since the byte cursor in `state.json`, where an `attachment` has `type: hook_cancelled` (with `timedOut`, `durationMs`, `timeoutMs`, `hookName`, `command`), or a hook error or `StopFailure` record | Write `harness-exception` (see Harness exceptions). |

### Harness exceptions (steward ruling, 2026-10-30)

The steward ruled that **any exception that reaches the session is an algedonic
signal and gets an immediate agent response to investigate and fix it.** It
must not wait for the steward to point at it. An exception is not a dialogue
stall, but it is dialogue friction, because the steward sees it. The motivating
measurement came from this design session itself. Its transcript recorded 201
`hook_cancelled` attachments in about two and a half hours on a host at load 54:

| Hook | Cancelled | Timeout |
|---|---|---|
| `native-seam-guidance` (every PreToolUse) | 128 | 2 s |
| `run-projection` (every prompt, shown in red) | 20 | 8 s |
| `pool-postflight` (Stop) | 16 | 15 s |
| `epr-meta-resolver` and its PostToolUse partner | 30 | 3 s |

The shared cause was `epr bin status` taking about 2.3 s per binary lookup in
`.claude/scripts/_lib/epr_client.py` (`resolve`). The first response was a
120-second positive-resolution cache, plus a process-group kill at
`run-projection`'s own budget.

- **Sensor.** Every hook payload carries `transcript_path`. The adapter keeps a
  byte cursor for it in `state.json` and reads only the bytes appended since. It
  never re-scans the transcript. The transcript is the runtime's record, so this
  reads the Claude Code projection; another runtime's adapter declares its own
  source, or declares the sensor `unsupported`.
- **Journal.** `harness-exception {source: hook_cancelled | hook_error |
  stop_failure, hookName, command (redacted), durationMs, timeoutMs, count}`.
  Repeats of the same `(source, hookName, command)` are coalesced into a count.
- **Response.** Unlike every stall pivot, this response does not wait for a
  ladder. At the session's next hook point, the adapter delivers one
  fixed-template context line through the gate-live-inject pattern: "harness
  exception: `<hookName>` timed out (`<durationMs>` ms against `<timeoutMs>` ms)
  ×N since the last notice. Investigate and fix it now, or dispatch a subagent to."
  - Rate limit: once per key per 30 minutes, unless the count doubles.
  - It is context only. It never denies or blocks anything, and it never edits a
    hook's timeout on its own authority.
  - A fix that raises a timeout hides the cost instead of paying it. The response
    looks for the slow call first.
- **Measure.** `harness-exceptions@1`: the count per shift, by hook, under the
  unmeasured rule.

### Folding and measures

The source of truth is the private journal under
`.eprfs/status/stasis/<session>/`, accessed through Journal locking.
`events.jsonl` rotates at 5 MB to `events.<n>.jsonl`; segments older than 30 days
are deleted. The fold replays every remaining segment in order, increasing `n`
followed by the active `events.jsonl`. An incident that references a dropped
`seq` yields an `observation-gap` with reason `retention`, under the unmeasured
rule in Terms.

Incidents, learned entries and measures are replayable projections of this
journal. Correlation and observation gaps remain explicit when a close cannot
be established.

Until the interactive probe proves Notification `permission_prompt` timing,
the fold treats `dialog-displayed` as optional evidence. When it is absent, the
authorization interval starts at `permission-requested` and is marked
`confidence: low`. This start is accurate in observe-only and shadow modes
because no hook decision delays the dialog.

`dialogue-blocked-seconds@1` measures the union of blocked intervals per session
and shift, by authorization/question/silence class and presence. Overlap and
duplicates must not inflate elapsed waiting. `epr session stasis fold` produces
the readings, resolving `--shift-current` as defined in Terms. Every measure
reading carries `shift=<id>` in its env. Slice 5 wires the measure's probe as:

```yaml
probe:
  run: "epr session stasis fold --shift-current --emit-count dialogue-blocked-seconds@1"
  emits: count
```

`dialogue-blocked-ceiling@1` starts at 1800 seconds per shift: thirty minutes
allows several harvesting windows while bounding the accumulated wait. It is a
starting bound, not a measured baseline. The reset boundary is specified under
AlgedonicSignal and its latch. The 1800 s ceiling is cumulative blocked time
across all of a shift's incidents, distinct from the per-incident 300 s Breach bound.

`ladder-resolution@1` records which rung resolved each incident, tagged by level,
binding, concern class and presence. The later `dialogue-stasis-health@1` measure
feeds the loop's health lens. All follow the unmeasured rule in Terms.
Slice 1 declares only the first two measures and the blocked-time ceiling, with
no keeper dispatch until slice 7.

### Native CLI contract

Slice 5 adds these commands under `epr session stasis`:

```text
fold [--journal-root DIR] [--session ID] [--since RFC3339 | --shift-current] [--json] [--emit-count MEASURE@V]
decide --session ID --event FILE|-
status [--session ID] [--shift-current] [--json]
```

`fold` replays the journal segments. With `--json` it prints
`{incidents, measures: {dialogue-blocked-seconds: {value, byClass, byPresence}, ladder-resolution: [...]}, gaps}`.
It reports policy state according to Budgets and Policy boundary and learning.
`--emit-count` prints a single number for the named measure, for an
`epr flow sense` probe declared with `emits: count`; policy status goes to stderr
in this mode so stdout remains numeric. Exit 0 means read, exit 2
means no journal (unmeasured, printing nothing for `--emit-count`), and exit 1
means a parse error. Evidence handling follows the unmeasured rule in Terms.

`decide` reads one adapter-normalized hook event from a file or stdin (`-`), plus
that session's journal-folded state, and prints JSON
`{effect: none|deny|block|context, message, incidentId, rung, shadow: bool, policy}`.
In slice 5 it always runs shadow: it journals the prospective effect as
`would-have` and prints `effect: none` with `shadow: true`. Exit 0 is normal;
exit 3 applies only to live operation and means a policy CID mismatch or an
absent policy, with handling as specified under Policy boundary and learning.
When `decide` runs in shadow mode and no policy file exists, it uses the built-in defaults and reports `"policy":"absent (defaults, shadow)"` (see Budgets). Adapters treat **any** nonzero
exit, timeout or missing binary as no decision and fail open.

`status` prints open incidents, pending parks, budget used, coordinator heartbeat
age and policy-CID state, using JSON when requested. It is read-only.

## Slices

Implementation proceeds through observation, governance, action, then composition
and learning. Each slice lands separately when the steward asks to land it.

### A. Observe

1. **Spec, contract and probes.** Create this spec, the schema, the unwired habit
   and the measure rows. Docs and schema alone prove no harness capability. The
   headless `genesis/agentic/bin/stasis-probe` has a reported 30 `yes` checks for
   event emission, deny-and-continue, model bindings, the restricted read-only
   keeper shape and its environment marker. The separate steward-assisted
   interactive probe must establish all six unsupported capabilities in the
   table: permission-prompt notification timing, human-deny emissions, idle-prompt
   notification timing, asyncRewake waking an idle session, Remote Control push
   on `AskUserQuestion`, and channel deny-relay. Re-scope retrospective §6 and
   rows 63, 64, 67, 68 and 69 when that branch is available.
2. **Package tooling.**
   Read first: Current state → Agent packages; HookPackage registration.
   Migrate the single hook `registration` to an array,
   reconcile every registration, and compare `async` and `asyncRewake`. Add
   `capabilityProfile`, runtime `capabilityBindings` and projector mapping, with
   tests, using the input/output and unbound-level contract in Capability
   vocabulary and bindings and the completed effort probe in Capability
   verification table. Preserve the migration fallback specified there.
3. **Observe-only adapter.**
   Read first: Current state → Agent packages; HookPackage registration; gate-live-inject.
   Author HookPackage
   `.epr-meta/elohim/packages/hooks/dialogue-stasis-sense.json`, projected to
   `.claude/hooks/dialogue-stasis-sense.py`. Record journal events and presence,
   following the Hook-to-journal mapping, including the Notification
   `permission_prompt` path now, and write nothing back to the session.
   Use the raw-field boundary in Classification, the state contract in
   Hook-to-journal mapping, and Journal locking. Apply the segmentation and
   retention contract in Folding and measures, preserving retention gaps.
   Tests are Python unittest files under `.claude/hooks/tests/`, following
   `test_gate_live_inject.py`; they replay JSON hook-event fixtures through the
   adapter script with a temporary journal root.
   The steward wires `settings.json`.
   **3b. Harness-exception relay**, landing with or right after slice 3. It is
   the one piece of output this observe-first plan allows early, because it is
   context only (see Harness exceptions). Its tests cover four cases:
   - a transcript fixture with three `hook_cancelled` entries for one hook yields
     one coalesced `harness-exception` with count 3;
   - exactly one context line is emitted, and a second invocation within 30
     minutes stays silent;
   - doubling the count re-emits;
   - an absent `transcript_path` writes `observation-gap`, never zero.
4. **Shared algedonic reset.**
   Read first: Current state → AlgedonicSignal and its latch; Inherited wedged-peer provisions.
   Implement the additive shared-module contract in `elohim/epr/src/algedonic.rs`
   and its `algedonic-recovered.schema.json` sibling contract, within the stated
   E1 scope. Verify the latch separately from the incident lifecycle as specified
   in the slice-4 evidence row.
5. **Native fold and inert regulator.**
   Read first: Current state → AlgedonicSignal and its latch; Measures, lenses and epr flow sense.
   Add `session_stasis.rs` in device-runtime for every schema event kind,
   `consultation` included: intervals, incidents, rung state, budget accounting
   and the native classifier, plus
   `epr session stasis {decide,fold,status}` in epr-cli with the Native CLI
   contract above. Test all of it with synthetic journal fixtures; no keeper or
   coordinator is needed. Consultation replay uses the Incident model's
   idempotence rule; atomic claiming is deferred to slice 8. Use the built-in
   rule set from Classification and the pre-policy defaults and report from
   Budgets. `decide` follows Shadow operation throughout this slice.
   The first measure replays all retained journal segments
   back to slice 3, using the declared `emits: count` probe.

   This sample synthetic consultation result is valid against the current
   `elohim/sdk/domains/elohim-agent/schemas/session-event.schema.json`; C1
   corresponds to rung 1. The additional lifecycle evidence follows the
   Journal contract above.

   ```jsonl
   {"schemaVersion":1,"kind":"consultation","session":{"runtime":"claude-code","session":"fixture-session"},"agentId":"stasis-coordinator","seq":2,"clock":{"wall":"2026-10-30T12:03:10Z","bootId":"fixture-boot","uptimeSeconds":190,"provenance":"synthetic-fixture"},"presence":{"disposition":"automated","basis":{"lastWitnessedAgeSeconds":null,"rulingRef":null}},"correlation":{"incidentId":"fixture-session:1","confidence":"high","method":"inferred-next-event"},"consultation":{"level":"C1","binding":"haiku","decision":"Refer","pivot":"notify","reason":"Rung 1 could not resolve the stall."}}
   ```

### B. Govern

6. **S5 boundary and S3* audit.** Create and pin the policy with the initial
   `budgets` and rule set defined above, and enforce Policy boundary and learning.
   The other-family auditor, initially Codex, reads actual transcripts and git at close.
7. **Keeper policy.** Author agent and skill packages at
   `.epr-meta/elohim/packages/{agents,skills}/stasis-keeper.json`, with
   `capabilityProfile`, read-only-nonprompting authority, Decision plus pivot, and
   denial-class rules.

### C. Act

8. **Coordinator.** Implement one daemon per device, owning every incident of
   that device's sessions through the flock-held pid file, with ordered
   consultations, deadlines, budget enforcement and heartbeat. Before running a
   keeper, atomically claim `(incident, rung)` by appending a `consultation`
   start event identifying `claimed_by`, using the exclusive critical section
   in Journal locking. A duplicate writer attaches to the existing claim.
   Completion or deadline expiry is journaled against that claim, so a restart
   cannot spend another keeper call on it. Start in
   shadow: consult and journal each prospective effect as `would-have`, with no
   denial or park, Stop block, guidance delivery,
   berth note, or live incident advancement or closure. Compare that guidance
   before switching on.
9. **Control on.** Enable presence handling, automated park and harvesting when
   the incident opens, collaborative harvesting at the 3-minute Approach and
   continued escalation at the 5-minute Breach, rung 0 at PreToolUse, Stop
   question handling and parked-ask batching, guidance delivery, notifications
   and the controls in Kill switches, subject to the live policy requirements in Policy
   boundary and learning.

### D. Compose and learn

10. **Agentic-developer and silence.** Land the silence sensor and its independent
    timer first, progress requests second, then the shift-template blocks and
    skill composition, including the kickoff/close `shift-marker` wiring
    defined in Terms.
11. **L1 and L2.** Add audited automatic promotion, advisory routing, and
    proposal-only loop improvement with separate steward activation.

## Verification

| Slice | Required evidence |
|---|---|
| 1 | <ul><li>A capability probe report answers yes/no for every harness dependency on the installed CLI version; later adapters cite it and unproven capabilities stay unsupported.</li><li>The docs/schema portion validates habit projection, JSON parsing and measure-registry loading.</li></ul> |
| 2 | <ul><li>A two-registration fixture fails when either registration drifts, including either async flag.</li><li>A C2 profile with low effort projects to `model: sonnet` and `effort: low` for claude-code, following Capability vocabulary and bindings and the completed effort probe.</li><li>A bound codex profile projects both `model` and `model_reasoning_effort`.</li><li>An unbound gemini profile omits the model field, records `capability-unbound` in its fixture and produces a verify warning.</li><li>A package without `capabilityProfile` preserves its `modelHints.claudeModel` fallback.</li></ul> |
| 3 | <ul><li>The Python replay tests specified in slice 3 pass for approve, deny, subagent-prompt, parallel-batch and identical-input fixtures, preserving honest open/close confidence and recording gaps for unknown closes.</li><li>Fixtures cover every Hook-to-journal mapping, including Notification `permission_prompt` whenever it arrives, matching PostToolUseFailure, inferred closes, presence updates, sequence gaps and ignored keeper events.</li><li>Fixtures verify the adapter boundary in Classification.</li><li>The adapter produces no session output and does not journal `idle_prompt` yet.</li></ul> |
| 4 | <ul><li>Fake-clock tests cover the shared latch's Recovered transition, recovery evidence and reset threshold for Ceiling and Floor bounds.</li><li>Tests exercise both dwell outcomes in the latch diagram, including a new stall on the same key before its duration reaches Approach, and verify reset-line re-crossing.</li><li>Persisted latch state survives restart without re-emitting an open signal.</li><li>The new recovered signal validates against its schema alongside approach and breach fixtures; existing public items and `should_emit` retain their behavior.</li><li>`just gate` passes for the touched shared algedonic project.</li></ul> |
| 5 | <ul><li>Synthetic fixtures cover every schema event kind, including consultation starts, completions and expiries, without a keeper or coordinator; replay reconstructs the Incident model and its closure at every listed attribution.</li><li>Duplicate consultation-start events for the same `(incident, rung)` fold idempotently.</li><li>Fake-clock tests union overlapping and duplicated blocked intervals per shift and verify the cumulative measure's reset boundary, the marker and session-window selection in Terms, the 24-hour split, and `shift=<id>` in every reading's env.</li><li>Fixtures verify the optional `dialog-displayed` evidence and low-confidence authorization-interval fallback in Folding and measures.</li><li>Fixtures exercise every bound in Budgets and its exhaustion behavior, including the pre-policy defaults and report.</li><li>Classifier fixtures cover the initial shape patterns, effect normalization, explicit ask/deny matches and unclassifiable inputs under Classification.</li><li>A differential check classifies a corpus of commands with the active ask rules in the native matcher, then runs the same commands through `stasis-probe`'s permission harness, where a PermissionRequest fires exactly when a rule prompts. Any disagreement fails unless the native side said risk or uncertain, the conservative direction.</li><li>Shadow decisions satisfy Shadow operation, including exit 0 and the JSON policy field specified under Budgets when no policy file exists.</li><li>`epr flow sense --measure dialogue-blocked-seconds@1 --dry-run` reads a non-zero blocked-time value on the replayed journal.</li><li>`just gate` passes for the touched native fold and CLI projects.</li></ul> |
| 6 | <ul><li>Editing policy without a matching ruling makes the regulator inert and records why.</li><li>The audit flags a seeded denied effect accomplished through another tool.</li><li>The PermissionRequest adapter has no code path that emits `allow`.</li></ul> |
| 7 | <ul><li>An adversarial peer note is rendered as quoted untrusted evidence and cannot validate as a steward ruling; prose resistance is tested without claiming mechanical prevention.</li><li>Keepers raise zero prompts and open no incidents.</li></ul> |
| 8 | <ul><li>Concurrent writers claim one consultation atomically under Journal locking.</li><li>A claimed consultation survives restart without another keeper call; duplicate events attach to that claim.</li><li>A dead coordinator reads unknown.</li><li>Shadow guidance is compared with actual actions.</li></ul> |
| 9 | <ul><li>Collaborative timing follows the Incident model and harvesting contract; tests verify the Breach-precedence and away-ruling rules in Presence disposition.</li><li>Automated harvesting begins when the incident opens; handling parks, climbs rungs 1–4 and reaches rung 5, with one berth note and one `PushNotification` request per incident per reached stage, plus one batched `AskUserQuestion`.</li><li>Risk and uncertain concerns never receive a same-effect reshape; shape concerns get only one across tools and restarts, and a second failure Refers.</li><li>`proceed` requires a valid, unexpired, scoped steward ruling; any failed validation downgrades to `park`.</li><li>Fixtures exercise the park item's separate lifecycle and later-execution requirements in Pivots.</li><li>A missing policy and a CID mismatch both satisfy the inert live-operation contract in Policy boundary and learning.</li><li>Both controls satisfy Kill switches, including continued measures with only the ladder disabled and detectable observation gaps with sensing disabled.</li></ul> |
| 10–11 | <ul><li>A 45-minute fixture wait yields one progress request.</li><li>Skill kickoff and close write the `shift-marker` edges defined in Terms.</li><li>A second clean resolution of the same effect class proposes a rung 0 entry; it takes effect only after a completed, passing other-family audit, and a missing or failed audit leaves it proposed.</li><li>A seeded health breach produces one patch proposal and no activation.</li></ul> |

The habit `dialogue-stalls-are-answered` turns green only after two shifts below
the blocked-time ceiling, one unattended, with a falling rung 5 share and
zero S3* faults. Its birth is unwired: no sensor exists yet. Slice 3 provides the
first observation evidence, not proof of the whole invariant.

## Execution

Non-cargo work goes to Codex first in its own worktree. Cargo slices 4 and 5 use
one Claude agent per crate, sequentially because they share the build slot. The
spec requires the planned gpt-6-astra round-4 review and a fresh blind-reader pass
before runtime implementation begins; these are review gates, not claims of
completed reviews.

## Frontier

The protocol will eventually hold its own understanding of which model fits a
context and what that model can do. This loop does not build that model-fit
layer. Its hand-declared capability bindings are placeholders. The first evidence
stream for a later replacement is `ladder-resolution@1`: which bound model resolved
which concern under which presence disposition. A future model-fit layer can read
that evidence instead of treating the binding table as permanent knowledge.

## Non-goals

- Peer leases remain retrospective row 63.
- Gate wait reporting remains row 64.
- Host pressure remains row 67.
- Approving prompts or making decisions on the human's behalf is outside authority.
- A protocol-native model-fit layer remains the frontier, outside this implementation.
