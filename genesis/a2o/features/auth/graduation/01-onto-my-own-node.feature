@e2e @auth @requires:doorway @act:i @graduation @concern:graduates-onto-own-key
Feature: Graduating onto my own node's key — I prove which key is mine, and the doorway follows
  As a person who started out hosted at a doorway and now runs a node of my own
  I want to move my doorway account onto the key my node holds, from my own account page
  So that the account is anchored to a key I keep, and nobody else can move it for me

  A HOSTED human is one whose doorway keeps their identity for them: it holds their password,
  mints their session, and runs a CELL for them — the home of their own record chain — on one of
  the CONDUCTORS it operates (a conductor is the network runtime that runs cells; the ones a
  doorway runs for other people are its POOL). That cell answers to a KEY the doorway made for them.
  Their ACCOUNT at the doorway is the row that says which key is theirs. A NODE is a machine the
  person runs themselves: its own conductor, holding its own key. GRADUATING onto that node is
  re-pointing the doorway account from the key the doorway made to the key the node holds. The
  person's account settings page shows an AGENCY PIPELINE, a row of labelled steps: the stages of how they take part, in order
  Visitor, Hosted, App Steward, Node Steward, with the one they are at marked current. App Steward
  is a person whose key lives in an app on one device of theirs that is not always on; Node Steward
  is a person whose key lives on a machine of theirs that stays on and serves the network.
  The pipeline always shows all four steps. A person who already runs a node goes straight from
  "Hosted" to "Node Steward" and never occupies "App Steward", so this story checks only those two
  marks. The pipeline only shows where the person is; the move itself is offered below it, as its
  own choice.

  Whose act this is, is the point of the story. Which key is a person's is that person's word,
  and the only proof of it is a signature from that key. So the move is three requests, each
  answered by a different party: the doorway issues a one-time CHALLENGE, but only after the
  person types their doorway password again; their node signs that challenge, but only for the
  person signed in to the node; and the doorway moves the account only when that signature
  checks against the key it names. No single credential is enough. An operator who can edit the
  doorway's records cannot sign as the node; something running on the node's machine without the
  doorway password cannot ask for a challenge. When the move succeeds, the doorway keeps the key it
  made as the account's PREVIOUS KEY, so its own record still says where the account came from,
  and, as part of the same move, stops hosting the old cell on its pool.

  Two of those steps happen on two different web pages, because each party only trusts what it
  sees itself. The password is typed on the DOORWAY PORTAL, the doorway's own sign-in and account
  pages, the only place a hosted person's password is ever entered. The signature is approved on
  the NODE PORTAL, the sign-in and settings pages the person's node serves. A node session counts
  only when it is BOUND to a DEVICE KEY: a key the person's browser made when they signed in to the
  node, kept in that browser, which proves each request came from the browser that signed in. A
  session without one (a copied session cookie, say) can read the node but cannot make it sign.
  The device key is not the node's key; it only says "this browser is the signed-in person". The
  doorway portal hands the challenge to the node portal and the node portal hands the signature
  back, by sending the browser from one site to the other and back again, which is why both
  sign-ins happen in the same browser.

  The doorway checks a signature, not a machine: it accepts any key that signs its challenge, and
  cannot tell a node's key from any other. What makes it the person's node is that only the person
  signed in to that node can make it sign. That is also why the stations below that test only the
  doorway's check can use a fresh signing key the story makes, instead of a whole node.

  The node used here is a fresh machine the story adds for the newcomer. The test runs on a
  HOUSEHOLD: a small network of a few machines a developer runs together, one of which hosts the
  doorway. The story adds one more machine to it as the newcomer's own node, and takes it away
  again at the end along with the newcomer's account, so it leaves the household as it found it.
  The finish line is checked from three sides, and the steps say which: what the person's pages
  show; what the doorway's account record says when the story reads it from the doorway itself,
  outside the browser ("asked directly"); and what its pool conductors hold.

  What this story does NOT claim. Moving the account does not tell the network that the old key
  and the new key are the same person. Anything the old key wrote stays unlinked from them until
  a key rotation is witnessed on the network; that is the last scenario, and it is not built yet.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a newcomer registered at doorway "alpha" as a hosted human with the display name "Graduate"
    And the doorway holds a cell for that human on one of its pool conductors

  # ---------------------------------------------------------------------------
  # The finish line: a hosted life, a node of one's own, the move, from the account page.
  # ---------------------------------------------------------------------------

  @browser
  Scenario: A hosted newcomer who now runs a node moves their account onto the node's key
    Given the newcomer's own node has joined the household
    And the newcomer is signed in to their node's portal in the browser
    And the newcomer is signed in to the doorway portal in the same browser

    When the newcomer opens their account settings
    Then the agency pipeline marks "Hosted" as the current step
    And the account settings offer to move the doorway account onto the newcomer's node

    When the newcomer chooses to move the account onto their node
    Then the doorway portal's account page asks for their node's address and their doorway password

    When the newcomer gives their node's address and their doorway password
    Then the browser is sent to the newcomer's node portal, carrying the doorway's challenge
    And the node portal shows which doorway and which account are asking, and asks the newcomer to approve
    When the newcomer approves on the node portal
    Then the browser is sent back to the doorway portal's account page, carrying the node's signature
    And the doorway portal's account page says the account now answers to the newcomer's node's key

    When the newcomer opens their account settings again
    Then the agency pipeline marks "Node Steward" as the current step

    # From the doorway's own account record, read outside the browser:
    Then asked directly, the doorway names the newcomer's node's key as the account's key
    And asked directly, the doorway keeps the key the doorway made as the account's previous key

    # From the pool conductors:
    Then the pool conductors no longer hold the newcomer's old hosted cell
    When the newcomer signs in at the doorway again
    Then no pool conductor starts a cell for them

  # ---------------------------------------------------------------------------
  # Stations. Each is a claim the finish line rests on, checked alone so a red names the
  # station that broke. Where a station needs a signature from "a key the newcomer holds" but
  # not a whole node, the story makes a fresh signing key of its own; the doorway's check is
  # the same either way.
  # ---------------------------------------------------------------------------

  Scenario: The doorway issues a challenge only to the person who types the password again
    When the newcomer asks the doorway for a key-move challenge with a wrong password
    Then the doorway refuses with "WRONG_PASSWORD"
    And the doorway still names the key it made as the account's key

    When the newcomer asks the doorway for a key-move challenge with their password
    Then the doorway answers with a challenge naming this doorway, the account and its current key
    And the challenge expires within five minutes

  Scenario: The node signs a challenge only for the person signed in to it
    Given the newcomer's own node has joined the household
    And the newcomer holds a key-move challenge from the doorway
    When the challenge is sent to the newcomer's node with no node session
    Then the node refuses with "NO_SESSION"
    When the challenge is sent to the newcomer's node with a session copied to another browser, which has no device key
    Then the node refuses with "SESSION_NOT_BOUND"

  Scenario: The node signs nothing but a key-move challenge
    Given the newcomer's own node has joined the household
    And the newcomer is signed in to their node
    When the newcomer asks their node to sign the text "pay 100 to anyone"
    Then the node refuses with "NOT_A_BIND_CHALLENGE"

  Scenario: A signature from any other key does not move the account
    Given the newcomer holds a key-move challenge from the doorway
    When the newcomer answers it naming one fresh key but signed by another
    Then the doorway refuses with "SIGNATURE_INVALID"
    And the doorway still names the key it made as the account's key

  Scenario: A challenge moves an account once
    Given the newcomer holds a key-move challenge from the doorway
    When the newcomer answers it with a fresh key they hold, signed by that key
    Then the doorway names that fresh key as the account's key
    When the same answer is sent to the doorway again
    Then the doorway refuses with "CHALLENGE_UNKNOWN"

  Scenario: An operator cannot move someone else's account
    Given the newcomer holds a key-move challenge from the doorway
    When the doorway's operator answers that challenge signed in as themselves, with a key they hold
    Then the doorway refuses with "CHALLENGE_NOT_YOURS"
    And the doorway still names the key it made as the account's key

  # Designed in outline (a witnessed key rotation on the person's identity), not yet built.
  @wip
  Scenario: The network witnesses that the old key and the node's key are the same person
    Given the newcomer has moved their doorway account onto their node's key
    Then the newcomer's identity on the network records a key rotation from the key the doorway made to the node's key
    And what the old key wrote is reachable from the newcomer's identity
