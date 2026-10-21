# Spec for the design this story waits on, by section:
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
#   §8: adds the "inference" grant scope (what an inference grant may cover)
#   §15: names erasing the counsel context on revoke as a step still missing
#   §17 slice 5: the build slice that delivers the grant scopes, inference among them
@e2e @compute @inference @act:i @requires:household-nodes @concern:counsel-context-erasable @wip
Feature: Jessica lends her elohim's thinking to a household machine, and taking the grant back erases what it held
  As Jessica, whose elohim sometimes thinks on a stronger machine than my phone
  I want to choose that machine myself, and to take back everything it held of mine when I end it
  So that lending my elohim a machine never leaves my thoughts behind on it

  This story is held: every scenario is tagged @wip, no step below has a definition yet, and what
  the steps check is not built. No household machine runs real inference today, there is no
  inference grant, and nothing keeps or erases what an elohim reasons about on another machine.
  Two pieces must be built before it can run. The first is the "inference" grant scope, which
  section 8 of the placement-contract design adds and slice 5 in its section 17 builds along with
  the other grant scopes. The second is erasing the counsel context when a grant is revoked, which
  section 15 of that design names as a step still missing. That design is
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md.
  The tag @concern:counsel-context-erasable is the name the habit register will use to point at
  this story as its proof; no habit checks it yet.

  Jessica, Matthew and their son James make up the Dowell household. Each of them keeps a PEER:
  an always-on machine at home, meaning one that runs continuously and is never switched off,
  which holds their data and does work for them. Matthew's peer is called Matthew's machine below,
  and James's peer is his desktop. Jessica's phone is not a peer: it is a device she carries, and
  it is off or asleep much of the time. Each of them also has an ELOHIM: an AI that works
  alongside its own person and for no one else. It helps them think things through. It is not a
  person, and it does not speak for anyone but its own.

  An elohim's thinking is INFERENCE: running a model over what the elohim has been given.
  Inference is heavy work, and Jessica's phone is slow at it. So her elohim can think on another
  machine in the household instead. While it thinks there, that machine holds Jessica's COUNSEL
  CONTEXT: what her elohim is working with, including the questions she asked, the notes she
  showed it, and its working state. Some of that lives only in the machine's memory while the
  elohim thinks, and some may be written to the machine's disk. The machine that holds it is the
  CARRIER. A carrier learns everything her elohim reasons about while it runs there, so the choice
  of carrier is Jessica's alone. Nothing chooses one for her: no machine is ever used for her
  elohim's thinking because it is able, idle or faster, only because her grant names it.

  An INFERENCE GRANT is Jessica's written permission for one machine to run her elohim's
  inference. It names her, the carrier, and what it covers. It is hers to make and hers to REVOKE.
  The grants in this story carry no end date: each lasts until Jessica revokes it, and revoking it
  is ending it now. A carrier that is revoked must ERASE Jessica's counsel context: keep nothing of
  it, in memory or on disk, and be unable to answer from it afterwards. Revoking the grant takes
  back only what the carrier held. Jessica's own copies, on her phone, are hers and are not touched.

  Every person and every machine in the household has its own signing key, and each machine knows
  the others' public keys. To say something is "signed by Matthew" or "signed by Matthew's machine"
  is to say its signature checks against that one key, so any machine in the household can tell who
  wrote it and that nobody changed it afterwards. Matthew's key and his machine's key are different
  keys, because they vouch for different things. A person signs what that person decides, such as
  lending a machine to others. A machine signs its own account of what it did with what it held,
  such as an erasure, because only the machine can speak for its own memory and disk, and its
  signature makes the machine answerable for that account.

  A machine runs anyone's inference only after its own person OFFERS it. To offer a machine is to
  make an offer for it. An OFFER names the machine, names what it will do for others (here, run
  inference for the household), and is signed by the machine's person with their own key. It is
  kept on every machine in the household, so any of them can read it, and while it stands the
  machine can be chosen for that work. Whether a machine is ABLE to run inference is a fact about
  the machine itself, its hardware and the model installed on it, and the test sets the household
  up so that both Matthew's machine and James's desktop are able. A machine with no offer cannot be
  chosen, however able it is: being able to is not offering. How an offer is made and withdrawn is told in its own story,
  genesis/a2o/features/compute/eligible-is-not-enlisted.feature, which carries the concern tag
  placement-floor. That story offers a machine for hosting live sessions; an offer here
  is the same kind of signed statement, naming inference as the work instead. This story starts
  after Matthew has made his. Matthew has offered his machine for the household's inference, and he
  keeps that offer standing throughout. Once an offer is withdrawn, no new grant can name that
  machine; what happens to a grant already made on it is a case this story does not cover.
  James's desktop could do the work just as well, but James has made no offer for it.

  What this story can check, and what it cannot. No test can look inside another person's machine,
  into its memory or its disk. What it can check is how the carrier behaves afterwards. It refuses
  every further request under the revoked grant. It writes an ERASURE RECORD: a statement, signed
  with the carrier machine's own key, that it erased Jessica's counsel context under that grant.
  Jessica can read the record and check its signature against that key. Every answer a carrier
  sends back is signed with its machine's key in the same way, which is how Jessica's elohim knows
  which machine thought a question through. And when Jessica later grants the carrier again, it
  answers as a machine that holds nothing from before. To make that last check mechanical,
  Jessica's note carries one line, "Ask about the late bus", which the test puts in no other content
  it gives the household. The check is that the carrier says it holds no earlier context, and
  that its reply does not contain that line. That shows only that the carrier did not repeat the
  line; it cannot show the carrier forgot it. A carrier caught answering from erased context is
  caught against its own signed word. Where the story checks that something does not arrive, it
  can only watch for a while, so it waits a fixed five minutes, far longer than one question takes.

  The household is tested on a small cluster of three peers, one each for Matthew, Jessica and
  James. Jessica's phone is a device the story adds to it.

  Background:
    Given the Dowell household's three peers are running
    And Matthew has made an offer for his machine, signed by him, that it will run inference for the household
    And James's desktop can run inference, and James has made no offer for it
    And Jessica's elohim runs on her phone
    And Jessica's phone keeps her note "Questions for the school meeting", which contains the line "Ask about the late bus"
    And the line "Ask about the late bus" appears in no other content the test gives the household

  Scenario: Jessica chooses where her elohim thinks, and only a machine that was offered can be chosen
    When Jessica asks which household machines her elohim could think on
    Then Matthew's machine is on the list she is shown
    And James's desktop is not on the list she is shown
    When Jessica grants Matthew's machine the inference for her elohim
    Then the grant names Jessica as the one granting, Matthew's machine as the carrier, and inference as what it covers
    When Jessica asks her elohim a question about her note "Questions for the school meeting"
    Then her elohim's answer comes back signed by Matthew's machine

  Scenario: Revoking the grant erases her counsel context from the carrier
    Given Jessica has granted Matthew's machine the inference for her elohim
    And her elohim has thought through a note titled "Questions for the school meeting" on Matthew's machine
    When Jessica revokes the grant
    Then Matthew's machine refuses her elohim's next request, naming the revoked grant
    And Matthew's machine writes an erasure record naming the revoked grant
    And Jessica can read that erasure record, and it is signed by Matthew's machine
    And the note "Questions for the school meeting" is still on Jessica's phone

  Scenario: Granting the same machine again starts from nothing
    Given Jessica's elohim thought through the note "Questions for the school meeting" on Matthew's machine under a grant she has since revoked
    When Jessica grants Matthew's machine the inference for her elohim again
    And her elohim asks Matthew's machine about the note it thought through before the revocation, without showing it the note
    Then Matthew's machine replies that it holds no earlier context for that note
    And its reply does not contain the line "Ask about the late bus"

  Scenario: Revoking while her elohim is thinking stops the thinking and erases it too
    Given Jessica has granted Matthew's machine the inference for her elohim
    And her elohim is thinking through a question on Matthew's machine
    When Jessica revokes the grant before the answer comes back
    Then Matthew's machine writes an erasure record naming the revoked grant, which covers the question it was thinking through and says it stopped that thinking before any answer was sent
    And Jessica can read that erasure record, and it is signed by Matthew's machine
    And no answer from that thinking reaches Jessica's elohim within five minutes of her revoking the grant
