@e2e @resilience @requires:household-nodes @wip @regression @concern:blob-durability @dataplane @act:i
Feature: The household-diversity dataplane reads the household projection
  As a household whose content must survive losing any single home,
  I want salvage and ingest placement to actually know which peers live in
  which household by reading the household_id the substrate already records,
  So that replicas spread across DISTINCT households instead of piling onto
  one, and a household going dark never takes the only copies with it.

  # Substrate-coherence guard for the humans-projection scope reconciliation
  # (2026-06-27-humans-projection-scope-reconciliation-plan). Production writes
  # humans under h_app_id="imagodei" (the identity pillar); the placement readers
  # used to filter under the content scope "lamad" and so enriched ZERO households,
  # silently degrading diversity placement to pure XOR. The fix routes every humans
  # reader through the canonical HUMANS_HAPP_ID. This scenario pins the felt outcome
  # (the candidate set carries real households) at the household floor.
  #
  # HONEST CEILING: this guards the SCOPE leg only. Full production efficacy also
  # needs (a) populated humans.agent_pub_key (per-pod registration, the humans-replayer
  # arc) and (b) agent_cid-namespace candidate ids (SELF_CID, the transport-id resolver).
  # The live cross-peer "replica count rises across households" proof is held to the
  # mesh (it needs @requires:shem); this scenario asserts the household-floor property
  # the salvage unit fixtures already prove.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  Scenario: Salvage candidates carry real households once humans are imagodei-populated
    Given a household "dowell" whose members have humans rows with populated agent keys under the identity scope
    And a content blob that is under-replicated
    When the node builds its salvage candidate pool
    Then each candidate that maps to a known household carries that household_id
    And the diversity placement strategy can span distinct households

  # Source: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
  Rule: While one household gains a hub, a piece short of holders gains one, and no handover leaves a piece with a single copy

    The scenario above belongs to the across-households layer this Feature declares. The four
    scenarios under this rule prove the layer beneath it, inside a single household, and tell
    one story: a household that gains a hub. They do not by themselves prove that replicas
    spread across distinct households; they prove that within the household a piece held by only
    one machine gains a second holder, and that once a piece has two holders no handover leaves it
    with one. All four wait on the placement rules
    every peer runs, and on a hub existing to add.

    @wip
    Scenario: Adding a hub changes nothing in the household until its person offers it
      The salvage scenario above supplies the household labels that placement across households
      needs; this story spreads copies across the machines inside one household, so that once a piece
      has two holders, losing any one machine never takes the only copy.
      In the dowell household matthew, jessica and james each run a peer: a program on their own
      machine that holds their data and serves their apps. A hub is an always-on machine a household
      adds to hold copies and serve its peers, so that people's own laptops need not. The household's
      apps say what they need, never which machine meets it, so adding a hub changes no app. A machine
      that could hold copies holds them only once its person has offered it; being able to is not
      the same as having agreed to. It reads the household through the two endpoints the Background
      names: the doorway, the web entrance through which a browser opens the household's apps on its
      peers, and elohim-storage, the storage service each peer runs, which keeps that peer's copies
      and records which machine holds which copy. From them it reads which machine holds which copy,
      and which apps open. This scenario is not built yet: it waits on the placement rules every peer runs, and on a hub existing to add.
      Given the household "dowell", whose peers are run by matthew, jessica and james, serves its apps and holds its content
      And every household app opens on matthew's, jessica's and james's peers, each peer running the release it already has
      When matthew adds an always-on machine to the household as its hub, without offering it to hold copies for anyone
      Then the hub holds no copy for anyone while matthew has not offered it
      And every household app still opens on matthew's, jessica's and james's peers, from the release those peers were already running before the hub was added

    @wip
    Scenario: Offering the hub to hold copies changes no app and no setting
      Offering is matthew's own act: it is how the hub he added comes to hold copies for the household.
      Because the household's apps say what they need and never which machine meets it, the offer
      reaches no app. It reads the household through the two endpoints the Background names: the
      doorway, the web entrance through which a browser opens the household's apps on its peers, and
      elohim-storage, the storage service each peer runs, which keeps that peer's copies and records
      which machine holds which copy. From them it reads which machine holds which copy, and which
      apps open. This scenario is not built yet: it
      waits on the placement rules every peer runs, and on a hub existing to add.
      Given the household "dowell" has a hub that matthew added and has not yet offered to hold copies
      And every household app opens on matthew's, jessica's and james's peers, each peer running the release it already has
      When matthew offers the hub to hold copies for the household
      Then within 60 seconds the hub is among the machines eligible to hold copies for the household, under an offer signed with matthew's key
      And no app was re-released, and nobody changed a setting in any app
      And every household app still opens on matthew's, jessica's and james's peers, from the release those peers were already running before matthew offered the hub

    @wip
    Scenario: Once offered, the hub takes up what is short of holders, and what is already held stays where it is
      Each piece of the household's content must survive the loss of any one machine, so it needs two
      holders, two machines that each keep a copy. Two is the number of holders the household's peers
      aim for unless they are set to another; these scenarios use that default. A copy counts as held only once the household has
      confirmed the machine keeps it; until then that machine is not one of the two. A piece with fewer
      than two holders is short of holders. A piece that already has its two is not moved just because
      a better machine has arrived: moving it would cost the household work and gain it nothing. It
      reads the household through the two endpoints the Background names: the doorway, the web
      entrance through which a browser reaches the household's peers, and elohim-storage, the storage
      service each peer runs, which keeps that peer's copies and records which machine holds which
      copy. From them it reads which machine holds which copy. This scenario is not built yet: it
      waits on the placement rules every peer runs, and on a hub existing to add.
      Given the household "dowell" has a hub that matthew has offered to hold copies for the household
      And one household photo is held by jessica's peer alone
      And another household photo is held by jessica's and james's peers
      When the household's peers next see the hub's offer
      Then the photo held by jessica's peer alone comes to be held by the hub as well
      And the photo held by jessica's and james's peers stays held by those two, and the hub takes no copy of it
      And every household peer names the same holders for each of the two photos

    @wip
    Scenario: A member who stops holding copies for the others hands them to the hub before letting them go
      jessica's peer runs on her laptop, and she wants it to stop holding copies of other people's
      photos. The hub can take that load. Each piece must survive the loss of any one machine, so it
      needs two holders, two machines that each keep a copy; two is the number of holders the
      household's peers aim for unless they are set to another, and this scenario uses that default.
      A copy counts as held only once the household has confirmed the machine keeps it. No piece may
      drop below its two holders during the handover. It reads the household through the two
      endpoints the Background names: the doorway, the web entrance through which a browser reaches
      the household's peers, and elohim-storage, the storage service each peer runs, which keeps that
      peer's copies and records which machine holds which copy. From them it reads which machine holds
      which copy. This scenario is not built yet: it waits on the placement rules every peer runs, and
      on a hub existing to add.
      Given the household "dowell" has a hub that matthew has offered to hold copies for the household
      And jessica has offered her peer to hold copies for the others, and under that offer jessica's peer and james's peer are the two holders of several photos that james saved, and the hub holds none of them
      And jessica keeps photos of her own on her laptop that she has not offered to hold for anyone
      When jessica withdraws her peer's offer to hold copies for the others
      Then for each of those photos the hub holds a copy, counted as held, before jessica's peer lets its own copy go
      And at no moment during the handover did any of those photos have fewer than two holders
      And the photos jessica kept on her laptop for herself stay on it
      And james was asked nothing and changed nothing
