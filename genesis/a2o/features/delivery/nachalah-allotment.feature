@e2e @delivery @elohim @concern:nachalah-allotment
Feature: A household earns its holding responsibilities without risking its records
  Matthew wants his household's working notes to stay within the household,
  its collective's deeds to survive a household loss, and shared governance
  records to remain available across seven independently admitted hubs.
  Holding a record never grants permission to read it or choose its canonical version.
  The household adopts the runtime that enforces these boundaries through the
  same witnessed release ceremony it already uses for upgrades.

  A holding domain is a household that can fail independently of another household.
  A holding floor is the recoverable coverage and distribution across independent failure domains needed for the promised recovery.
  A membership epoch fixes the roster admitted to one space; changing that roster creates a successor space.
  An arc is the portion of a space's records assigned to one device, divided into sectors for coverage checks.
  Gold means governance records requiring seven diverse hubs holding the whole space;
  working notes and collective deeds use their narrower household or collective policies.
  A holding commitment records an assignment authorized by that space's named stewards.
  The controller proposes and tracks those assignments; it cannot grant itself authority.
  Ark supervises local processes, and a berth is their persistent device placement and credentials.
  For upgrades, the household's authorized stewards bound a provider's right to delegate installation.
  The provider grants James's peer, the recipient, permission for a specific release and device target.
  The release channel records the release chosen under its governance; election alone grants no installation permission.
  A candidate signer signs the proposed replacement and must hold the authority needed for that proposal.
  An Agent EPR identity names an agent's protocol record; a Holochain key signs its authored actions.
  Their relationship must be authenticated before either identity can stand for the other.
  Source chains preserve each agent's authored history; an unchanged PID means the same process keeps running.
  Actuation applies a proposed holding assignment, while DHT authority is the duty to validate and serve its records.
  A lineage release is the authorized instruction to carry selected records into successor spaces.
  Alpha is the deployment under test; its canary is the first trial holder used to prove a release before wider adoption.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # Missing step definitions remain explicit. Native ark identity tests prove
  # one prerequisite only; they are not release, holding, or recovery receipts.
  @wip @requires:household-nodes
  Scenario: Household affirmation includes a member while excluding an outsider
    Given Matthew and Jessica have affirmed James as a member of their household
    And the household has a verified fixed membership epoch
    And an outsider knows the household space's network seed
    When James authors a household working note in that epoch's space
    Then James can author without first earning the external reputation required of an unconnected stranger
    And the household's admitted devices can hold and validate the note
    And the outsider cannot join the space or receive its restricted payload or metadata

  @wip @requires:household-nodes
  Scenario: Losing one device does not lose a household working note
    Given Matthew has authored a household working note
    And its holding evidence satisfies recovery after any one household device loss
    When the only device with the original authored copy becomes unavailable
    Then Jessica reconstructs the exact note from the surviving authorized copies
    And reconstruction preserves the note's authorship and canonical-version history

  @wip @requires:household-nodes
  Scenario: Insufficient recovery capacity remains visible without preventing local work
    Given Matthew's household has only one available device
    When Matthew authors a private working note
    Then the note remains available on that device
    And the household reports that its one-device-loss recovery floor is unmet
    And it does not certify a second holding domain from an unknown peer identity

  @wip @requires:household-nodes
  Scenario: A backup custodian helps recovery without learning the note
    Given Jessica has authorized encrypted backup custody outside her household
    And the custodian holds a verified reconstructable encrypted copy of her working note
    And an authorized surviving household device retains the required decryption keys
    And the custodian has no reading permission for her working note
    When Jessica loses the local plaintext copy
    Then an authorized household device reconstructs and decrypts the exact note
    And the custodian cannot obtain its plaintext or restricted metadata through any egress path
    And restricted metadata includes its title, author, household roster, and record relationships
    And custody gives the custodian no canonical-version authority

  @wip @requires:household-nodes
  Scenario: A collective deed survives the loss of any one holding household
    Given a collective deed is held in three evidenced household holding domains
    And each domain has consented to its assignment in the collective's membership epoch
    When each holding household is made unavailable in a separate recovery run
    Then the remaining authorized households reconstruct the exact deed in every run
    And a peer whose household identity is unknown does not count toward the floor
    And this single-host exercise certifies fault recovery without claiming physical diversity

  @wip @requires:household-nodes
  Scenario: Wider disclosure requires a witnessed promotion
    Given Matthew's household note has not been promoted beyond its household
    When a collective requests the note
    Then the collective receives no restricted plaintext or metadata
    When Matthew and Jessica as the household's authorized stewards witness promotion of that note's exact version to the collective
    And the collective verifies the receiving holding floor and its admission epoch
    And an authorized collective reader retries the read for the promoted version
    Then only the promoted version becomes available to authorized collective readers
    And the promotion retains its source provenance

  @wip @requires:household-nodes
  Scenario: A successor space carries only the authorized record classes
    Given Matthew's predecessor space contains working notes, economic deeds, and governance records
    And a lineage release selects working notes for a household successor, economic deeds for a collective successor, and governance records for a governance successor
    And that release binds each successor's destination seed and record-class selection
    And Matthew has transferred only part of the selected records when the carry is interrupted
    And his predecessor space still serves reads while the successor transfers are incomplete
    When Matthew resumes an interrupted carry using the same lineage release
    And Matthew repeats adoption after the resumed carry completes
    Then each admitted successor contains exactly its selected records and required authorized dependencies
    And excluded payloads and metadata are absent from its carry witnesses
    And predecessor reads remain available until destination verification succeeds
    And Matthew keeps the same agent keys and reusable source chains

  @wip @requires:household-nodes
  Scenario: A joining epoch cannot inherit membership from its seed alone
    Given Jessica's household stewards have removed James from its next membership roster through the authorized ceremony
    And James holds an assignment valid only in the predecessor epoch
    When Jessica's household installs a successor membership epoch beside its predecessor
    Then admission is checked against the fixed verified roster of the successor
    When James attempts to join and claim successor responsibility using that assignment
    Then the stale-epoch assignment authorizes neither admission nor a successor holding transition
    And the migration does not claim to undo disclosure already made in the predecessor

  @wip @requires:household-nodes
  Scenario: A partial authority validates fully and serves reads through the network
    Given Matthew's non-gold space uses verified partial-arc assignments
    And Jessica holds a valid record outside Matthew's assigned arc
    When Matthew requests that record and its validation dependencies
    Then Matthew receives the valid record through authority discovery and network reads
    And each assigned authority performs full integrity validation

  @wip @requires:household-nodes
  Scenario: A partial authority rejects invalid operations even after a block is lifted
    Given Jessica fully validates her assigned portion of the collective space
    When James publishes an invalid operation for that portion
    Then Jessica rejects the operation and the resulting warrant reaches its assigned authorities
    Given a scoped peer block now prevents James from communicating in that space
    When the collective's authorized stewards lift a narrowly scoped peer block so James can communicate again
    And James republishes the same invalid operation
    Then Jessica still rejects it
    When James publishes a valid record for Jessica's portion
    Then publish routing and subsequent gossip deliver it to the assigned authorities

  @wip @requires:household-nodes
  Scenario Outline: Responsibility is not retired before its replacement is ready
    Given Matthew holds an incumbent assignment in a non-gold space
    And the space's authorized stewards have issued a commitment assigning replacement responsibility to Jessica
    When the transfer encounters "<interruption>" while James continues authoring records
    Then Matthew retains responsibility until Jessica has verified the records and dependencies including James's new writes
    And only synchronized coverage is advertised
    And overlapping transitions are serialized against the predecessor assignment
    And incumbent authority retires only after every affected sector retains its required independent holding floor
    And retiring DHT authority does not itself delete the last recoverable bytes

    Examples:
      | interruption             |
      | a supervisor restart     |
      | interrupted gossip       |
      | a conflicting assignment |
      | stale trust evidence     |
      | replacement failure      |

  @wip @requires:household-nodes
  Scenario: Revocation stops new disclosure while replacement preserves recovery
    Given the collective's authorized stewards have revoked Jessica's holding assignment
    And Jessica still has the last recoverable copy of one authorized record
    When the controller prepares a replacement assignment
    Then Jessica receives no newly disclosed restricted records
    And missing or unsigned trust evidence authorizes neither new grants nor shrinking
    And the last recoverable copy is retained until authorized replacement recovery is verified

  @wip @requires:household-nodes
  Scenario: A staged executable is not reported as the running release
    Given Matthew's ark supervises a conductor pinned by its executable digest
    And a replacement executable has been staged at its artifact path
    When Matthew inspects adoption readiness before the conductor is replaced
    Then the old running executable cannot certify adoption of the replacement
    And a readiness announcement from a different executable cannot satisfy the identity check
    And an unavailable executable observation cannot become positive adoption evidence

  @wip @requires:household-nodes
  Scenario Outline: An upgrade cannot borrow another agent's authority
    Given James's conductor is running with usable source chains
    And a compatible elected conductor candidate names an exactly authenticated signed grant
    But "<missing binding>" has not been authenticated
    When James's peer evaluates the candidate for activation
    Then activation is refused with an identity-binding reason
    And James's incumbent conductor keeps the same PID and usable source chains

    Examples:
      | missing binding                                      |
      | the grant author to the provider's Agent EPR identity  |
      | the acting peer to the grant recipient                |
      | the candidate signer to its delegated authority       |

  @wip @requires:household-nodes
  Scenario Outline: Incomplete grant status cannot interrupt a working conductor
    Given James's conductor is running with usable source chains
    And a compatible elected conductor candidate has an authenticated in-scope delegation
    But the available grant-status evidence is "<evidence>"
    When James's peer evaluates the candidate immediately before stopping the incumbent
    Then activation is refused with a grant-status reason
    And James's incumbent conductor keeps the same PID and usable source chains

    Examples:
      | evidence                                                    |
      | unavailable                                                 |
      | stale                                                       |
      | forged                                                      |
      | an authenticated revocation by the authorized revoker       |
      | an empty DHT search that may be missing a gossiped revocation |

  @wip @requires:household-nodes
  Scenario: A late revocation after a fresh status rolls the conductor back to its previous release
    Given James's conductor is running with usable source chains
    And a compatible elected conductor candidate has an authenticated in-scope delegation
    And fresh authenticated grant status is the last check before the first destructive action against the incumbent
    And no outstanding-lease grace follows a revocation
    And James's peer has received a correctly signed response saying the grant is active, fresh at that last check
    When that fresh status authorizes stopping the incumbent and activating the candidate
    And the provider's revocation, notarized before the recorded stop, reaches James's peer only after the incumbent has stopped
    Then the late revocation triggers local rollback to the retained previous release
    And James's conductor runs the previous release again
    And both the late revocation and the status it superseded are attested through the evidence path

  @wip @requires:household-nodes
  Scenario Outline: The household adopts and recovers through the shared upgrade ceremony
    Given a candidate conductor release is published through an elected release channel
    And its platform, installed-version, database, and protocol requirements have been verified locally
    And its database format permits executable rollback to the previous release
    And the provider, James's peer, and candidate signer have authenticated authority under the exact in-scope delegation
    And grant validity and revocation are resolved by a declared ordering rule that authorizes activation at the stop boundary
    When James fetches and adopts the candidate and encounters "<outcome>"
    Then James's ark restarts only his conductor under the preserved berth and credentials
    And readiness includes the actual running executable's identity
    And the verified running release is "<running release>"
    And any failed readiness attempt triggers local rollback without a conductor authorization round trip
    And adoption or rollback is attested through the existing evidence path after service returns
    And a second complete upgrade run preserves agent identity and usable source chains
    And unsupported downgrade paths are refused before activation

    Examples:
      | outcome          | running release  |
      | successful boot  | candidate        |
      | failed readiness | previous release |

  @wip @requires:household-nodes
  Scenario: Assignment support arrives before any arc is changed
    Given James has adopted an assignment-capable conductor with actuation disabled
    And Matthew still runs a compatible older conductor
    When the household observes proposed assignments and actual synchronized coverage
    Then Matthew can continue his existing valid role
    And Matthew does not count as supporting transitions his installed runtime cannot understand
    And the release channel retains recoverable coverage during the runtime upgrade
    When the controller attempts a transition that requires Matthew to support assignments
    Then actuation is refused because Matthew lacks the required installed capability

  @wip @requires:household-nodes
  Scenario: Downgrading cannot abandon an active partial assignment
    Given James has a verified active partial assignment under an assignment-capable conductor
    When James attempts to downgrade while his partial assignment is still active
    Then the downgrade is refused until sufficient coverage is restored for the older runtime

  @wip @requires:household-nodes
  Scenario: Identical workloads demonstrate useful savings without sacrificing recovery
    Given repeated full-arc baseline runs have measured workload variance
    When the household repeats the identical workload with earned arcs
    And each device holds only its assigned share while the declared recovery floors remain satisfied
    Then replicated operations and validation executions decrease beyond measurement noise
    And network traffic, disk growth, CPU, memory, usable-content latency, synchronization time, and recovery time are compared
    And the device-loss and household-loss reconstruction proofs still pass

  @wip @requires:alpha-cluster-6peer
  Scenario: Alpha adopts an eligible canary before expanding responsibility
    Given alpha has independently verified eligible holders for a non-gold space
    When one canary adopts the assignment-capable conductor release and policy through the elected release ceremony
    Then the canary accepts only its authorized record assignment and verifies its records and dependencies
    And a canary loss exercise reconstructs those records exactly from the remaining admitted holders
    And additional holders keep their existing assignments until both proofs succeed

  @wip @requires:alpha-cluster-6peer
  Scenario: Six independent holders cannot certify the seven-hub gold floor
    Given alpha has exactly six independently evidenced admitted gold holders
    When the collective evaluates whether its gold holding floor is satisfied
    Then the seven-hub gold floor is reported unmet
    And that missing holder is not replaced by a logical household on an existing host

  @wip @requires:seven-independent-hubs
  Scenario: Seven diverse admitted hubs receive the complete gold space
    Given seven admitted hubs have independent diversity evidence meeting the gold policy
    When the governance stewards publish a valid gold record through its elected canonical channel
    Then all seven hubs receive and fully validate the record and its dependencies
    And each hub's verified holdings cover every sector of the gold space
