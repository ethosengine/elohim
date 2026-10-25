@e2e @devflow @concern:dataplane-convergence @act:host @requires:epr-cli
Feature: Each coding session keeps its leases, work claims and pinned runtime until it ends
  As Matthew, who delegates work to Claude Code and Codex on one device
  I want each agent session's leases, work claims and recall to be found under one session identity
  So that ending one session releases what it held without disturbing the other

  An agent session's envelope is the one record that names who is working: the session id, the
  device, the human that device stands for, and the agent runtime. Leases (a share of the device's
  capacity) and work claims (commitments to a piece of work) keep their own records; the envelope
  only lets them be found under that id.

  A session may also have been granted extra permissions for its work, such as a relaxed review
  rule for the files it is writing. Matthew grants these as the human responsible for the device,
  and the session's covenant is the record of them. Closing a session releases its leases, seals
  its covenant (freezes it, so what the session was allowed to do stays on the record after it
  ends), and writes exactly one close witness, an append-only note that the session ended. A
  session whose agent process dies without closing is found by the device's session collector,
  which notices the dead process and closes the session the same way. Closing or collecting a
  session that is already closed changes nothing.

  epr is the device's native command-line runtime. Every build of it is stored under its content
  identifier (CID), and a session runs one pinned build. Recall, the session's memory lookup,
  runs inside that pinned build, including from the hooks that run when a session starts and
  before each prompt. This is the device's share of dataplane convergence: a new build reaches a
  device as fast as it lands, and a session already running is never switched under itself.

  These scenarios run on a test device built for them: no real device key, stored build or
  running agent is changed.

  Scenario: Two agent sessions close independently, and a killed one is released
    Given Matthew's test device runs a Claude Code session and a Codex session, each started by its own agent runtime process
    When each session opens its envelope and takes its own leases and work claims
    Then each envelope lists only its own leases and claims, under its own session id
    When the Claude Code session is closed twice
    Then its leases are released, its covenant is sealed and exactly one close witness is written, while the Codex session still holds its lease
    When the Codex runtime process is killed and the device's session collector runs twice
    Then the Codex session's leases are released, its covenant is sealed and exactly one close witness is written for it

  @regression
  Scenario: Rebuilding epr mid-session cannot change which build the session's recall runs on
    Given Matthew's test device has a session pinned to one stored epr build
    When the session-start hook opens recall, and epr is rebuilt from source before the next prompt hook runs
    Then both hooks find recall under the session's envelope id and run the same pinned epr build, with the same CID
