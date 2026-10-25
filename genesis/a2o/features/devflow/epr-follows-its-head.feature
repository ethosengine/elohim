@e2e @devflow @concern:household-follows-head @act:host @requires:epr-cli
Feature: Matthew's workspace follows its landed epr head
  epr is the protocol's command-line evaluator: a session's governance checks, flow commands and
  memory recall all run through it, so a session that silently runs the wrong build silently
  changes what those operations accept. Matthew's development workspace is one of his devices. It
  keeps epr releases by CID (content identifier: the hash of the release's executable inventory) and one landed head:
  the build a new session should run. A build is first ingested into that store, then landed as
  the head only if all four hold: it came from a clean commit (built with no uncommitted changes); that commit is published, meaning
  it is contained in the development branch the device follows (its own `dev` or the remote's);
  it descends from the current head, so its source commit has the head's source commit in its git
  history; and it passes its health check (the evaluator names its own executable CID and answers a read-only flow
  command). "Lands" below means both steps; a build ingested but not landed stays in the store,
  available to pin or to revert to. A revert moves the head backward deliberately, and only to a
  published build. The store's verbs (ingest, land, resolve, status, gc) are `epr bin`; `epr` on
  PATH is a launcher that asks the same store which build to run. The first launcher resolution
  in a session pins that head, so a gate may land a newer build without changing the evaluator
  underneath work already in progress. A build ingested from an unfinished, dirty tree remains
  available for deliberate testing, but it cannot silently become the device's head.

  "Reports build B's CID" below means the reported executable CID matches the epr entry in
  build B's release inventory; the store itself resolves the release CID.
  "Everywhere" means the two selection surfaces agree: running `epr` through the stable launcher
  and asking the store which build it resolves. In these scenarios the launcher and store live in
  a temporary device directory and the source history lives in a temporary git repository.
  Builds A and B are real epr evaluator copies padded with different trailing bytes, so each has a
  distinct release CID. Each release carries epr and bin-launch; `epr --version` names the
  executable bytes, which must match that release's inventory. A session normally takes its identity
  from the agent runtime process it runs under (Claude Code, Codex, …); a session with no agent
  runtime above it keys its pin by the session ID it declares in ELOHIM_SESSION_ID.
  `EPR_PIN_CID` is different: it is a deliberate explicit pin for one launch inside the same
  session. It creates no session and moves neither that session's pin nor the device head.

  The peer daemon is the same epr binary running as a long-lived background process (`epr peer
  serve`, the device's link to its human's other devices). Because it outlives any session, it
  runs the device's landed head, never the pin of the session that started it, and it holds a
  lease on the build it runs: a record in the store naming that build, kept while the daemon's
  process lives. Garbage collection removes stored builds the device no longer needs. It keeps
  the head, the previous head, builds ingested within the last 14 days, and every build a live
  session pin or daemon lease names.

  Older devices may still use a Python launcher and addresses beginning with sha256:. Their
  installed epr has no bin command for managing the native store. Their start hook queues a
  background build, then the new native commands migrate the store and install its launcher.
  Historical single-executable releases remain valid; the new release carries both epr and its
  launcher. Matthew can then run the landed evaluator.

  Background:
    Given Matthew's workspace has an isolated device directory and source history

  Scenario: A new session runs the landed head everywhere
    Given clean build A is the landed epr head
    When clean build B lands from the next source commit
    And Matthew opens a fresh session with no agent runtime above it
    Then epr through the launcher reports build B's CID
    And the store resolves build B's CID for that session

  Scenario: An open session keeps its pin
    Given clean build A is the landed epr head
    And Matthew's open session has run epr through the launcher
    When clean build B lands from the next source commit
    Then the open session still runs build A's CID
    And the store reports that session behind the head
    And the open session receives one notice naming the landed release
    When Matthew explicitly pins build B for that session
    Then epr through the launcher reports build B's CID

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
    And that session's own launcher resolution is still build A's CID
    And build A remains the landed epr head

  Scenario: Moving backward needs a revert
    Given clean build A is the landed epr head
    And clean build B lands from the next source commit
    And Matthew checks out build A's older source commit
    When Matthew tries to land the stored build A
    Then the landing is refused as "non-descendant"
    When Matthew promotes the stored build A as head with --revert
    Then the revert succeeds and build A is the landed epr head again

  Scenario: A device on a pre-native epr arrives on the native store
    Given instead Matthew's device has a legacy sha256 store and a launcher without the bin verb
    When the device's start hook queues a native bootstrap
    Then the detached bootstrap lands a native release CID and replaces the legacy launcher
    And the replacement launcher runs the landed native evaluator

  Scenario: A changed executable inventory upgrades a release from the same source commit
    Given the gate stored paired build B before an older arrival landed single-file build A from that same commit
    And Matthew's open session has run epr through the launcher
    When the device applies and follows its declared epr inventory
    Then the device lands the paired release and installs its launcher
    And the open session still runs build A's CID

  Scenario: A daemon follows the device head, not the session that started it
    Given clean build A is the landed epr head
    And Matthew's open session has run epr through the launcher
    And clean build B lands from the next source commit
    When that pinned session starts the peer daemon
    Then the peer daemon's lease and status name build B as its running build
    And the open session still runs build A's CID

  Scenario: A running daemon's build survives garbage collection
    Given clean build A is the landed epr head
    And Matthew's open session has run epr through the launcher
    And clean build B lands from the next source commit
    And that pinned session starts the peer daemon
    When the head is reverted to build A and build B loses previous-head and 14-day history protection
    And garbage collection runs while the peer daemon lives
    Then build B remains in the store, held by the daemon's lease

  @regression
  Scenario: An unpublished feature build cannot become the head even with a revert
    Given clean build A is the landed epr head
    When build B is ingested from a clean unpublished feature branch
    And Matthew tries to land build B
    Then the landing is refused as "unpublished"
    And build A remains the landed epr head
    When Matthew tries to revert to stored build B
    Then the landing is refused as "unpublished"
    And build A remains the landed epr head
