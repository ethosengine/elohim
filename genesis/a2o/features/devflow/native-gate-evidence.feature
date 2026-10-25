# A gate verdict is keyed by the bytes it judged, not by the branch that carried them.
# Driven the way `steps/devflow/pin-attestation.steps.ts` drives the gate runner: every
# scenario mints its OWN scratch repository and runs the real gate-runner against it with
# GATE_ROOT. Habits: elohim/brit/.epr-meta/brit-native-daily-driver.habit.md and
# genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md. Plans:
# /projects/.claude-config/plans/brit-is-greenfield-we-elegant-journal.md (F1) and
# /projects/.claude-config/plans/sure-let-s-go-with-toasty-noodle.md (slices 1-2).
#
# The tags are suite-routing labels, not behaviour: @e2e and @devflow route this file
# into the devflow suite; @act:host says these scenarios run on the host alone and need
# no mesh peer; each @concern: tag joins its scenario to the habit that claims it.
# Every scenario spawns the `brit` and `brit-build-ref` binaries and
# carries @requires:brit-cli and @requires:brit-build-ref-cli — the tool-cap gate in
# src/framework/fixtures/substrate-scope.ts HOLDS them, never fails them, when a binary is
# absent. @wip until steps/devflow/native-gate-evidence.steps.ts exists.

@e2e @devflow @act:host @requires:brit-cli @requires:brit-build-ref-cli
Feature: A gate remembers the closure it judged, not the branch that carried it

  A local gate — the build, lint and test recipe a project runs before a push — is paid in
  minutes: the storage gate in this repository averages about thirty. Today that price is
  paid on every push, including a rebase that moved nothing the recipe reads, a docs-only
  commit beside the code, and the integrator's fast-forward of commits that were gated an
  hour earlier on the author's branch. The verdict was bound to a branch name and a push
  range, so nothing could say "these exact bytes were already judged".

  brit, the native developer tool (a command-line tool built on git), binds the verdict to
  its closure instead. A closure here is not a function: it is every input of the verdict
  that the repository can name. What it cannot name (a flaky test, a network that was down)
  stays outside it, so one closure can carry a red and later a green. It has five parts,
  and each part has its own scenario below:
  the content identity of the files the step declares as inputs at the commit being gated;
  the manifest that declares them; the recipe the gate runs; the environment the recipe
  ran under (the toolchain pin, the job cap); and the files the project's own code and
  tests read beyond those declared inputs. The gate runner finds those reads by scanning
  the code for the paths it opens. When a read cannot be named, the whole commit stands in
  for it, so only the identical commit can match. brit computes the closure's content identity:
  one hash over the content of every file the declared inputs match at that commit, the
  manifest file's own bytes, the recipe's bytes, and the declared environment values, so a
  declaration that changes moves the identity even when the files it matches do not. Its
  sibling command-line tool brit-build-ref reads and writes the signed attestations, which
  live as notes in the repository's own git refs; a closure may carry several, and the
  newest green is the one a hit names. Before a gate runs, the runner asks
  for a signed attestation of that closure and prints one line to its output: a hit with
  the signer, the age and the commit that carried it, or a miss. After the gate runs, the
  runner records the verdict it observed — red as well as green — against the same closure,
  and emits one cycle observation, as every gate run already does: one record of the run's
  duration and its kind, appended to the measure ledger (an append-only file of
  observations kept under the repository's status directory) — "full-gate" when a person ran it, "pre-push" when the push hook ran it,
  "reused" when an attestation stood in for it. The kind is what lets a person later read
  how many gate minutes the evidence saved; "pre-push" is the hook's kind and is proved
  where the hook is, not here.

  Every attestation is signed by the key of the workspace that ran the gate (one key per
  checkout, held beside its git directory and minted on first use). A person's roster is
  the signed list of the devices that are theirs; a device joins it through a ceremony
  between two of the person's devices (the new one asks, an existing one authorizes, the
  new one binds), and the resulting row travels in the repository like any other file. A
  signer will be admitted when its key belongs to a device on the author's roster, or is one
  of the named keys the repository admits (the integrator, CI). That needs brit-build-ref to
  check each attestation's signature before reporting it, and it does not yet. Until it does,
  the only admitted signer is this workspace itself: its attestations never leave this
  checkout, so a forged one would need write access to the checkout, which already
  controls the gate.

  The work arrives in slices, one rule each below. In the report-only slice, the default,
  the gate names the evidence it holds and still runs, whatever that evidence says. In the
  reuse slice the operator exports GATE_EVIDENCE=reuse before
  running the gate and lists the project as enabled for reuse. Then this workspace's own
  green hit stands in for the run, an unadmitted signer's is shown and not honored, and a
  red hit never stands in for anything: the gate runs again. The last rule is written ahead
  for roster and named-key admission, and its scenarios stay pending until brit-build-ref
  checks signatures.

  Every scenario mints its own scratch repository, so nothing it records touches this
  repository's refs or ledgers, and "this workspace" always means that scratch checkout.

  Background:
    Given a scratch repository with a gate project "sample" whose step declares "src/**" as its inputs
    And the code of project "sample" reads no file outside its declared inputs
    And the scratch repository holds its own workspace key
    And the project's recipe records every run it performs
    And brit and brit-build-ref are available to the gate runner

  Rule: In the report-only slice the gate names the evidence it holds and still runs

    Background:
      Given the operator has not exported GATE_EVIDENCE=reuse

    @wip @concern:brit-native-daily-driver
    Scenario: The first run of a closure is a miss, and the gate records what it saw
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: miss"
      And the recipe ran once
      And the closure of project "sample" at commit "A" carries one green attestation signed by this workspace
      And the cycle observation for that run records its kind as "full-gate"

    @wip @concern:brit-native-daily-driver
    Scenario: A commit outside the step's inputs leaves the closure unchanged, and the gate names the evidence it holds
      Given the gate ran green for project "sample" at commit "A"
      And a commit "B" changes only "docs/README.md"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" equals its closure at commit "A"
      And the gate printed "evidence: hit green" naming this workspace as signer and commit "A"
      And the recipe ran a second time, two runs in all
      And the cycle observation for that run records its kind as "full-gate"

    @wip @concern:brit-native-daily-driver
    Scenario: A change inside the step's inputs is a new closure
      Given the gate ran green for project "sample" at commit "A"
      And a commit "B" changes "src/lib.rs"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" differs from its closure at commit "A"
      And the gate printed "evidence: miss"
      And the closure of project "sample" at commit "A" still carries its green attestation

    @wip @concern:brit-native-daily-driver
    Scenario: A change to the manifest is a new closure, even with the sources untouched
      Given the gate ran green for project "sample" at commit "A"
      And a commit "B" changes only the manifest, widening the step's declared inputs to "src/** and build.rs"
      And no file named "build.rs" exists at commit "B"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" differs from its closure at commit "A"
      And the gate printed "evidence: miss"

    @wip @concern:brit-native-daily-driver
    Scenario: A change to the recipe is a new closure, even with the sources and the manifest untouched
      Given the gate ran green for project "sample" at commit "A"
      And a commit "B" changes only the body of the gate recipe the manifest names for project "sample"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" differs from its closure at commit "A"
      And the gate printed "evidence: miss"

    @wip @concern:push-gate-pays-once
    Scenario: A file the code reads beyond the declared inputs is part of the closure
      Given a test of project "sample" reads "fixtures/expected.json", which lies outside "src/**"
      And the gate ran green for project "sample" at commit "A"
      And a commit "B" changes only "fixtures/expected.json"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" differs from its closure at commit "A"
      And the gate printed "evidence: miss"

    @wip @concern:push-gate-pays-once
    Scenario: A read the scan cannot name makes the whole commit part of the closure
      Given a test of project "sample" walks the repository for files whose names it does not spell out
      And the gate ran green for project "sample" at commit "A"
      And a commit "B" changes only "docs/README.md"
      When the gate runs for project "sample" at commit "B"
      Then the closure of project "sample" at commit "B" differs from its closure at commit "A"
      And the gate printed "evidence: miss"

    @wip @concern:brit-native-daily-driver
    Scenario: The environment the recipe ran under is part of the closure
      Given the gate ran green for project "sample" at commit "A" with toolchain "1.96.1"
      When the gate runs for project "sample" at commit "A" with toolchain "1.99.0"
      Then the closure of project "sample" at commit "A" under toolchain "1.99.0" differs from its closure under toolchain "1.96.1"
      And the gate printed "evidence: miss"

    @wip @concern:brit-native-daily-driver
    Scenario: A red run is recorded against its closure
      Given the project's recipe fails
      When the gate runs for project "sample" at commit "A"
      Then the recipe ran once
      And the gate exits 1
      And the gate printed "evidence: miss"
      And the closure of project "sample" at commit "A" carries one red attestation signed by this workspace

    @wip @concern:brit-native-daily-driver
    Scenario: A recorded red is read back as red, never as green
      Given the gate ran red for project "sample" at commit "A"
      When the gate runs a second time for project "sample" at commit "A"
      Then the gate printed "evidence: hit red" naming this workspace as signer and commit "A"
      And the recipe ran a second time, two runs in all

    @wip @concern:brit-native-daily-driver
    Scenario: A fix inside the step's inputs is a new closure, and its green stands on its own
      Given the gate ran red for project "sample" at commit "A"
      And a commit "B" changes "src/lib.rs" so that the recipe succeeds
      When the gate runs for project "sample" at commit "B"
      Then the gate printed "evidence: miss"
      And the recipe ran once
      And the closure of project "sample" at commit "B" carries one green attestation signed by this workspace
      And the closure of project "sample" at commit "A" still carries only its red attestation

    @wip @concern:brit-native-daily-driver
    Scenario: A green run on a closure that carried a red supersedes it
      Given the gate ran red for project "sample" at commit "A"
      And the recipe now succeeds without any file changing
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: hit red" naming this workspace as signer and commit "A"
      And the recipe ran once
      And the closure of project "sample" at commit "A" carries a red attestation and a newer green one
      # The newest green is now the one a hit names.
      When the gate runs a third time for project "sample" at commit "A"
      Then the gate printed "evidence: hit green" naming this workspace as signer and commit "A"
      And the recipe ran a third time, three runs in all

  Rule: In the reuse slice this workspace's own green hit stands in for the run

    Background:
      Given the operator has exported GATE_EVIDENCE=reuse
      And project "sample" is enabled for reuse

    @wip @concern:push-gate-pays-once
    Scenario: This workspace's own green hit stands in for the run
      Given the gate ran green for project "sample" at commit "A"
      When the gate runs a second time for project "sample" at commit "A"
      Then the gate printed "reused" naming this workspace as signer and commit "A"
      And the recipe ran once in all
      And the cycle observation for that run records its kind as "reused"

    @wip @concern:push-gate-pays-once
    Scenario: A project not enabled for reuse runs even on its own green hit
      Given project "sample" is not enabled for reuse
      And the gate ran green for project "sample" at commit "A"
      When the gate runs a second time for project "sample" at commit "A"
      Then the gate printed "not reused: project not enabled for reuse"
      And the recipe ran a second time, two runs in all

    @wip @concern:push-gate-pays-once
    Scenario: Edited bytes in the working tree are not the commit's bytes, so the gate runs
      Given the gate ran green for project "sample" at commit "A"
      And "src/lib.rs" is edited in the working tree and not committed
      When the gate runs a second time for project "sample" at commit "A"
      Then the gate printed "not reused: declared inputs dirty"
      And the recipe ran a second time, two runs in all

    @wip @concern:push-gate-pays-once
    Scenario: A newer red on the same closure is never overruled by an older green
      Given the gate ran green for project "sample" at commit "A"
      And the gate then ran red for project "sample" at commit "A"
      When the gate runs a third time for project "sample" at commit "A"
      Then the gate printed "not reused: newest attestation is red"
      And the recipe ran a third time, three runs in all

    @wip @concern:push-gate-pays-once
    Scenario: Evidence from another workspace is shown and not honored
      Given the gate ran green for project "sample" at commit "A" on another workspace
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: hit green" naming the other workspace's key
      And the gate printed "not reused" naming the other workspace's key as not this workspace
      And the recipe ran once in this workspace

  Rule: Once brit-build-ref checks signatures, a roster device's or a named key's green hit stands in too

    Background:
      Given the operator has exported GATE_EVIDENCE=reuse
      And the author's roster in the scratch repository lists this workspace's device
      And a signer is admitted when its key is a device on the author's roster or a key the repository names

    @wip @concern:brit-native-daily-driver
    Scenario: Evidence from a signer the author does not admit is shown and not honored
      Given the gate ran green for project "sample" at commit "A" on another workspace whose key is on nobody's roster
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: hit green" naming the other workspace's key
      And the recipe ran, because an unadmitted signer's evidence is shown and not honored
      And the cycle observation for that run records its kind as "full-gate"

    @wip @concern:brit-native-daily-driver
    Scenario: An admitted signer's green hit stands in for the run
      Given the gate ran green for project "sample" at commit "A" on another device the author's roster lists
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: hit green" naming that device as signer
      And the recipe did not run
      And the cycle observation for that run records its kind as "reused" and names the attestation it stood on

    @wip @concern:brit-native-daily-driver
    Scenario: The integrator's fast-forward reuses the author's green through a key the repository admits by name
      Given the gate ran green for project "sample" at commit "A" on the author's device
      And the integrator's workspace key is one the repository admits by name, and is on nobody's roster
      When the integrator's workspace runs the gate for project "sample" at commit "A"
      Then the gate printed "evidence: hit green" naming the author's device as signer and commit "A"
      And the recipe did not run
      And the cycle observation for that run records its kind as "reused" and names the attestation it stood on

    @wip @concern:brit-native-daily-driver
    Scenario: A red hit from an admitted signer never stands in; the gate runs again
      Given the gate ran red for project "sample" at commit "A" on another device the author's roster lists
      And the recipe now succeeds without any file changing
      When the gate runs for project "sample" at commit "A"
      Then the gate printed "evidence: hit red" naming that device as signer
      And the recipe ran once
      And the cycle observation for that run records its kind as "full-gate"
