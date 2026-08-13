# STEP DEFINITIONS ARE NOT WIRED YET — this feature is honestly @wip, not green.
# No step definition in genesis/a2o/steps/ drives either surface below today. The
# wiring is a named follow-up: a `steps/devflow/run-plane.steps.ts` in the register
# that steps/seeder.steps.ts already established for a process-driven scenario
# (spawn the process against a scratch repository root, assert on its exit status
# and on what it wrote). Until that lands, every scenario below reads as pending —
# which is the honest state, and is why the feature carries @wip at the feature
# line rather than a claimed green.
#
# The two processes under test, so no reader has to guess what a step would drive:
#   sections 1 and 3 — the `epr flow` CLI (`epr flow note`, `epr flow stocks`)
#                      built from elohim/eprfs/epr-cli, run against a scratch root;
#                      the flow store it appends to is .eprfs/status/flows.jsonl.
#   section 2        — the run-projection emitter, .claude/hooks/run-projection.py,
#                      invoked as the agent's next turn is submitted; the block IS
#                      its standard output, and a step asserts on that text.
#
# Specs: genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md
#        genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md
# Habit: dev-system-equilibrium (genesis/manifests/habits.yaml) — the equilibrium
#        scenarios below are the story form of that habit's runnable check.

@e2e @devflow @wip @requires:epr-cli
Feature: The run plane — what a long run remembers, and whether the work is draining
  As an agentic developer working one objective across many sessions, where the
  conversation between sessions is summarised away and cannot be relied on
  I want the corrections I am given, the promises still owed, and the honest
  answer to "is this system finishing work as fast as it takes it on?" to live in
  the repository's own durable record and be handed back to me every turn
  So that a correction survives the loss of the conversation that carried it, an
  open promise is never quietly marked done, and nobody claims the work is under
  control on the strength of a measurement that measured nothing.

  # Vocabulary, so this story stands without its authors:
  #
  #   flow store    — the repository's append-only record of the development
  #                   work: promises made (commitments), things done (events).
  #                   It is derived from the repository, never hand-edited, and
  #                   is rebuilt from scratch by projecting the repository again.
  #   projection    — re-deriving that record from the repository's current
  #                   state. It is the operation that must not lose anything a
  #                   human or agent deliberately wrote.
  #   correction    — an authored note against an open commitment: what was
  #                   tried, why it was wrong, what to do instead. It is the
  #                   mid-run "no, not like that" made durable.
  #   the emitter   — the one process that produces the block. It stores nothing
  #                   and is the block's only producer, so "the block for a turn"
  #                   means exactly the text the emitter wrote for that turn.
  #   a turn begins — the emitter is run because the agent's next turn is being
  #                   submitted. That submission is the event; the block is what
  #                   the emitter writes in response to it.
  #   the block     — a short state summary the emitter re-derives and writes at
  #                   the start of every turn, to be read by the agent before it
  #                   acts. Nothing in it is authored; every line is re-read from
  #                   a register.
  #   WIP fence     — the standing rule that at most two habits may be active at
  #                   once. The block shows it so it cannot be quietly exceeded.
  #   stock         — a quantity that fills and drains. Open commitments are a
  #                   stock: minting one fills it, discharging one drains it.
  #   equilibrium   — drain at least as fast as fill, compared as two rates over
  #                   one declared window. Never a level against a ceiling.

  Background:
    Given the repository's valueflow has been projected into the flow store
    And an open commitment exists for work still owed

  # ══════════════════════════════════════════════════════════════════════════
  # 1. The write leg — a correction becomes durable, and stays honest
  # ══════════════════════════════════════════════════════════════════════════

  @concern:run-plane-note
  Scenario: A correction written in one session is still there for the next one
    # The graduation trigger for the write leg: the correction crosses a session
    # boundary through the repository's record, with no conversation carried
    # over. This is the difference between a correction that survives and one
    # that was absorbed by the summariser that lost it.
    Given a correction is written against the open commitment naming what went wrong and what to do instead
    And the session that wrote it has ended
    When a later session projects the repository's valueflow again
    Then that correction is still readable in the flow store, word for word
    And the correction still names the commitment it was written against
    And no conversation from the earlier session was needed to recover it

  @concern:run-plane-note
  Scenario: A correction annotates an open commitment and never discharges it
    # The load-bearing rule of the write leg. A record that says something about
    # a promise must never be mistaken for the promise being kept — otherwise
    # writing a note would silently retire live work, and the frontier would
    # shrink because someone described the work rather than finished it.
    Given the open commitment appears as unfulfilled in the forward frontier
    When a correction is written against that commitment
    Then the commitment still appears as unfulfilled in the forward frontier
    And the count of unfulfilled commitments is unchanged by the correction
    And the correction is readable as a note about the work, not as its delivery

  @concern:run-plane-note
  Scenario: Writing the same correction twice leaves one record, not two
    # Replay safety. A run that repeats itself — a retry, a re-read, a second
    # agent reaching the same conclusion — must not inflate the record, because
    # every later count of what happened is read off that record.
    Given a correction has already been written against the open commitment
    When a correction identical to it in every detail is written again
    Then the flow store holds exactly one such correction
    And the second write reports success rather than an error

  @concern:run-plane-note
  Scenario: A correction against work that cannot be found is refused, not filed loose
    # Refusal beats a plausible guess. An orphaned note is worse than no note:
    # it reads as covered ground while pointing at nothing, and no later reader
    # can tell the difference.
    Given a correction names a commitment that does not exist in the flow store
    When that correction is written
    Then the write is refused and names the target it could not resolve
    And nothing at all is appended to the flow store
    And the flow store is byte-for-byte what it was before the attempt

  # ══════════════════════════════════════════════════════════════════════════
  # 2. The read leg — the per-turn block, and how it degrades
  # ══════════════════════════════════════════════════════════════════════════

  @concern:run-plane-projection
  Scenario: Every turn opens with the fence, the frontier, and the newest correction
    # The block is re-derived, never remembered. That is what stops the state an
    # agent is steering by from sinking deeper into the conversation as the
    # session grows, and what makes a compaction survivable rather than lossy.
    Given the habit register holds a red habit and at most two active habits
    And a correction has been written against an open commitment
    When the emitter runs because a turn is beginning
    Then the block it writes is at most twenty lines long
    And that block names the top red habit and the first check that would prove it
    And that block names the habits currently active, and no more than two
    And that block names the equilibrium reading for the open-commitment stock
    And that block names the most recent correction from the flow store
    And that block closes by teaching how to write a correction that outlives this window

  @concern:run-plane-projection
  Scenario: An input the block cannot read costs one line, never the turn
    # A state summary that can fail the turn it is summarising is a worse deal
    # than no summary. Partial sight, plainly partial, is the contract.
    Given one of the registers the emitter derives the block from cannot be read
    When the emitter runs because a turn is beginning
    Then the emitter still writes a block
    And that block omits only the line it could not derive
    And no error text or stack trace appears anywhere in what the emitter wrote
    And the emitter finishes within its declared per-turn budget rather than waiting on the failed read
    And the emitter reports success, so the turn is never failed by it

  @concern:run-plane-projection
  Scenario: A stale equilibrium reading says so instead of reporting an old rate
    # "We have not measured since the record changed" and "the work is draining"
    # are different claims, and the block must never let the first pass as the
    # second. Staleness is reported, never smoothed over.
    Given the flow store has changed since the equilibrium reading was last derived
    When the emitter runs because a turn is beginning
    Then the equilibrium line of the block it writes reads as awaiting a fresh measurement
    And that line does not report a rate derived before that change
    And the rest of that block is unchanged by the stale reading

  # ══════════════════════════════════════════════════════════════════════════
  # 3. The equilibrium verdict — filling, draining, or unmeasured
  # ══════════════════════════════════════════════════════════════════════════

  @concern:dev-system-equilibrium
  Scenario: A stock taking on work faster than it finishes it fails the check
    # The red this habit exists to write. A development system that mints more
    # promises per week than it discharges is accumulating debt whether or not
    # anybody feels it that week — and the check says so out loud rather than
    # leaving it to be noticed later.
    Given a window in which more commitments were minted than were discharged
    When the equilibrium check is run over the commitment stock for that window
    Then the check exits with a non-zero status
    And the reading names the commitment stock as filling
    And the reading states inflow and outflow as rates over the declared period, not as a level against a ceiling
    And the reading states which discharge paths it counted as drain

  @concern:dev-system-equilibrium
  Scenario: A stock finishing work at least as fast as it takes it on passes
    Given a window in which at least as many commitments were discharged as minted
    When the equilibrium check is run over the commitment stock for that window
    Then the check exits with a zero status
    And the reading names the commitment stock as draining

  @concern:dev-system-equilibrium
  Scenario: A window with nothing to measure refuses rather than reporting equilibrium
    # The over-claim this whole vocabulary exists to prevent. "We cannot see the
    # drain" and "the drain is adequate" must never arrive as the same verdict —
    # a green earned by measuring nothing is the most expensive kind of green.
    Given a window in which no commitment was either minted or discharged
    When the equilibrium check is run over the commitment stock for that window
    Then the check exits with a non-zero status
    And the reading names the drain as unknown rather than adequate
    And the reading never reports equilibrium for a window it could not measure

  @concern:dev-system-equilibrium @regression
  Scenario: Finishing a promise drains the stock and never fills it
    # Guards a sign inversion that arrives through the data rather than through
    # anyone's arithmetic: the record of finishing a piece of work is shaped, at
    # the storage layer, like the record of producing something new. Read
    # carelessly, every completion would count as fresh intake and a draining
    # system would report itself as filling.
    Given a window in which one commitment was minted and that same commitment was discharged
    When the equilibrium check is run over the commitment stock for that window
    Then the discharge counts once toward the outflow
    And the discharge counts nothing toward the inflow
    And the check exits with a zero status
