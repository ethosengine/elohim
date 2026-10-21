# HELD (2026-10-20). Every scenario is @wip: no step below has a definition yet, and the thing it
# checks is not built. There is no design yet for a key held on one machine signing for a cell
# that runs on another. The placement-contract design names it as a missing node ("device key to
# remote cell", §15 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md)
# and places it in §17 slice 10, the runtime redesign. This story is held until that slice lands.
# The same design leaves one question open for the operator (§19, question 3): whether a hub that
# holds the key is an acceptable first stage, with this story as the upgrade from it.
@e2e @auth @remote-signing @act:i @requires:household-nodes @concern:device-key-remote-cell @wip
Feature: Jessica's household hub runs her cell, and only her phone can sign for it
  As Jessica, whose phone is too small to run the network's machinery for itself
  I want the always-on machine at home to do that work for me while my phone keeps my key
  So that nothing is ever signed in my name by a machine I do not hold in my hand

  Jessica lives in the Dowell household with Matthew and their son James. Each of them has an
  identity on the network. A person's IDENTITY ROOT is the lasting name of that identity, and the
  network holds it, not any one device. A DEVICE of hers is ENROLLED under her identity root: the
  network holds a record saying that this device's key acts for her, and any peer can check it.
  Her identity root is published on the network as a record any peer can look up by its address,
  and every enrollment record is linked to the identity root it enrolls a device under, so a peer
  can go from an identity root to every key enrolled under it. Her phone is a device enrolled under
  her identity root, and the key it holds is the one that signs for her.

  Each machine keeps the signing keys it holds in its KEY STORE. The hub described below has a
  key store of its own, holding the hub's own key, which signs for the hub's own identity as a
  machine on the network. Nothing in this story puts a key that signs for Jessica in it.

  The network's machinery for a person is a CELL: the person's own record chain, kept and
  served by a CONDUCTOR, the network runtime that runs cells. A record becomes part of the shared
  network only when other peers check it and accept it; a record that one conductor writes and no
  other peer accepts is not part of the network, whatever that conductor holds. Every peer that
  accepts a record keeps a copy of it and can serve it. That is why each
  scenario below ends on what another peer accepts or refuses. Every record a cell writes is
  signed, and the other peers accept a record only when it is signed by a key enrolled under the
  identity root it claims. To check, a peer looks up that identity root, follows its links to the
  enrollment records under it, and compares the record's signature against the keys they name; a key that no enrollment record
  names is refused, so a record signed with the hub's own key is refused, because only Jessica's
  phone is enrolled under her identity root. A conductor is heavy work for a phone: it must stay running
  to answer the other peers, and it stores everything the cell holds. So Jessica's cell runs on
  the household's HUB, a machine the household leaves on all the time at home.

  REMOTE SIGNING is the arrangement this story proves. The hub runs Jessica's cell: it stores
  her records, answers other peers and does the work. Her phone reaches the hub over the
  household's home network, and everything between them travels along that one connection: what
  the apps on her phone ask the hub to do, and what the hub asks her phone to sign. When the cell
  needs to write something new in her name, the hub sends what is to be signed to her phone: the
  record that will hold the new content. Her phone signs that record with her key, and the hub
  writes the signed record. A record Jessica makes on her
  phone is signed as soon as she makes it; a record that anything else asks the hub to write in her
  name waits until she approves it on her phone. The hub never holds her key, in any form, so it cannot
  sign as her. While her phone is off or out of reach, the hub can still serve what she already
  has, but it cannot write anything new in her name.

  What this story proves is narrower than everything Jessica might want from the arrangement: no
  record is accepted in her name unless her phone signed it. It does not test the connection
  between her phone and the hub itself, whether that connection is protected from others on the
  home network, or whether what her phone is asked to sign is exactly what she meant. Those are
  not part of this story. Where a step says her phone can reach the hub, it means only that the
  two can exchange messages over the home network; it claims nothing about how safe that is.

  The arrangement is not built yet, and there is not yet a design for it: a key held on one
  machine signing for a cell that runs on another. Every scenario is therefore marked as work in
  progress, and none of its steps can run yet. The story is held until the design for remote
  signing exists and is built.

  This is a different arrangement from being HOSTED, where a hub or a doorway runs a person's
  cell and also keeps their key, and can therefore sign as them (the hosted-human series,
  `../hosted-human/`). It is also different from running a node of one's own, where the person's
  own machine runs the cell and holds the key (the graduation series, `../graduation/`). What a
  person is told when they open an app says which of the three they are in.

  The machines in this test. The household is tested on a small cluster of three always-on
  peers. One of them, Matthew's, plays the hub: he has OFFERED it to run cells for the household,
  which means he has published a record on the network saying that his peer will accept and run
  the cells of the other household members. That record is what Jessica's phone finds when she
  chooses a machine to run her cell. An offer is a standing record: any household member can
  choose the offered peer to run their cell without asking the person who offered it again. Every
  offer in this story works the same way: the offered peer runs the chooser's cell, and the
  chooser's own device keeps the only key that signs for them. Withdrawing an offer, and how many cells an
  offered peer will take, are not part of this story. The other two peers stand in for the
  household's other machines. They check what the hub writes, keep a copy of each record they
  accept, and, until the last scenario, run nothing for Jessica; in the steps, "another household
  peer" means one of those two. One of them is James's, which he may also offer for running cells
  in the same way as Matthew's. When Jessica chooses a different machine to run her cell, that
  machine builds her cell from the copies of her records that the household peers already keep;
  her records are not handed over from the hub, and no key moves with them. Jessica's phone is a fourth device the story
  adds, and it runs no conductor of its own. Reaching the hub from outside the home is not part of
  this story.

  Jessica sets up the arrangement herself, once, from her phone: she chooses the household hub to
  run her cell and confirms that her phone keeps signing for her. The first scenario shows that
  choice and what she is told; every scenario after it begins with the choice already made.

  Background:
    Given the Dowell household's three always-on peers are running
    And Matthew has offered his always-on peer as the household hub for running cells
    And Jessica's phone is enrolled under her identity root and holds the key that signs for her
    And Jessica's phone can reach the hub over the household's home network

  Scenario: Jessica chooses the household hub to run her cell, and her phone keeps her key
    Given no machine runs Jessica's cell yet
    When Jessica chooses, on her phone, the household hub to run her cell
    Then her phone asks her to confirm that the hub will run her cell and that only her phone will sign for her
    When Jessica confirms
    Then the hub's conductor starts running Jessica's cell
    And her phone tells her that the household hub runs her cell and that only her phone signs for it
    And the hub's key store holds no key that can sign for Jessica's identity

  Scenario: The hub runs Jessica's cell and holds no key that can sign for her
    Given Jessica has chosen the household hub to run her cell, with her phone signing for it
    When Jessica opens an app on her phone and saves a note
    Then the hub receives the note from her phone and sends her phone the record to be signed, which holds the note
    And her phone signs that record with her key
    And the hub's conductor writes the signed record to Jessica's cell
    And another household peer accepts the record, signed by the key her phone holds
    And the hub's key store holds no key that can sign for Jessica's identity
    And the app tells Jessica that the household hub runs her cell and that only her phone signs for it

  Scenario: While her phone is off, the hub serves what she has and writes nothing new in her name
    Given Jessica has chosen the household hub to run her cell, with her phone signing for it
    And Jessica has saved a note through the hub
    When Jessica's phone is switched off
    Then another household peer can still read Jessica's note from the hub
    When something other than her phone, such as a task running on the hub, asks the hub to write a new record in Jessica's name
    Then the hub reports that the record is waiting for Jessica's phone to sign it
    And no new record in Jessica's name is accepted by any household peer
    When Jessica's phone is switched on again and she approves the waiting record
    Then another household peer accepts it, signed by the key her phone holds

  Scenario: A record the hub signs by itself in Jessica's name is refused by the other peers
    Given Jessica has chosen the household hub to run her cell, with her phone signing for it
    When the hub writes a record in Jessica's name signed with a key of its own
    Then every other household peer refuses the record, naming a signature from a key not enrolled under Jessica's identity root
    And Jessica's cell holds no such record when it is read from another household peer

  Scenario: Moving her cell to another household machine moves no key
    Given Jessica has chosen the household hub to run her cell, with her phone signing for it
    And Jessica has saved a note through the hub
    And James's peer has accepted Jessica's note and keeps a copy of it
    And James has offered his always-on peer for running cells
    And Jessica's phone can reach James's peer over the household's home network
    When Jessica chooses James's peer to run her cell instead of the hub
    Then James's peer's conductor starts running Jessica's cell, built from the copies of her records the household peers keep
    And James's peer serves Jessica's note
    When Jessica saves another note on her phone
    Then James's peer sends her phone the record to be signed, which holds the new note
    And her phone signs that record with her key
    And James's peer's conductor writes the signed record to Jessica's cell
    And a household peer other than James's accepts the record, signed by the key her phone holds
    And neither the hub's key store nor James's peer's key store holds a key that can sign for Jessica's identity
