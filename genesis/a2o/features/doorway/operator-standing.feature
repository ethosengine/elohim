@e2e @doorway @concern:operator-dashboard-under-standing @requires:doorway
Feature: A doorway's operator surface opens to the people who hold the doorway
  THE STORY. A doorway is a shared facility: it gives people on the web a name,
  a secure connection and a cache in front of the peer network, and it hosts
  people who do not yet run a node of their own. Hosting means the doorway's
  conductors (the processes that run a person's apps on the network when the
  person has no device doing it yet) carry an app for that person. Someone has
  to look after the facility, for example by removing hosted apps nobody owns
  any more, adjusting how many people it can host, and choosing which other
  doorways it federates with (the doorways it knows about and routes to). In
  this network the people who look after a doorway are its holders, and a
  holder's standing is a record on the network itself. That record is a
  commitment to operate this doorway; it names its holder, the doorway it
  covers, and the kinds of work it allows (its capabilities, for example
  "manage hosting" or "manage federation"). A holder's standing on a doorway
  is exactly the authority that commitment gives them there, and nothing
  else. The commitment is issued by the network's steward of record: the
  person who answers for the network the doorway serves. A steward of record
  may issue and revoke such commitments but, like anyone, may act on a
  doorway only through a commitment they themselves hold there. Everyone can
  read a commitment. A network has one steward of record; how they come to
  answer for it, and who decides a revocation, are governance stories of
  their own, given here.

  A commitment covers exactly one doorway, so holding one doorway gives no
  standing on another. A doorway can have more than one holder: its primary
  holder, and deputies whose commitments name fewer capabilities. Each sees
  and does only what their own commitment names.

  Holding a doorway is stewardship of a facility, not sovereignty over the
  people it hosts. A commitment allows the work it names and nothing more.
  Holders look after what the doorway hosts; they never decide which key
  belongs to a person.

  The doorway reads that record through its own peer: the node it runs on
  the network. There is no central database to ask. The peer keeps a local
  copy of the network's records, and a new record (a revocation, say)
  reaches it when the peer next exchanges records with the peers that hold
  it: shortly after it is written, never before. Until then the doorway
  still acts on the old record; the scenarios wait for the record to arrive
  and do not pretend it is instant.

  The operator surface (the doorway's dashboard and settings pages) is a view
  of that record and of the doorway's own state. It is not a second place
  where authority lives. Today it isn't built this way. Any signed-in account
  can open the dashboard, the one check some actions make is a permission
  field in the doorway's private account database, and a few hosting actions
  answer callers who aren't signed in at all. The person harmed is everyone
  the doorway hosts. Whoever reaches those actions can start or stop hosting
  them, and nobody can later show whose standing allowed it.

  These scenarios assert the replacement:
    - The surface opens only for a holder of an active commitment to operate
      THIS doorway, read on every request from the network record the
      doorway's own peer holds.
    - It says which commitment it opened under, so the holder can see what
      they stand on.
    - Every action on it checks that commitment for the capability the action
      needs, and names the commitment it acted under.
    - Revoking the commitment closes the surface on the first request after
      the doorway's peer receives the revocation, because the surface checks
      the record on each request, not only at sign-in.
      Nobody waits for a sign-in token to expire.

  A stuck account is one whose key has no app behind it on any of the
  doorway's conductors: the person can sign in, but nothing on the network
  answers for them. The surface shows the holder such an account as stuck.
  The remedy belongs to the account's owner: they point their account at a
  key that has an app behind it, typically their own node's key, by proving
  they hold that key. That act is a separate story; here the surface only
  addresses the remedy to the owner, and the holder cannot apply it.

  Graduation is a different event. A person "graduates" when they start
  running their own node, so the doorway no longer needs to host them. The
  person graduates; the holder only records that it happened, which
  releases the hosted app the doorway's conductors kept for them
  (capability `graduation.record`). What happens after that release belongs
  to the graduation story; this feature checks only who may record it.

  In the scenarios: the household mesh is a local test network of three
  household peers, each with its own conductor, and three doorways in front
  of them. "Matthew" holds the household's first doorway, "doorway A".
  "James" is a member of the household with an account on the same doorway
  and no commitment to operate it. "Refused" means an explicit authorization
  refusal (401 or 403) that shows no doorway state. Every "refused" step
  checks for exactly that status; a missing route, a crash or an empty page
  fails the scenario, and none of them counts as a pass.

  Capability names in the scenarios (such as `hosted_users.lifecycle` or
  `users.permissions.mutate`) are the literal strings a commitment record
  carries. Each is granted or withheld independently of the others. The
  doorway has more capabilities than these (reading its state, managing its
  cache, and others). Every action on the surface needs exactly one named
  capability and passes through the same single check of that capability
  against the holder's commitment, which is why the few exercised here stand
  for all of them. A deputy never holds a capability the doorway does not
  define.

  These scenarios are written before the system passes them; they describe
  what the surface must come to do, not what it does today.

  The last two scenarios (Act II) run on a different test network, the alpha
  fleet, under their own rule with their own setup. The two networks are
  separate: nothing recorded on one gives standing on the other. The alpha
  fleet has two doorways of its own: "alpha-A", held by Matthew, and
  "alpha-B", held by Adam. Adam is a test persona. A persona's standing is
  real only because a real person stands behind the test network it lives
  in: its steward of record. A doorway checks only the commitment the person
  asking holds, never who issued someone else's, so a steward of record with
  no commitment on a doorway is refused there like anyone else.

  Matthew is the alpha fleet's steward of record. He issued Adam's
  commitment, and on alpha-B he holds only a deputy commitment that leaves
  out one capability: changing an account's permissions
  (`users.permissions.mutate`, which sets what a signed-in account may do on
  that doorway). Being steward of
  record does not fill that gap.

  # Habit `operator-dashboard-under-standing` (doorway/doorway-service/.epr-meta/)
  # stays red until these pass on the household:
  # `just test mesh features/doorway/operator-standing.feature`.

  @act:i
  Rule: On the household, doorway A opens only to the person who holds it

    Background:
      Given doorway "A" on the household mesh
      And "Matthew" holds an active commitment to operate doorway "A"
      And "James" has an account on doorway "A" and holds no commitment to operate it

    Scenario: The holder sees the standing the surface opened under
      When "Matthew" signs in to doorway "A" and opens its operator surface
      Then the surface opens
      And it names doorway "A" as the doorway the commitment covers
      And it names the commitment it opened under and the capabilities that commitment allows
      And it says the commitment was read from the network, with the time it was read

    Scenario: A signed-in member without a commitment is refused, and shown why
      When "James" signs in to doorway "A" and opens its operator surface
      Then the surface is refused
      And the refusal says "James" holds no commitment to operate doorway "A"
      And no account, conductor or hosting state of doorway "A" is shown to "James"

    Scenario Outline: The operator actions refuse anyone who does not hold the doorway
      # Each of these changes whom the doorway hosts or whom it federates with,
      # and each needs the one named capability in its row. It must refuse a
      # caller who isn't signed in and a member with no commitment, and in both
      # cases it must change nothing.
      When <caller> asks doorway "A" to <action>
      Then the request is refused
      And the refusal names the capability "<capability>" the action needs
      And doorway "A" hosts and federates exactly as it did before the request

      Examples:
        | caller                       | action                                          | capability              |
        | a caller who isn't signed in | begin hosting a new person                      | hosted_users.lifecycle  |
        | a caller who isn't signed in | stop hosting a hosted person                    | hosted_users.lifecycle  |
        | a caller who isn't signed in | record a hosted person as graduated             | graduation.record        |
        | a caller who isn't signed in | add a doorway to the doorways it federates with | federation.peers.mutate |
        | "James"                      | begin hosting a new person                      | hosted_users.lifecycle  |
        | "James"                      | stop hosting a hosted person                    | hosted_users.lifecycle  |
        | "James"                      | record a hosted person as graduated             | graduation.record        |
        | "James"                      | add a doorway to the doorways it federates with | federation.peers.mutate |

    Scenario: An action names the commitment it acted under
      # A refresh re-reads the doorway's list of federated doorways and rewrites
      # only the doorway's own cached copy of it, which is why it needs a mutate
      # capability yet changes nothing a person relies on. Every accepted action
      # on the surface answers the same way.
      When "Matthew" refreshes the doorways that doorway "A" federates with, from its operator surface
      Then the refresh is accepted
      And the response names the commitment "Matthew" acted under
      And the response names the capability "federation.peers.mutate" that commitment matched

    Scenario: A revoked commitment closes the surface on the next request
      Given "Matthew" is signed in to doorway "A" with its operator surface open
      When "Matthew"'s commitment to operate doorway "A" is revoked
      And the doorway's peer reports that commitment as revoked when asked for it
      And "Matthew" reloads the operator surface without signing in again
      Then the surface is refused while that sign-in is still valid
      And the refusal says the commitment it last opened under has been revoked

    Scenario: A stuck account is shown to the holder, but standing does not reach the account's key
      Given an account on doorway "A" whose key has no app on any of the doorway's conductors
      And a hosted account on doorway "A" whose app is installed and answering
      When "Matthew" opens the hosting view of doorway "A"'s operator surface
      Then the first account is listed as stuck, with the key it points at
      And the hosted account whose app is answering is not listed as stuck
      And the surface offers "Matthew" no control that changes which key the account points at
      And the stuck account carries a notice that only its owner can resolve it, with no action on it for the holder

  @act:ii
  Rule: On the alpha fleet, standing covers one doorway and only the capabilities it names

    Background:
      Given the alpha fleet, whose steward of record is "Matthew"
      And "Matthew" holds the primary commitment to operate doorway "alpha-A"
      And "Adam" holds the primary commitment to operate doorway "alpha-B", issued by "Matthew"
      And "Matthew" holds a deputy commitment on doorway "alpha-B" without the capability "users.permissions.mutate"

    Scenario: Holding one doorway gives no standing on another
      When "Adam" opens the operator surface of doorway "alpha-B"
      Then the surface opens and names the commitment "Adam" holds
      When "Adam" opens the operator surface of doorway "alpha-A"
      Then the surface is refused
      And the refusal says "Adam" holds no commitment to operate doorway "alpha-A"

    Scenario: A deputy can do only what the deputy commitment names
      When "Matthew" opens the operator surface of doorway "alpha-B"
      Then the surface opens and names the deputy commitment "Matthew" holds
      And it offers "Matthew" no control that changes an account's permissions
      When "Matthew" asks doorway "alpha-B" to change an account's permissions
      Then the request is refused
      And the refusal names the capability "users.permissions.mutate" the action needs
