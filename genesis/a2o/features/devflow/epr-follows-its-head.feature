@e2e @devflow @concern:household-follows-head @act:host @requires:epr-cli
Feature: Matthew's workspace follows its landed epr head
  epr is the protocol's command-line evaluator: a session's governance checks, flow commands and
  memory recall all run through it, so a session that silently runs the wrong build silently
  changes what those operations accept. Matthew's development workspace is one of his devices. It
  keeps epr builds by CID (content identifier: the hash of the build's bytes) and one landed head:
  the build a new session should run. A build is first ingested into that store, then landed as
  the head only if it came from a clean commit, passes its health check (it names its own CID and
  answers a read-only flow command) and descends from the current head: its source commit has the
  head's source commit in its git history. "Lands" below means both steps; a build ingested but
  not landed stays in the store, available to pin or to revert to. The store and its verbs
  (ingest, land, resolve, status) are `bin-store`, the device's tool for every followed binary;
  `epr` on PATH is a launcher that asks the same store which build to run. The first launcher
  resolution in a session pins that head, so a gate may land a newer build without changing the
  evaluator underneath work already in progress. A build ingested from an unfinished, dirty tree
  remains available for deliberate testing, but it cannot silently become the device's head.

  In this first slice, “everywhere” means its two selection surfaces agree: running `epr` through
  the stable launcher and asking `bin-store --tool epr resolve`. In these scenarios the launcher and store
  live in a temporary device directory, with no prior head or session pin, and the source history
  lives in a temporary git repository. Builds A and B are real epr evaluator copies padded with
  different trailing bytes, so each has a distinct CID and `epr --version` names what actually ran.
  A session normally takes its identity from the Claude Code agent process it runs under; a
  session with no such process above it ("no Claude ancestor") keys its pin by the session ID it
  declares in ELOHIM_SESSION_ID.
  `EPR_PIN_CID` is different: it is a deliberate explicit pin for one launch inside the same
  session. It creates no session and moves neither that session's pin nor the device head.

  Background:
    Given Matthew's workspace has an isolated empty epr store, launcher, and source history

  Scenario: A new session runs the landed head everywhere
    Given clean build A is the landed epr head
    When clean build B lands from the next source commit
    And Matthew opens a fresh session with no Claude ancestor
    Then epr through the launcher reports build B's CID
    And bin-store for epr resolves build B's CID for that session

  Scenario: An open session keeps its pin
    Given clean build A is the landed epr head
    And Matthew's open session has run epr through the launcher
    When clean build B lands from the next source commit
    Then the open session still runs build A's CID
    And bin-store for epr reports that session behind the head

  Scenario: A build that fails its health check never becomes the head
    Given clean build A is the landed epr head
    When build B is ingested from a clean commit whose health check fails
    And Matthew tries to land build B
    Then the landing is refused as "health-flow-failed"
    And build A remains the landed epr head

  Scenario: A build that reports the wrong CID never becomes the head
    Given clean build A is the landed epr head
    When build B is ingested from a clean commit but reports the wrong CID during health
    And Matthew tries to land build B
    Then the landing is refused as "health-identity-mismatch"
    And build A remains the landed epr head

  Scenario: A dev-context build runs only when pinned
    Given clean build A is the landed epr head
    And build B is ingested from Matthew's dirty working tree
    When Matthew tries to land build B
    Then the landing is refused as "dirty-build"
    And a fresh unpinned session runs build A's CID
    When Matthew launches build B with "EPR_PIN_CID"
    Then the launcher reports build B's CID
    And that session remains pinned to build A's CID
    And build A remains the landed epr head

  Scenario: Moving backward needs a revert
    Given clean build A is the landed epr head
    And clean build B lands from the next source commit
    And Matthew checks out build A's older source commit
    When Matthew tries to land the stored build A
    Then the landing is refused as "non-descendant"
    When Matthew promotes the stored build A as head with --revert
    Then the revert succeeds and build A is the landed epr head again
