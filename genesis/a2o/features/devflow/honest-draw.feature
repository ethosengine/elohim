@e2e @devflow @concern:workspace-device-arrival @act:host @requires:epr-cli
Feature: A device judges a sibling's epr from what it says it needs, before fetching any of it
  Matthew keeps more than one device. When one of them lands a new build of epr (the Elohim
  protocol's command-line evaluator), it writes a record saying it holds that release, and the
  record is copied to (crosses to) his other devices. That crossed record is an offer: a
  device that follows epr, meaning it is set to keep its own epr at the newest landed build, can
  draw the release it names, fetching the sibling's bytes instead of compiling them, which saves
  a long build. A release is about 160 MB.

  A build is honest about what it needs. When a device records a build, it also records the
  build's requirement statement: what the executable needs from the machine that runs it, such
  as its processor architecture, its operating system and the oldest C library it will start on.
  The statement is read from the executable's own bytes, not from anything observed about the
  device that built it. It travels inside the crossed record, and the record also names the
  statement's content identifier (CID), so a reader can check that the statement it holds is
  the one recorded.

  The statement cannot say which processor instructions the C code bundled inside epr needs
  (its cryptography and its database are written in C). Every release is meant to compile that
  C for the baseline instruction set of its platform, which on 64-bit Intel and AMD Linux is
  plain x86-64, the set every such processor has. The device that builds a release checks that
  it did, and only then adds a marker to its record: the C baseline, with the value `x86-64`.
  The value a drawing device expects is compiled into its own epr, one value per platform. A
  release whose record has no marker, or a different value, may need instructions the drawing
  device's processor lacks; that need is unmeasured, so it is not drawn.

  The scenarios that reach the marker check are written for a drawing device on that platform
  (64-bit x86 Linux with the GNU C library, `x86_64-unknown-linux-gnu`) and are tagged with it;
  they assume the epr under test was built for the device running them. An epr built for a
  platform with no declared baseline draws nothing from a sibling at all, so on a device of
  another platform those scenarios are skipped rather than run.

  Before the drawing device fetches a single byte, it checks four things, in order:
  - the commit is published here: the device lands only a commit that its integration branch
    contains, as it last fetched that branch (with brit, this workspace's repository tool);
  - the statement is readable: present, and matching the CID its record names;
  - the record carries the C baseline marker with the value this device's epr knows for its own
    platform;
  - this device meets the statement, judged by the device's own observation of itself.

  If any check fails, the device fetches nothing. It says which check failed and how to fix it.
  Then it goes on as if no sibling had offered anything: a device that can build compiles epr
  itself, and when the commit is not published here its own follow stops there too, since it
  lands only a published commit. Drawing is a shortcut, so a failed check here falls back to the device's own path
  rather than refusing the follow. (Bytes that arrive and do not make the release they were
  offered as are a different case, outside these scenarios: that refuses the follow.)

  In these scenarios device B follows epr and has opted to draw from a sibling first. Device A
  is B's admitted sibling: Matthew has enrolled both devices as his, so each accepts the
  other's records. A has recorded a release of epr for the commit B has checked out, and A's
  records carry the C baseline marker `x86-64` unless a scenario says otherwise. "Device B
  updates its epr" is the moment B acts on its follow setting. Each scenario names the one thing
  that differs.

  Background:
    Given Matthew's devices A and B are admitted to each other
    And device B follows epr and draws from a sibling first
    And device A has recorded a release of epr for the commit device B has checked out

  @requires:x86_64-unknown-linux-gnu
  Scenario: A release this device can run is drawn
    Given device B's integration branch contains that commit
    And the release's statement crossed with its record and matches its CID
    And device A's record carries the C baseline marker "x86-64"
    And device B meets every requirement in the statement
    When device B updates its epr
    Then device B fetches device A's release

  @requires:x86_64-unknown-linux-gnu
  Scenario Outline: A release whose record does not carry the C baseline this device knows is not drawn
    Both cases are the same unmeasured requirement with the same cure: the refusal quotes the
    marker the record carries, or says it carries none.

    Given device B's integration branch contains that commit
    And the release's statement crossed with its record and matches its CID
    And device B meets every requirement in the statement
    But device A's record carries <marker>
    When device B updates its epr
    Then device B fetches nothing
    And device B says the instructions the release's C needs are unmeasured, naming what the record carries: <said>
    And device B says A should rebuild for the declared platform, or B should build for itself
    And device B goes on to its own build

    Examples:
      | marker                            | said                          |
      | no C baseline marker              | "records no env:isa-c"        |
      | the C baseline marker "x86-64-v3" | "records env:isa-c=x86-64-v3" |

  @requires:x86_64-unknown-linux-gnu
  Scenario: A release this device cannot run is refused from its statement, and nothing is fetched
    Given device B's integration branch contains that commit
    And the release's statement crossed with its record and matches its CID
    And the release's statement says it needs a newer C library than device B has
    When device B updates its epr
    Then device B fetches nothing
    And device B says the release cannot run here, naming the unmet C library requirement
    And device B goes on to its own build

  Scenario: A release recorded before statements crossed is not drawn
    Given device B's integration branch contains that commit
    And device A recorded the release with an epr from before statements crossed, so the record names its statement's CID but does not carry the statement
    When device B updates its epr
    Then device B fetches nothing
    And device B says the offer predates the crossing statement and that A should record the release again with a newer epr
    And device B goes on to its own build

  Scenario: A statement that does not match its CID is not trusted
    Given device B's integration branch contains that commit
    And the release's statement crossed with its record but does not match its CID
    When device B updates its epr
    Then device B fetches nothing
    And device B says the statement cannot be verified
    And device B goes on to its own build

  Scenario: A commit that a stale integration branch does not contain is not drawn
    An unpublished commit stops the device's own follow too, so this one ends with nothing
    landed rather than with a build of its own.

    Given device B last fetched its integration branch before that commit landed on it
    When device B updates its epr
    Then device B fetches nothing
    And device B names each branch it checked, with the commit each one points to on this device
    And device B says to fetch the remote branch with "brit fetch -r origin"
    And device B lands nothing
