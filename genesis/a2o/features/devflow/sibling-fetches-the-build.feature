@e2e @devflow @act:host @requires:epr-cli @local @concern:sprint-lands-each-closure-once
Feature: A sibling device fetches a build instead of rebuilding it
  Matthew uses more than one admitted device for development. When one device has
  already built the exact closure another needs, the sibling should receive the
  verified binary quickly, without trusting an address or escaping its grant.
  Matthew can see whether the bytes were reused or rebuilt and why.

  Words used below:
  - A build KEY is derived from the build's inputs (its method plus the projected source), so the same inputs give the same key. Different outputs under one key are recorded as divergence; a pinned output can still be fetched from holders that agree with it.
  - A VARIANT is a build for a declared target: a CPU instruction set and runtime requirements. A pin can name a set of variants of the same tool, each with its own key and content fingerprint.
  - A PIN names which variants of a build a device will accept; a TOOL PIN is the pin a device follows for one tool, and a VARIANT-SET PIN names the expected output for each variant of one key.
  - A CID is a content address, the hash of the bytes: bytes either verify against it or are refused.
  - A build output is cut into CHUNKS; its MANIFEST lists every chunk by CID. The build record names the manifest CID and the output CID separately.
  - A METHOD names a build recipe (mesh-build); a STEP names one build of it (mesh-build:elohim-storage). Serve grants are per method.
  - A HOLDER is a device that holds a verified output and serves it. A holder serves a method only to the audiences its serve list names; Matthew's serve list grants method mesh-build to every admitted device. ADMITTED means on Matthew's device roster.
  - The PAYLOAD STORE is the one store on a device that holds verified bytes.
  - PRODUCE is the builder's record that it made an output; USE is a device's record that it consumed bytes. A serve-blob Use is the serving device's record of the bytes it delivered.
  - WASTED BYTES are bytes received but discarded before verification, recorded as a measured quantity.

  Background:
    Given matthew's admitted device "A" runs a peer daemon, can build "elohim-storage", and holds a serve list granting method "mesh-build" to admitted devices
    And matthew's admitted device "B" runs a peer daemon, needs "elohim-storage", and can build it locally
    And matthew's admitted device "matthew-node" runs a peer daemon, holds outputs as an alternate holder, serves them, and holds a serve list granting method "mesh-build" to admitted devices
    And devices A and B use the step "mesh-build:elohim-storage"
    And the fetch budget allows 8 chunks in flight per holder, a 30 second stall, a 1800 second total, and a 1 MiB manifest limit

  # Variant selection steps are not wired yet.
  @wip @concern:sprint-lands-each-closure-once
  Scenario: The v2 device fetches the compatible variant without downloading the v3 build
    Given device B observes an x86-64-v2 CPU with AVX but without AVX2
    And its tool pin offers v2 and v3 variants held by device A
    And each variant's requirements can be read without fetching its bytes
    And device B meets both variants' runtime library requirements and device A grants it access to both
    When the v2 device resolves its pin
    Then it refuses the v3 variant with reason "unmet" on axis "isa.x86", naming the required and observed features including the missing AVX2
    And it fetches no binary bytes of the v3 variant and never launches it
    And it derives the selected v2 variant's build key itself and fetches only that variant
    And it verifies the received bytes against the selected content fingerprint before installing them
    And its pin is unchanged and no local rebuild is needed

  @wip
  Scenario: Device B installs the exact build that device A produced
    Given device A has recorded a successful Produce for key "storage-key-1", output "storage-output-1", and its chunk manifest
    And device A's serve list grants device B method "mesh-build"
    When device B resolves the build record by key "storage-key-1"
    And device B fetches every chunk named by the resolved manifest
    Then every received chunk verifies against its own CID before device B stores it
    And the assembled binary verifies against output CID "storage-output-1"
    And device B installs the verified binary as "elohim-storage"
    And device B records one Use for "storage-output-1" naming device A's Produce
    And device A records one completed serve-blob Use naming device B and the bytes served

  @wip
  Scenario: An accepted build record whose bytes are unavailable becomes a typed miss and a local build
    Given device B resolves an accepted build record for key "storage-key-2" from device A
    And device A has no reachable route that can supply the record's output bytes
    When device B tries to fetch that output
    Then the fetch prints "FETCH MISS: NoRoute(device=A)"
    And the fetch records no successful artifact Use
    And device B builds "elohim-storage" locally
    And the build output prints "building locally after NoRoute(device=A)"
    And device B records a Produce for the local build

  @wip
  Scenario: An interrupted fetch asks the remaining holder only for missing chunks
    Given devices A and "matthew-node" hold the same output for key "storage-key-3"
    And device B already has 120 verified chunks of that output
    And device A's connection closes after device B receives 40 more verified chunks
    When device B resumes the fetch from device "matthew-node"
    Then no request names any of the 160 chunks device B already verified
    And the completed binary verifies against the resolved output CID before installation
    And the fetch record names both holders and the number of resumed chunks

  @wip @concern:sprint-lands-each-closure-once
  Scenario: A trickling holder is abandoned within budget and its chunk is re-raced
    Given device B is fetching key "storage-key-6" from device A
    And device A trickles one requested chunk without completing it
    And that chunk is the last one device B needs to complete the output
    And device "matthew-node" can deliver the same chunk within the 1800 second total
    When device B waits on device A past the 30 second stall
    Then device B abandons device A without waiting for another byte from it
    And the unfinished chunk is re-raced to device "matthew-node"
    And the fetch completes within the 1800 second total
    And bytes received from device A and discarded before verification are recorded as wasted bytes

  @wip
  Scenario: With no other holder a trickling holder ends the fetch at the total budget
    Given device B is fetching key "storage-key-7" from device A
    And device A trickles one requested chunk without completing it
    And no other device holds that output
    When device B reaches the 1800 second total
    Then device B abandons device A
    And device B prints "DeadlineExceeded" with fetched and total byte counts
    And no binary is installed

  @wip
  Scenario: An oversized manifest is refused before any chunk moves
    Given device A offers a manifest larger than the 1 MiB manifest limit
    When device B requests the manifest
    Then device B prints "ManifestTooLarge" with the offered length and the 1 MiB manifest limit
    And device B sends no chunk request
    And device B receives and stores zero chunk bytes
    And no binary is installed

  @wip
  Scenario Outline: Swapping the manifest CID and output CID is refused in either direction
    Given a build record names manifest CID "manifest-1" and output CID "output-1"
    And device A presents <swap>
    When device B validates the record and manifest before requesting chunks
    Then device B prints "ManifestRejected" with reason <reason>
    And device B sends no chunk request
    And no binary is installed

    Examples:
      | swap                                            | reason                                      |
      | "output-1 where the manifest CID must appear"  | "output CID used as manifest CID"          |
      | "manifest-1 where the output CID must appear"  | "manifest CID used as record output CID"   |

  @wip @concern:sprint-lands-each-closure-once
  Scenario: Different outputs for one derivation key leave the pinned output available
    Given device A advertises output "storage-output-a" for key "storage-key-4"
    And device "matthew-node" advertises output "storage-output-b" for key "storage-key-4"
    And device B's variant-set pin names "storage-output-a" for that key
    When device B resolves key "storage-key-4"
    Then device B records divergence naming both output CIDs
    And device B fetches from device A and verifies the bytes against "storage-output-a"
    And device B fetches no bytes of "storage-output-b" and does not rebuild locally

  @wip
  Scenario: An admitted device outside the artifact grant receives a named refusal
    Given device B is admitted by Matthew's roster
    And device B asks device A for an artifact produced by method "not-shared"
    And device A's serve list does not grant device B method "not-shared"
    When device B requests that artifact's manifest
    Then device B receives refusal "OutsideStanding" with label "serve-blob:not-shared"
    And device A sends no manifest or chunk bytes
    And device A records one refused serve-blob Use for device B with quantity 0

  @wip
  Scenario: A stranger receives the same refusal regardless of the artifact's grant status
    Given a requester with no admitted device identity asks device A for a granted artifact, an ungranted artifact, and an unknown artifact
    When device A answers the three manifest requests
    Then the three refusal responses are byte-identical
    And none names an artifact, method, grant, or policy label
    And device A sends no manifest or chunk bytes
    And device A counts the refusals without writing a Use that identifies the stranger

  @wip @concern:sprint-lands-each-closure-once
  Scenario Outline: Each transport mode either delivers the verified artifact or reports its typed gap
    Given device A offers key "storage-key-5" to device B over transport mode <mode>
    And the offered output is 16 MiB and its manifest is within the 1 MiB limit
    And <condition>
    When device B attempts to fetch and verify the output
    Then the fetch reports <outcome>
    And a transport that reports a typed gap installs no fetched bytes

    Examples:
      | mode         | condition                                                     | outcome                                  |
      | "direct"     | "the direct address is reachable and delivers at least 4 MiB in each 10 second window" | "FETCHED"              |
      | "relay"      | "the stream delivers at least 4 MiB in each 10 second window" | "FETCHED"                               |
      | "iroh-blobs" | "the payload store offers no iroh-blobs byte path"  | "TransportCannotDeliver(iroh-blobs)"    |

  @wip
  Scenario: The 454 MB storage binary arrives faster than rebuilding it
    Given device A has produced a 454 MB "elohim-storage" binary for the exact key device B needs
    And device B can reach device A directly on the household network
    When device B times a verified fetch and install of that binary
    And the fixture records the duration of a local rebuild of the same key on device B
    Then the installed binary verifies against the Produce output CID
    And the fetch Use records 454 MB, elapsed seconds, and device A's Produce
    And the measured fetch duration is less than the measured rebuild duration
