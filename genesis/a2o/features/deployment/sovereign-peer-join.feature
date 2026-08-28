@e2e @deployment @sovereign-peer @concern:sovereign-peer-join @requires:doorway @requires:local-conductor @act:ii
Feature: A developer's own conductor joins the alpha network as a real peer
  As a developer working in a workspace that runs my own conductor
  I want that conductor to join the same network the alpha fleet runs
  So that I can build and measure against the fleet's real peers and test content
  without borrowing anyone's key or pretending to be someone the fleet hosts —
  a "sovereign peer": a participant that holds its own key and answers for itself

  Terms this story uses (each is defined here so the steps below can be judged
  without any other document):
  - The "fleet" is the set of machines the project operates for its alpha
    network; each runs a conductor. Two of them also run a doorway; this story
    only ever talks to "doorway alpha", the one that offers the bootstrap
    endpoint, because joining is about the network, not about which gateway
    a browser later uses.
  - A "conductor" is the process that holds one agent's key and takes part in
    the shared network (a Holochain DHT). The "workspace agent" is the agent
    whose key the workspace conductor holds — a NEW agent, never a copy of any
    fleet member's key. Two conductors are on the SAME network only if the
    application they installed has the same fingerprint (its "DNA hash"); a
    different fingerprint on the same endpoints is a separate, partitioned
    network that looks joined and is not.
  - A conductor finds the network through two endpoints it is configured with:
    the "bootstrap endpoint" (where it announces itself and downloads the list
    of other agents) and the "signal endpoint" (the relay through which it
    opens direct connections to those agents). Its "peer store" is the list of
    agents it has learned this way; an entry that is only the bootstrap
    endpoint itself proves nothing about discovery.
  - A "doorway" is the web gateway a browser talks to. "Doorway alpha" is the
    fleet's gateway; its "conductor diagnostics" page lists every agent the
    fleet's conductors currently see as live. The "workspace doorway" is the
    developer's OWN gateway, started beside the workspace conductor. It has
    two postures: "secure" (it signs its own login sessions with a secret only
    it holds, and provisions through the same door the fleet uses) and
    "keyless" (no account store, no signing secret; the developer on the same
    machine is simply treated as the conductor's operator). Only the secure
    posture can hold a session at all, so only it appears in this story; the
    keyless posture has nothing to present to the fleet and is out of scope.
  - "Provisioning" is what a doorway does at POST /hc/connect for a logged-in
    session: it creates that person's agent on a conductor it controls and
    hands the browser a token to use it. If doorway alpha provisioned for a
    workspace session, a workspace login would become a fleet-hosted identity.
  - A "fixture human" is one of the fleet's seeded test personas (Terrance,
    Susan, …) whose accounts exist so tests can log in through doorway alpha.
    Logging in as one through the normal hosted login is the sanctioned way to
    reach test content; it is NOT the same as a conductor presenting itself as
    that person on the network, which this story forbids.
  - "elohim-host-landing" is the one content node every environment seeds first
    (the landing page); it is the anchor used below because it always exists.
  - Time bounds: "within 3 minutes" is the ordinary-propagation bound used for
    every network check in this story — three 60-second sync rounds, the
    measured time for a peer to refill from the network. The one longer bound
    ("within 10 minutes") is the fleet's own agent-announcement interval, which
    is slower than propagation and outside the workspace's control.
  - Status vocabulary: a scenario tagged @wip has steps not yet wired to the
    test harness (the join and same-read scenarios were wired 2026-08-28 and
    run on the T3 hybrid rung: `just dev conductor alpha`, then the
    a2o run with ELOHIM_CAP_LOCAL_CONDUCTOR_STATUS=available). Independently
    of that, a scenario title ending
    "(RED — the gap)" is one whose behaviour is KNOWN not to hold today — it is
    written to fail until a named capability lands. Scenarios without that
    marker describe behaviour that has been observed to hold (the join scenario
    was exercised by hand on 2026-08-28) or is expected to hold once wired.
  - "the workspace conductor has joined the alpha network" means: all three
    checks of the join scenario below hold for it. Every scenario that starts
    from that state says so with exactly that phrase.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA" (the address the fleet's doorway is reachable at, read from the environment)
    And the deployed application bundle that doorway "alpha" runs is available to the developer

  Scenario: The workspace conductor joins the alpha network and the fleet sees it
    Given a workspace conductor that has installed the deployed application bundle
    When the developer starts it with its bootstrap endpoint set to doorway "alpha"'s and its signal endpoint set to the fleet's
    Then within 3 minutes the workspace conductor lists every application fingerprint that doorway "alpha" reports, and no other — a leftover install from an earlier run would be a second fingerprint
    And within 3 minutes the workspace conductor's peer store holds at least one fleet agent — proof it discovered a peer, not merely the bootstrap address it was given
    And within 10 minutes doorway "alpha"'s conductor diagnostics list the workspace agent's key as a live agent

  @wip
  Scenario: A locally built bundle with a different fingerprint is refused before it can join
    Given a workspace conductor that has NOT installed any bundle yet
    And a locally built application bundle whose fingerprint differs from the one doorway "alpha" runs
    When the developer starts the workspace conductor pointed at doorway "alpha" without explicitly choosing the local bundle (an override exists for developers who knowingly want a private, partitioned network; that path is outside this story)
    Then the start-up stops before installing the local bundle
    And the developer sees a message naming the deployed bundle as the one to install and the two fingerprints that differ

  @wip
  Scenario: The workspace agent authors a content node that the fleet serves (RED — the gap)
    Given the workspace conductor has joined the alpha network
    When the workspace agent authors a content node under its own key
    Then within 3 minutes the node resolves by its id through doorway "alpha"
    And doorway "alpha" reports the node's author as the workspace agent's key, not any fixture human's

  @wip
  Scenario: A workspace doorway session cannot be provisioned on the fleet
    Given the workspace conductor has joined the alpha network
    And the workspace doorway runs beside it in its secure posture, the only posture this story concerns itself with
    And the developer has logged in to the workspace doorway and holds its session token
    When that token is presented to doorway "alpha" for provisioning
    Then doorway "alpha" refuses it as unauthenticated and provisions nothing, so the workspace login never becomes a fleet-hosted identity
    And the same token still provisions on the workspace conductor when presented to the workspace doorway — the fleet's refusal did not invalidate it

  Scenario: A hosted login on alpha and the workspace peer read the same node the same way
    Given the workspace conductor has joined the alpha network
    And the developer is logged in on doorway "alpha" as fixture human "Jessica" through the normal hosted login
    When the developer reads "elohim-host-landing" through doorway "alpha" as Jessica
    And the workspace agent reads "elohim-host-landing" through the workspace conductor
    Then both reads return the same content hash for the node
    And the workspace agent's read carried no hosted-session token
