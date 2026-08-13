# STEP DEFINITIONS ARE NOT WIRED YET — this feature is honestly @wip, not green.
#
# BLIND-READER LOOP: five fresh-context cycles ran 2026-08-13 (a2o-story profile).
# Two findings are OPERATOR-DEFERRED (2026-08-13) to the step-def wiring follow-up,
# where the emitter contract is exercised for real; one final reader runs after
# that wiring lands: (1) whether a block MARKS a line it could not derive rather
# than silently omitting it — a real emitter design choice, the reader's
# false-green argument is recorded and unrebutted; (2) a cold-start scenario for
# a session whose start-of-session check produced no reading.
# No step definition in genesis/a2o/steps/ drives either surface below today. The
# wiring is a named follow-up: a `steps/devflow/run-plane.steps.ts` in the register
# that steps/seeder.steps.ts already established for a process-driven scenario
# (spawn the process against a scratch repository root, assert on its exit status
# and on what it wrote). Until that lands, every scenario below reads as pending —
# which is the honest state, and is why the feature carries @wip at the feature
# line rather than a claimed green.
#
# The tags are suite-routing labels, not behaviour: @e2e and @devflow route this
# file into the end-to-end devflow suite, and each @concern:<name> joins its
# scenario to the habit check that claims it in genesis/manifests/habits.yaml.
#
# The two processes under test, so no reader has to guess what a step would drive:
#   sections 1 and 3 — the `epr flow` CLI (`epr flow note`, `epr flow stocks`)
#                      built from elohim/eprfs/epr-cli, run against a scratch root;
#                      the flow store it appends to is .eprfs/status/flows.jsonl.
#                      This is what @requires:epr-cli declares.
#   section 2        — the run-projection emitter, .claude/hooks/run-projection.py,
#                      invoked as the agent's next turn is submitted; the block IS
#                      its standard output, and a step asserts on that text. Its
#                      prerequisite is a different one, and having the CLI does not
#                      imply it: that hook must be present on disk AND registered
#                      in .claude/settings.json so a submitted turn actually runs it.
#                      That prerequisite is stated here in prose rather than as a
#                      tag on purpose: the @requires: namespace covers declared
#                      capabilities only — @requires:epr-cli is one — and no
#                      fixture capability exists yet for "the hook is installed and
#                      registered". Prose is the honest state until one does.
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
  open promise is never quietly marked done, nobody claims the work is under
  control on the strength of a measurement that measured nothing, and every turn
  points me at the work that matters most right now.

  # Vocabulary, so this story stands without its authors:
  #
  #   valueflow      — the development work read as economic activity: promises
  #                    to do something (commitments) and things actually done
  #                    (events), each naming who acted, on what, and how much.
  #   commitment     — one promise still owed. Commitments are not typed into the
  #                    store: a plan in the repository decomposes into work items,
  #                    and a work item marked claimed becomes an open commitment
  #                    the next time the repository is projected. That is the only
  #                    way one comes into being, including in the setups below.
  #   flow store     — the repository's append-only record of that valueflow.
  #                    Never hand-edited: it is appended to only by the flow
  #                    tooling — some records projected from the repository,
  #                    others authored through the note command — and it only
  #                    ever grows: nothing already written into it is deleted
  #                    or rewritten. Honest limitation, stated here so
  #                    no scenario below over-promises: the store file lives in
  #                    this workspace and is not carried by git, so deleting the
  #                    file loses the corrections authored into it. Durability
  #                    beyond one workspace is the named protocol graduation
  #                    path, not something this feature claims today.
  #   projection     — re-reading the repository's current state and appending
  #                    to the flow store whatever is new there. It adds; it
  #                    never removes. That is why it cannot lose what a human or
  #                    agent deliberately wrote. It is also the only route by
  #                    which a commitment reaches the store — nothing, including
  #                    the setup of a scenario below, writes one there by hand.
  #   correction     — a note against an open commitment: what was tried, why it
  #                    was wrong, what to do instead. It is the mid-run "no, not
  #                    like that" made durable. A correction is given by the
  #                    human or authored by the agent itself; either way it ends
  #                    up in the flow store as a record like any other. Records
  #                    made at one commit share that commit's date, so they are
  #                    ordered by their position in the append-only store:
  #                    "the newest correction" means the last one appended, even
  #                    when two corrections arrive in the same turn.
  #   the frontier   — the still-owed commitments, read off the flow store by
  #                    the walk and status readers. "The count of unfulfilled
  #                    commitments" is that same set, counted.
  #   habit          — a promise in the register at genesis/manifests/habits.yaml
  #                    about what this system reliably does.
  #   check          — the runnable proof a habit names for itself. A habit
  #                    without one is a claim nobody can settle.
  #   red habit      — a habit whose check exists and currently fails. That is
  #                    what makes it schedulable work rather than an intention.
  #   top red habit  — the first red habit in the register's own order. The
  #                    register is curated by hand, so its order IS the
  #                    priority; the block imports no other ranking.
  #   the emitter    — the one process that produces the block. It authors
  #                    nothing into any register — the habit register, the flow
  #                    store and the saga are all read-only to it — but it does
  #                    keep a private cache so a turn never waits. That cache
  #                    has two tiers: the cheap lines (fence, frontier, saga,
  #                    newest correction) are re-derived whenever the registers
  #                    they come from change; the equilibrium reading is not —
  #                    it can only come from actually RUNNING the check, so the
  #                    emitter runs the check once as a session starts,
  #                    declaring the window itself (the trailing week ending
  #                    that day), and between session starts it can only show
  #                    the reading it has or call it stale. Losing the cache
  #                    costs a re-derivation and nothing else. The emitter is
  #                    the block's only producer, so "the block for a turn"
  #                    means exactly the text the emitter wrote for that turn.
  #   the saga       — the ordered register of the system's recovery story:
  #                    numbered resiliency chapters, each green, red, or
  #                    regressed, with a frontier chapter where the story
  #                    currently stands. It feeds exactly one line of the
  #                    block, the same way the habit register feeds the fence.
  #   session        — one process lifetime of the agent and the conversation
  #                    it carries. A session has ended when that process has
  #                    exited and none of its conversation memory remains; a
  #                    later session is a new process whose only state shared
  #                    with the old one is the repository and its flow store.
  #   a turn begins  — the emitter is run because the agent's next turn is being
  #                    submitted. That submission is the event; the block is
  #                    what the emitter writes in response to it.
  #   the block      — a short state summary the emitter re-derives and writes
  #                    at the start of every turn. Its consumer is the agent,
  #                    which reads it before it acts. Nothing in it is authored:
  #                    every line is re-read from a register, or — for the
  #                    equilibrium line — from the cached reading derived from
  #                    one, which is why that line can go stale and say so.
  #   WIP fence      — the standing rule that at most two habits may be active
  #                    at once. The block shows the active habits against that
  #                    limit, so the limit cannot be quietly exceeded.
  #   stock          — a quantity that fills and drains. Open commitments are a
  #                    stock: minting one fills it, discharging one drains it.
  #   discharge path — which kinds of recorded event the check counts as
  #                    draining a promise. Today there is exactly one: a
  #                    fulfillment that names the commitment. The reading has to
  #                    say which paths it counted, so a drain that was never
  #                    looked for cannot pass as a drain that was not there.
  #   window         — the one declared span of time both rates are measured
  #                    over. Every rate in this story is a rate over the window.
  #                    It is declared, never inferred: whoever runs the check
  #                    states it as two dates and a period, so "a declared week"
  #                    below means exactly a start date, an end date, and a
  #                    period of one week.
  #   equilibrium    — drain at least as fast as fill, compared as two rates
  #                    over one window. Never a level against a ceiling.
  #   the equilibrium check
  #                  — the runnable proof named by the dev-system-equilibrium
  #                    habit, and the surface section 3 exercises: the
  #                    `epr flow stocks` command from the top of this file, run
  #                    with its window declared on the invocation as two dates
  #                    and a period (`--window START..END --per week`).
  #   the reading    — the output of the last equilibrium check the emitter
  #                    ran, held in its private cache — derived and disposable,
  #                    never a register — and tagged with the state of the flow
  #                    store it was derived from. It is what the block
  #                    displays; the tag is what makes it possible to call
  #                    stale when the store has changed since.
  #   unmeasured     — the check's verdict when the window contains no flow
  #                    events for the stock at all: nothing minted inside it
  #                    and nothing discharged inside it. The boundary is
  #                    strict — zero inflow with observed discharges is
  #                    measurable (and draining); unmeasured means the window
  #                    showed the check nothing whatsoever to count.
  #
  # One deliberate collision, so it does not read as an accident: a filling
  # stock and a window with nothing to measure BOTH make the check exit
  # non-zero. That is intended — the exit status only ever says "not fine". The
  # text of the reading is what distinguishes "filling" from "could not measure".

  Background:
    Given a repository whose plan carries one work item marked claimed
    And that repository has been projected, so the flow store holds one open commitment for that work item

  # ══════════════════════════════════════════════════════════════════════════
  # 1. The write leg — a correction becomes durable, and stays honest
  # ══════════════════════════════════════════════════════════════════════════

  @concern:run-plane-note
  Scenario: A correction written in one session is still there for the next one
    # The graduation trigger for the write leg: the correction crosses a session
    # boundary through the repository's record, with no conversation carried
    # over. This is the difference between a correction that survives and one
    # that was absorbed by the summariser that lost it. The ended session in the
    # second Given is the whole point — it is stated so that the flow store is
    # the only route left by which the later session could recover the text.
    Given a correction is written against the open commitment naming what went wrong and what to do instead
    And the session that wrote it has ended
    When a later session reads the flow store with no conversation carried over
    Then that correction is still readable in the flow store, word for word
    And the correction still names the commitment it was written against

  @concern:run-plane-note
  Scenario: A correction annotates an open commitment and never discharges it
    # The load-bearing rule of the write leg. A record that says something about
    # a promise must never be mistaken for the promise being kept — otherwise
    # writing a note would silently retire live work, and the frontier would
    # shrink because someone described the work rather than finished it.
    Given the open commitment is among the still-owed commitments in the frontier
    When the agent writes a correction against that commitment
    Then that commitment is still among the still-owed commitments in the frontier
    And the count of unfulfilled commitments, which counts that same set, is unchanged
    And the record written is a note, not a fulfillment

  @concern:run-plane-note
  Scenario: Writing the same correction twice at one commit leaves one record
    # Replay safety. A run that repeats itself — a retry, a re-read, a second
    # agent reaching the same conclusion — must not inflate the record, because
    # every later count of what happened is read off that record. The identity
    # of a record is its own content, not who typed it or when the key was
    # pressed: same text, same target, same commit is the same record. The
    # commit is what fixes that, because a record's date and author are read off
    # the repository's current commit rather than off the clock — so every
    # replay at one commit produces the identical record. Once the repository
    # has moved on to a new commit, those same words carry a new date and are a
    # new act: a correction given again later is a second correction, and
    # should be.
    Given a correction has already been written against the open commitment
    And the repository is at the same commit it was at when that correction was written
    When the same correction text is written against that same commitment again
    Then the flow store holds exactly one such correction
    And the second write reports success rather than an error

  @concern:run-plane-note
  Scenario: A correction against work that cannot be found is refused, not filed loose
    # Refusal beats a plausible guess. An orphaned note is worse than no note:
    # it reads as covered ground while pointing at nothing, and no later reader
    # can tell the difference.
    Given a correction names a commitment that does not exist in the flow store
    When that correction is written
    Then the write exits with a non-zero status and names the target it could not resolve
    And the flow store is byte-for-byte what it was before the attempt

  # ══════════════════════════════════════════════════════════════════════════
  # 2. The read leg — the per-turn block, and how it degrades
  #    Every scenario in this section needs the run-projection hook present and
  #    registered (the header note above) — none of them can run on the CLI
  #    alone, and no @requires: tag exists yet to say so. Stated here so the
  #    gap is visible where the scenarios are, not only at the top of the file.
  #    When the hook is absent these scenarios HOLD as pending — they never
  #    fail a run on a machine that has no emitter to test. And like the CLI's
  #    scratch root, the registers and settings these Givens arrange are
  #    scratch copies pointed at by the test — no Given below mutates the real
  #    habit register.
  # ══════════════════════════════════════════════════════════════════════════

  @concern:run-plane-projection
  Scenario: Every turn opens with the fence, the frontier, and the newest correction
    # The block is re-derived, never remembered. That is what stops the state an
    # agent is steering by from sinking deeper into the conversation as the
    # session grows, and what makes a compaction survivable rather than lossy.
    # The last line is the same discipline applied to the write leg: the exact
    # command is printed so the agent can write a correction before the
    # conversation is next compacted, without leaving the block to look it up.
    Given the habit register holds two red habits and two active habits
    And the saga's frontier chapter is known
    And an equilibrium reading is held for the current flow store
    And a correction has been written against an open commitment
    When the emitter runs because a turn is beginning
    Then the block it writes is at most twenty lines long
    And that block names the top red habit — the first red in the register's own order — and the check that would prove it
    And that block names the active habits and the standing limit of two
    And that block names the saga's frontier chapter
    And that block names the frontier — the count of still-owed commitments — beside the equilibrium reading for that stock
    And that block names the newest correction, which is the last one appended to the flow store
    And the block's final line is the exact command that writes a correction

  @concern:run-plane-projection
  Scenario: A register already over the fence is shown as exceeded, not renormalised
    # The fence line is only worth printing if it can read as broken. A block
    # that can only ever say "two or fewer" reports the rule back to itself and
    # proves nothing. So: the register is over the limit before the turn begins,
    # and the block has to say so rather than dropping the third habit to make
    # the line look compliant.
    Given the habit register holds three active habits
    When the emitter runs because a turn is beginning
    Then that block names all three active habits against the standing limit of two
    And that block marks the fence as exceeded
    And the emitter reports success, so a breached fence never fails the turn

  @concern:run-plane-projection
  Scenario: An input the block cannot read costs one line, never the turn
    # A state summary that can fail the turn it is summarising is a worse deal
    # than no summary. Partial sight, plainly partial, is the contract.
    Given the saga register's read hangs rather than returning
    When the emitter runs because a turn is beginning
    Then the emitter still writes a block
    And that block omits only the line it could not derive
    And no error text or stack trace appears anywhere in what the emitter wrote
    And the emitter finishes within the five-second budget declared for it in .claude/settings.json rather than waiting on the failed read
    And the emitter reports success, so the turn is never failed by it

  @concern:run-plane-projection
  Scenario: A stale equilibrium reading says so instead of reporting an old rate
    # "We have not measured since the record changed" and "the work is draining"
    # are different claims, and the block must never let the first pass as the
    # second. The reading carries the state of the flow store it came from, so
    # staleness can be seen rather than guessed at, and is reported when it is.
    # Making a reading stale on purpose needs nothing exotic: change the flow
    # store after the reading was derived from it — the second Given below does
    # it by projecting one new correction in — and the tag the reading carries
    # no longer matches the store's state.
    Given an equilibrium check has run and its reading is held in the emitter's cache
    And a new correction has been written into the flow store since that reading was derived
    When the emitter runs for a turn later in the same session — after the session-start refresh, so the check is not re-run
    Then the equilibrium line of the block it writes reads as awaiting a fresh measurement
    And that line does not report the rate the stale reading still holds
    And the fence, frontier, and correction lines are derived fresh from their registers, untouched by the staleness of the reading

  # ══════════════════════════════════════════════════════════════════════════
  # 3. The equilibrium verdict — filling, holding, or unmeasured
  #    This check is what produces the reading that section 2's block displays.
  #    The check is the `epr flow stocks` command named at the top of this file,
  #    and its window is declared on the invocation rather than inferred from the
  #    data: two dates and a period, `--window START..END --per week`. So "a
  #    declared week" below is exactly that — a start date, an end date, and a
  #    period of one week, chosen by whoever runs the check. What falls inside it
  #    is arranged the only way anything reaches the flow store: by projecting a
  #    repository whose plan gained (or did not gain) claimed work items in that
  #    span, and whose fulfillments named (or did not name) those commitments.
  #    The Background's one commitment was projected BEFORE any window below
  #    starts, so it sits in the level every scenario inherits — but inside no
  #    window's rates. Every "gained" below counts what fell inside the window.
  # ══════════════════════════════════════════════════════════════════════════

  @concern:dev-system-equilibrium
  Scenario: A stock taking on work faster than it finishes it fails the check
    # The red this habit exists to write. A development system that mints more
    # promises per window than it discharges is accumulating debt whether or not
    # anybody feels it that week — and the check says so out loud rather than
    # leaving it to be noticed later.
    Given a declared window of one week in which the projected repository gained two claimed work items
    And no fulfillment in that week named either of those commitments or the background commitment
    When the equilibrium check runs over the commitment stock for that declared window
    Then the check exits with a non-zero status
    And the reading names the commitment stock as filling
    And the reading states inflow and outflow as rates over that window, not as a level against a ceiling
    And the reading names the fulfillment path as the only discharge path it counted as drain

  @concern:dev-system-equilibrium
  Scenario: A stock finishing work at least as fast as it takes it on passes
    # Equal rates pass. The promise is that the stock is not growing, not that
    # it is shrinking, so the verdict has to name both of those as the same
    # acceptable state rather than claiming a drain that did not happen.
    Given a declared window of one week in which the projected repository gained one claimed work item
    And one fulfillment in that week named the commitment from the Background, so as many were discharged in the window as were minted in it
    When the equilibrium check runs over the commitment stock for that declared window
    Then the check exits with a zero status
    And the reading names the commitment stock as holding or draining

  @concern:dev-system-equilibrium
  Scenario: A window with nothing to measure refuses rather than reporting equilibrium
    # The over-claim this whole vocabulary exists to prevent. "We cannot see the
    # drain" and "the drain is adequate" must never arrive as the same verdict —
    # a green earned by measuring nothing is the most expensive kind of green.
    Given a declared window of one week in which the projected repository gained no claimed work item
    And no fulfillment in that week named any commitment — the background commitment predates the window, so nothing falls inside it to measure
    When the equilibrium check runs over the commitment stock for that declared window
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
    Given a declared window of one week in which the projected repository gained one claimed work item
    And a fulfillment in that week named that same commitment, discharging it
    When the equilibrium check runs over the commitment stock for that declared window
    Then the reading counts that discharge exactly once as outflow
    And the reading does not count that discharge as inflow
    And the reading names the commitment stock as holding or draining rather than filling
    And the check exits with a zero status
