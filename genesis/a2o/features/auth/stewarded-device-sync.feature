@e2e @auth @stewarded-device @concern:stewarded-device-sync @requires:doorway @requires:local-conductor @act:ii
Feature: A second device becomes one of a human's own — and its writes are the human's
  As a human who already exists on the network
  I want a new device I hold (a workspace, a laptop, a phone) recognised as MINE by the
  network's peers, so that what I author on it propagates because peers recognise me,
  not because a web gateway ingested it on my behalf

  THE STORY. Today a content update authored in matthew's workspace reaches the public
  surface only through a pipeline pass and a doorway seed; the p2p dataplane — the
  authoritative plane — receives the bytes second-hand from its own projection. This
  feature asserts the replacement: the workspace joins the network as a peer that is
  one of matthew's DEVICES, the peers recognise its key as acting for matthew within a
  witnessed, revocable grant, and a write made there becomes the served head on every
  doorway with no seed and no pipeline. James's second household device makes the same
  handshake as a stewardee (the canonical story "James and the Spoke"); matthew's
  workspace is the adult instance of the same primitive.

  Terms this story leans on (defined here so the steps can be judged alone):
  - A "device agent" is the NEW key a device's conductor mints for itself when it joins
    (see sovereign-peer-join.feature — never a copy of the human's key). "W" below is the
    workspace's device agent.
  - A human's "identity head" is the declaration naming which keys currently act for
    that human and under what policy — its "controllers". Binding a device means the
    device agent joins the controller set; the policy always names the human's
    steward-set or recovery quorum alongside the human (community-backstopped by
    construction — no key is ever the sole controller of a human).
  - A "transport binding" declares which network addresses (iroh node id, libp2p peer
    id) a device agent operates, so a peer can resolve "who is talking" to "which
    agent" without comparing strings across namespaces.
  - A "grant" is a witnessed, bounded, revocable commitment on the network by a
    corpus's steward naming who may perform which act within what bounds. Every act a
    peer accepts under it names the grant ("bounded_by"), so anyone can check WHICH
    grant it acted under. "adam" stewards the elohim-protocol corpus; "matthew" holds
    a declare-head grant from him over it.
  - "Reach" is the audience tier a write declares. Two matter here: "commons" is the
    open public surface — accepted from any signer today, so what it lacks is recorded
    as a residual rather than refused; "community" is a recognised peer group — a
    write there is accepted only from a signer the peer can resolve to a bound agent.
    Flagged-at-commons / refused-at-community is the gradient this story measures.
  - A "content node" is one addressable piece of content; an "EPR" (elohim protocol
    record) is the signed envelope a content node travels in, named by an identifier
    such as `elohim-protocol-manifesto`. A "corpus" is a named body of content under
    one steward's authority; the elohim-protocol manifesto is a document in adam's
    corpus and is the fixture every declare-head step below writes to.
  - A "declared head" is the version a peer announces as canonical for a piece of
    content; its "served head" is what its doorway delivers.
  - Topology: two doorways serve this network. Doorway "alpha" is backed by storage
    conductor "alpha-A" — matthew's own peer; "elohim.host" is the federation partner
    doorway backed by a different peer. A head reaching BOTH proves the write moved
    through the peer network, not through one gateway.
  - The "stewardship handshake" is the approval flow in which a stewardee's co-stewards
    confirm a new device's binding; the stewardee never holds sole authority to add a
    controller and never sees a recovery secret.
  - "Refused" means an explicit authorization refusal carrying a reason — a missing
    route or a crash is a failure of the scenario, never a pass. "Flagged" means the
    act was accepted AND a witnessed residual names what it lacked.
  - Time bounds: "within 3 minutes" is the ordinary-propagation bound (three 60-second
    sync rounds); "within 10 minutes" is the fleet's agent-announcement interval.
  - "the workspace conductor has joined the alpha network" means the three checks of
    sovereign-peer-join.feature's join scenario hold: the workspace conductor lists
    exactly the application fingerprints doorway "alpha" reports (same network, not a
    partition); its peer store holds at least one fleet agent (it discovered a peer,
    not merely the bootstrap address); and doorway "alpha"'s conductor diagnostics
    list the workspace agent as live. That file is stations 1-2 of one ladder
    (joined, pulled); this file is stations 3-4 (recognised, native sync).
  - Every scenario below stands alone: each Given establishes the whole state it
    needs (binding, transport binding, grant, prior write) through the step
    definitions; none depends on another scenario having run first.
  - Status vocabulary: @wip = steps not yet wired. A title ending "(RED — the gap)" is
    KNOWN not to hold today and is written to fail until a named capability lands — a
    wired RED scenario is a measurement, not a placeholder. The capabilities waited on:
    the identity-head `bind_identity` coordinator (mishpat), a signed AgentPeerBinding
    (imagodei), and the fleet pull leg fetching network-authored ids.

  Design: genesis/docs/superpowers/specs/2026-08-30-workspace-stewarded-device-peer-design.md
  Run locally (household mesh): just test mesh '@concern:stewarded-device-sync and not @wip'

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA" (the address the fleet's doorway is reachable at, read from the environment)
    And the workspace conductor has joined the alpha network
    And the workspace's device agent is "W"

  @wip
  Scenario: matthew binds the workspace's device agent as a controller of his identity head (RED — the gap)
    Given human "matthew" holds an identity head whose controllers name his key and his steward-set
    When matthew declares from his primary device that "W" joins his controllers under the same policy
    Then within 3 minutes the identity head served for "matthew" lists "W" among its controllers
    And the declaration names a policy that includes matthew's steward-set — "W" is never the sole controller
    And the declaration is revocable: it carries a commitment id a later revocation can name

  @wip
  Scenario: the workspace's transport identity is bound to its device agent and the fleet resolves it (RED — the gap)
    Given the workspace conductor advertises an iroh node id and a libp2p peer id
    When "W" signs a transport binding naming both addresses as operated by "W"
    Then within 3 minutes at least one fleet peer holds an identity binding row for "W" whose anchor is on the network
    And that peer resolves the workspace's iroh node id to "W" without comparing raw identity strings

  Scenario: a write signed by a recognised device is accepted at community reach; an unrecognised one is refused (RED — the gap)
    Given a fleet peer that holds an identity binding for "W"
    When "W" authors a content node at reach "community"
    Then the fleet peer accepts the write and names "W" as the signer
    When a device agent with no identity binding authors a content node at reach "community"
    Then the fleet peer refuses the write with a reason naming the missing binding
    And a fleet peer whose binding lookup fails refuses rather than accepts — absence is never read as consent

  @wip
  Scenario: a device write under the corpus steward's grant moves the declared head (RED — the gap)
    Given "adam" holds an active declare-head grant to "matthew" over the elohim-protocol corpus
    And "W" is a controller of matthew's identity head
    When "W" authors a new version of the elohim-protocol manifesto naming that grant as "bounded_by"
    Then alpha-A's declared head for the manifesto is the version "W" authored within 3 minutes
    And the act's attribution resolves to matthew's identity root, not to "W"'s raw key

  Scenario: the device-declared head becomes the served head on both doorways with no seed and no pipeline (RED — the gap)
    Given "adam" holds an active declare-head grant to "matthew" over the elohim-protocol corpus
    And "W" is a controller of matthew's identity head with a transport binding the fleet resolves
    And "W" has declared a new head for the elohim-protocol manifesto under matthew's grant
    Then within 3 minutes the served head for EPR "elohim-protocol-manifesto" matches the declared head on peer "alpha-A"
    And within 3 minutes the served head for EPR "elohim-protocol-manifesto" matches the declared head on peer "elohim.host"
    And no doorway seed and no pipeline build occurred between the declaration and the served-head match

  @wip
  Scenario: a device write with no bounding grant is flagged at commons reach and refused at community reach (RED — the gap)
    Given "W" is a controller of matthew's identity head
    When "W" authors a content node at reach "commons" naming no grant
    Then the write is accepted and a witnessed residual names the missing "bounded_by"
    When "W" authors a content node at reach "community" naming no grant
    Then the write is refused with a reason naming the missing grant

  @wip
  Scenario: adam revokes the grant and the next device write is refused
    Given "W" has authored under matthew's declare-head grant
    When "adam" revokes that grant
    Then within 3 minutes a further declare-head write by "W" under the revoked grant is refused with a reason naming the revocation
    And the previously served head is unchanged — revocation withdraws authority, it does not rewrite history

  @wip
  Scenario: James's second household device makes the same handshake as a stewardee
    Given human "james-son" is a stewardee whose identity head names "matthew" and "jessica" as co-stewards
    And a second household device mints a device agent "J2" for james
    When james opens elohim-app on that device and the stewardship handshake completes through his stewards
    Then within 3 minutes james's identity head lists "J2" among its controllers under the co-steward policy
    And no recovery secret was shown to james at any point
    And what james saves on the second device is visible on his first within 3 minutes
