# @concern:reach-by-key joins this story to the habit that will hold it; no habit names it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three nodes are running. @requires:device-node: a further
# node is staged as Matthew's laptop. @requires:owned-substrate: the test moves that laptop onto a
# separate network, stops the relay and restarts nodes, so it runs only on a mesh this run owns.
# Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md (§5.2, §15, §17).
@e2e @dataplane @wip @concern:reach-by-key @requires:household-nodes @requires:device-node @requires:owned-substrate @act:i
Feature: Two of a household's machines on different networks meet through a relay, and a machine that reaches nobody says so

  Matthew, Jessica and James share a household, and each has a node of their own: a machine that
  keeps its own copy of what the household shares. On each node runs a conductor, the process that
  signs and records what its person does and exchanges those records with the other nodes'
  conductors. Two nodes are connected when their conductors are. The nodes at home are named after
  their people: "matthew", "jessica" and "james". Matthew also has a laptop, node "workspace", which
  he takes out of the house.

  Each node is known to the others by its key, not by where it is on the internet. Whichever way two
  nodes reach each other, each proves to the other that it holds its key, and each checks the other's
  proof. The proof is a handshake at the start of every connection: each node signs, with its key,
  something fresh the other has just sent it, and the other checks that signature against the key it
  knows for that node. Each conductor records when it checked a node's proof and when it first
  accepted anything from that node, so the order of the two can be read afterwards. So a node can
  move from the home network to a café's and still be the same node to the others. That is what
  reaching by key means here.

  Two machines on different networks often cannot reach each other directly: a home router lets
  nothing in that the home did not ask for, and a café's network is usually the same. A relay is a
  machine both of them can reach that passes their traffic between them. A relay is its own program,
  separate from the conductor: the household's relay runs on James's machine beside node "james"'s
  conductor. Stopping the relay leaves node "james"'s own conductor connected to the nodes at home;
  it cuts off only the nodes that reach the household through the relay. The traffic is sealed
  between the two nodes: encrypted with keys only those two nodes hold, so the relay cannot read it.
  The relay keeps no copy of what it passes. What it does learn is which nodes talked to each other,
  and when.

  A node relays for others only when its person offers it. An offer is something the person does,
  not a setting left on by default: James chose, in his node's settings, to relay for the household,
  and his node records that choice as an offer with his name on it, which every node in the household
  can read and which he can withdraw the same way. That is why the household's relay runs on node
  "james". The offer is what tells the other nodes where the relay is: a node that reads it knows to
  reach the relay at node "james", and its conductor connects through the relay on its own whenever it
  cannot reach the household's nodes directly. No one has to connect it by hand.

  The offer and the relay program are two separate things. The offer is James's willingness to relay;
  the relay program running is the machinery that does it. Either can change without the other: while
  the offer is withdrawn, a relay program that is still running refuses to pass anyone's traffic; and
  if the relay program stops, James's offer still stands, so the relay is simply out of service until
  it starts again.

  Two surfaces tell a person about connections. Each node has its own connection status, shown on that
  node to its person: which of the household's nodes it reaches, whether directly or through a relay,
  and, for a relay, on which node it runs and who offered it. The household's status report is a
  separate surface, the summary Matthew reads to check on the whole household: one line per node,
  saying whether it is running, whether it is ready, and which other nodes its conductor is connected
  to. A node whose conductor has started but is connected to no other conductor can do nothing with
  anyone. It must say so in its own connection status and in the household's status report, and never
  be reported as ready.

  The scenarios below are the specification, and two of the behaviours they assert do not exist yet.
  The first is the recorded offer that says who runs a relay for a household. Every scenario starts
  from James's offer, and the first scenario tests the offer itself, so until it exists none of them
  can pass. The second is a check that reports a node connected to nobody as not connected; the
  scenario where the relay is gone asserts that check, so it cannot pass until the check exists.
  Today a household whose relay is missing or wrong starts with every node reporting ready while
  connected to no one. The story is marked work in progress until both exist.

  Background:
    Given the household's nodes "matthew", "jessica" and "james" are running at home
    And node "workspace", Matthew's laptop and a second node of his besides node "matthew", is also running at home on the same network
    And James has offered, in node "james"'s settings, to relay for the household
    And node "james" runs the household's relay

  Scenario: James's offer to relay is recorded with his name, every node can read it, and he can withdraw it
    When James withdraws, in node "james"'s settings, his offer to relay for the household
    Then within 30 seconds nodes "matthew", "jessica" and "workspace" each read that James has no offer to relay
    And the household's relay is still running on node "james"
    And a node that asks node "james" to pass its traffic to another node of the household is refused
    When James offers again, in node "james"'s settings, to relay for the household
    Then within 30 seconds nodes "matthew", "jessica" and "workspace" each read an offer to relay from James, naming node "james"
    And a node that asks node "james" to pass its traffic to another node of the household is served

  Scenario: Matthew's laptop at a café and his node at home meet through the relay
    Given node "workspace" is on a different network from the household's nodes at home
    And neither network lets a connection in from the other
    When Matthew writes a note "Call the plumber" on node "workspace"
    Then within 30 seconds node "matthew" holds the note "Call the plumber", written by Matthew
    And nodes "workspace" and "matthew" reached each other through the household's relay
    And node "matthew"'s conductor recorded checking node "workspace"'s proof of its key before it first accepted anything from node "workspace"
    And node "workspace"'s conductor recorded checking node "matthew"'s proof of its key before it first accepted anything from node "matthew"

  Scenario: The relay passes traffic along, cannot read it, and keeps none of it
    Given node "workspace" is on a different network from the household's nodes at home
    And the traffic passing through the household's relay is being captured
    When Matthew writes a note "Call the plumber" on node "workspace"
    And node "matthew" receives the note through the relay
    Then the captured traffic between the two nodes is encrypted, and the text "Call the plumber" appears nowhere in it
    And a search of the relay's own storage finds no copy of the note
    And the relay's log of that exchange names only nodes "workspace" and "matthew" and the times they talked

  Scenario: Matthew's laptop shows which relay it relies on and who runs it
    Given node "workspace" is on a different network from the household's nodes at home
    And node "workspace" reaches the household only through the relay
    When Matthew opens node "workspace"'s connection status on the laptop
    Then it shows that node "workspace" reaches the household through the household's relay
    And it shows that the relay runs on node "james", offered by James

  Scenario: With the relay gone, the laptop says it reaches no one, and is not reported as ready
    Given node "workspace" is on a different network from the household's nodes at home
    And the household's relay has stopped, while node "james"'s conductor keeps running and James's offer to relay still stands
    When node "workspace" is restarted
    Then node "workspace"'s conductor starts but connects to no other conductor
    And node "workspace"'s connection status tells Matthew that it reaches none of the household's nodes, naming the relay it tried
    And the household's status report shows node "workspace" as connected to no one, not as ready
    When the household's relay starts again
    Then within 60 seconds, allowing the laptop time to find the relay again, node "workspace" reaches node "matthew" through the relay
    And the household's status report shows node "workspace" as connected

  Scenario: Back home on the same network, the laptop talks to the household directly
    Given node "workspace" reached the household through the relay while it was away
    When node "workspace" joins the home network
    Then within 60 seconds, allowing the laptop time to notice it is home, node "workspace" talks to node "matthew" directly
    And the relay no longer passes any traffic between them
