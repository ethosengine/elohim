# Design: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md
# (§2 the notarization-carrying record; §4 authority + the common language; §5 the bridge;
# §8 the rehearsal each Station below is one link of). Implementation status lives in the
# habit atom elohim/holochain/.epr-meta/happ-lineage-migration.habit.md, not here.
@e2e @delivery @happ-lineage @concern:happ-lineage-migration @requires:household-nodes @act:i @wip
Feature: The network changes the rules its peers hold each other to without losing a single thing it already witnessed

  This story is the Holochain Evolution Epic — the class the upgrade-velocity
  ladder was climbed to reach, and the one every earlier rung was paid off for. Rung 5 taught peers to change
  their own behaviour by election — coordinator code, config — with nobody
  restarted and nobody asked. But the VALIDATION RULES peers hold each other
  to are a different kind of thing: in this substrate those rules ARE the
  network's identity, so changing them means standing up a second network
  and asking everyone to walk across. Until now the only way across was to
  throw the old network away and start clean. The household did exactly that
  once, on 2026-09-03, while it was still small enough to afford it. It will
  never be able to afford it again.

  What this story asks for is narrower and harder than "upgrade": that when
  the rules cross to a new version, EVERY FACT ALREADY WITNESSED crosses with
  them — who said what, when, signed by whom — and the new network can check
  those facts for itself, with the old network's proof in hand, not on
  anyone's say-so. A photo matthew published under the old rules is the same
  photo, notarized at the same moment by the same key, under the new. That
  is what "without wiping out the notarization integrity of the network"
  means, and it is the whole point.

  Who decides a crossing is not the household. The network is a commons and
  its core rules are stewarded by the ELOHIM — the protocol's own agents,
  one running on each person's own hardware as THEIR elohim, holding
  authority only as bounded, revocable, attested commitments; the elohim
  whose earned standing meets the bar at a given reach form the council
  that holds the network's decisions, and a person's own elohim is how they
  are heard by it — so a
  crossing of the shared rules is their decision inside the closed system —
  the network's own governance, which no one outside it can push —
  and every peer's runtime follows it the way it follows any elected head:
  automatically, with no dialog and no veto. What a person keeps is the
  right to look at their own elohim's reasoning and to raise a mishandling
  up the ladder; the revert below is that remedy. What the network keeps is
  diversity: a community may run its own branch of the rules, and the same
  four primitives this story exercises — lineage, witness, a notarized path,
  a bridge map — are the common language in which any reconciliation chain
  between branches is built, so a person can move through the network's
  diversity along a path that was notarized before they walked it.

  The Stations below are the rehearsal of that design on the household's
  smallest shared DNA, the node registry. Its records — which peer holds
  which shard, who attested whose health — are plain notarized facts like
  any other; the mechanism they exercise is the same for a photo, a
  commitment or a credential, because it works on the proof, not the
  content. The photo above is the stake; the node registry is the bench.

  Vocabulary the scenarios lean on, defined before it is used.

  A ROLE is one named slot in the household's application — "node-registry"
  is one — filled by a DNA: the packaged validation rules for that slot,
  whose hash IS the identity of the network that runs them. A CELL is one
  agent's running instance of one DNA: their chain of records under those
  rules, plus their share of the shared store. Each peer runs one
  CONDUCTOR — the process that holds that peer's cells and enforces their
  rules — and one RUNTIME, the reconciliation logic beside it that verifies
  releases, installs, carries and reports, always through its own
  conductor. A peer's PASSPORT is the runtime's published self-description:
  which rule versions it runs per role, which cells, which one it authors
  on, under which key. It is what every other peer and every check below
  reads. A CHANNEL is a named, elected stream of releases a runtime follows;
  "runtime:lineage:node_registry:commons" names the household's shared
  lineage channel for the node-registry role.

  A RULE VERSION is one integrity identity of a DNA — v1 is what the
  household runs today, v2 is v1 plus the knowledge of how to be migrated
  (the witness type below). A peer that has adopted v2 runs BOTH versions
  side by side under its ONE agent key — its old chain and its new chain
  belong to the same person — and is called DUAL-CELLED. Which of the two it
  AUTHORS on and which it only READS from is its declared posture, shown in
  its passport.

  Every fact a peer records is an ENTRY (the content) committed by an ACTION
  (who, when, signed). An ENTRY HASH names the content alone, so the same
  bytes have the same entry hash under any rule version; an action hash
  names one particular act of recording, so it differs per version. A
  WITNESS is a record in v2 that carries a v1 fact's original action and the
  author's signature over it, so v2's own validation re-checks the original
  notarization with no access to v1. SELF-CARRY is the author bringing their
  own records across; HELD-CARRY is a dual-celled peer couriering someone
  else's records across with that someone's signature intact, because that
  someone has not crossed yet. A held record is never confused with a
  native one: the courier is named as a courier. A CARRY RECEIPT is what a
  carry returns — how many records crossed and a DIGEST, a fingerprint of
  them, which must equal the digest v1 itself computes over the same records
  when asked to export them. ATTESTING A CARRY is publishing that receipt
  where the other peers can read it. Each peer's STORAGE PROJECTION is its
  local, queryable copy of what its conductor holds; for a carried record it
  shows the v2 ANCHOR (the new action) beside the NOTARIZED time and author
  from the v1 proof.

  A MIGRATION COMMITMENT is a notarized path: the elohim's recorded, signed
  commitment that a named v1 may be migrated to a named v2 on a named
  release, with a named REVERT HORIZON — the time until which the elohim
  hold themselves free to reverse it. It is held where all the network's
  agreements are held, it is revocable, and revoking it IS the revert. A
  SUNSET COMMITMENT is a second, separate commitment that closes the old
  chains for good. Until the sunset the elohim can reverse the crossing at
  any point; after it, at no point — which is why the sunset is its own
  decision and the last one. NOTARIZING a commitment means it carries enough
  signatures — the QUORUM — from the current COUNCIL ROSTER at the REACH the
  change needs (reach is the scale a decision spans — a household, a
  community, the commons — and a wider change needs a wider roster), under the same CONSTITUTION ROOT the rule version itself
  declares; the roster is the set of elohim whose earned standing meets the
  bar, attested by the roster before it, back to the root. A commitment
  signed by too few, or by keys not on the roster, or under a different
  root, is not a path — every peer's own verification says so, and nobody is
  trusted in between. In this household's rehearsal the bootstrap steward's
  key — a key the household was born with, distinct from any person's own
  agent key — is the declared one-of-one roster, exactly as it stands in
  for every other election the household already runs.

  A LINEAGE RELEASE is a release manifest that installs a rule version and
  names the version it migrates FROM. That naming — the parent version, the
  lineage the new version declares, and the carry recipe that turns a record
  on the old version into one on the new and back — is the release's BRIDGE
  MAP, and it is what makes a release a path rather than a replacement. A
  lineage release is ADMISSIBLE once a peer's own verification has checked
  its bridge map against what that peer actually runs; it is ADOPTABLE only
  when, on top of that, a migration commitment notarizes it — two states,
  two Stations. The bridge map is the RELEASE's declaration; separately,
  the new rule version's DNA carries its own lineage as a property, and it
  is THAT declaration a witness is checked against when facts are carried —
  two checks at two layers, which is why "lineage mismatch" (a release) and
  "lineage unrecognized" (a witness) are different refusals. A
  lineage release is EARNED when its channel's election has promoted it past
  staging — the same election every content head rides, told in full in the
  sibling story runtime-upgrade-propagation.feature and inherited here — and
  RE-ELECTION is that same election moving the channel's
  head back to a prior release when the current one is no longer held.
  CANARY MODE is a peer's declared posture to adopt an earned release first
  and attest how it soaks; james is the household's canary, matthew follows
  once the canary's attestation is read. The MIGRATION WINDOW opens when an
  earned lineage release has a notarized migration commitment naming it,
  and stays open until the sunset. A BRIDGE SWEEP is a dual-celled peer's
  runtime reading its reading cell for facts its authoring cell lacks and
  couriering them across; it runs on the runtime's ordinary reconciliation
  interval — the SWEEP INTERVAL. A single sweep interval pages only part of a
  long chain, so a WALK — one full pass of the sweep across the neighbour's
  whole chain — can span several sweep intervals; a record written mid-walk
  is not seen until the walk that follows it. BACKWARD CARRY is the same courier work in
  the other direction, v2 into v1, possible only when v1 has the witness
  type. RE-AUTHORING is different from either carry: it is an author
  committing the same content again, natively, on a cell they hold — the
  same bytes give the same entry hash, and because it is the author's own
  act it needs no witness type on the receiving side, which is why an
  author can re-author on v1 even when backward carry is unavailable. A
  peer is STALE only when it is still on v1 after the sunset; inside the
  window it is merely not yet across.

  CLOSING a chain is the last action committed on a cell, naming the rule
  version its story continues on; everything before it stays readable
  forever. The close is a SEALING act: the peer's runtime issues it on the
  sunset commitment, under the peer's own key, regardless of the cell's
  declared reading posture — it is the one write a reading cell still takes
  from its own runtime, and the last — as the epic's kernel test measured (elohim/holochain/tests/sweettest, happ_lineage_migration). The substrate itself does not stop a careless or hostile node
  writing after its own close — measured — so the household enforces the
  close where it already checks everything: v2 carries each close as a
  proof and refuses any carried fact that came after it, and each runtime
  routes the role's writes to v2 while the closed cell stays enabled and
  readable. OPENING is the matching
  action on the continuing cell, naming the close it continues from. Both
  cells exist and have been written to long before either happens: the
  close and open are the seal on a crossing already made, not the making of
  it, and they happen only at the sunset.

  A WARRANT is the record a peer publishes when a fact it received fails
  validation — an accusation naming the fact and the cell that authored it.
  A BLOCK is what the substrate does the moment the accusing peer's own
  validation confirms the warrant (no quorum, no vote, no appeal): that peer
  refuses to gossip with the accused CELL — that one chain on that peer, not
  the person's other cells — again, permanently, and nothing in the
  substrate lifts it. Each peer keeps its blocks in a BLOCK LIST, which the
  household can read from that peer's own databases (each entry names the
  blocked cell and the warrant it cites). A write after a close earns both
  from every neighbour that sees it; that is the price of the sunset, and
  the reason a closed chain is never written on again. (Measured on the household 2026-09-05,
  read from the conductors' own databases with crates/hc-dbtool.)

  The TEST HARNESS is the rehearsal's own hand: the a2o runner that drives
  the household mesh. It holds the fixture humans' keys and the bootstrap
  steward's key, because that is how the household is staged, and it can
  join as a fourth peer. When it does something a well-behaved peer never
  would — write on a closed cell, forge a witness, sign below the bar — it
  stands in for a careless or hostile node, and the refusal it earns is the
  protocol's, not a test-only door.

  Each Station below is its own world: its Given sets exactly the state it
  needs, so a later Station may begin where an earlier one would have ended
  without replaying it. The chain they form is the design's, not one run's.

  Background:
    Given the household mesh is running three peers, each on its own conductor, all built from the same conductor software
    And the household's node-registry role runs rule version v1
    # The sunset below is this story's one irreversible act, and it spends the chain it seals. A
    # closed chain must not be authored on again: every neighbour WARRANTS such a write and BLOCKS
    # the writing cell (see the vocabulary). Station 8's own post-close write therefore leaves a real
    # block behind on this household, which is why a story that began on an already-sealed household
    # would poison itself with its first write — any ordinary v1 write on a closed chain IS such a
    # write, which is why the guard tests "closed", the cause, and not "blocked", the consequence —
    # and why this precondition refuses such a household by name before anything is written.
    # Station 8's comment says what it measures and what it does not.
    And no peer's v1 node-registry chain is already closed
    And each peer's runtime follows release channel "runtime:lineage:node_registry:commons"
    And matthew, james and jessica each hold node-registry records they authored on v1
    And no peer has been restarted or re-keyed at any point in this story

  # ── Station 1: the crossing is admissible only when the release names what it migrates from ──
  Scenario: Station 1 — a lineage release names its parent, and only then does verification let it through
    When matthew publishes a lineage release for the node-registry role whose manifest migrates from v1 and installs v2
    Then every peer's runtime verifies that release locally and reports it admissible, naming v1 as the parent its bridge map recognises
    When matthew publishes a second release that installs v2 without naming what it migrates from
    Then every peer's runtime refuses it, each naming "lineage mismatch" as its reason

  # ── Station 2: the path must be notarized before anyone walks it ──
  Scenario: Station 2 — no peer walks a path that has not been notarized first
    Given the lineage release is earned on its channel
    But no migration commitment names it
    When each peer's runtime next reconciles
    Then no peer installs v2, and each names "path not notarized" as its reason
    When the elohim notarize a migration commitment naming that release, v1 as its origin, v2 as its target and a revert horizon
    And each peer's runtime next reconciles
    Then each peer's runtime reads that commitment through its own conductor, not from the release, and reports the release adoptable
    And no peer asked anyone in the household anything

  # ── Station 3: adopt beside, never instead ──
  Scenario: Station 3 — james, the canary, runs v2 beside v1 under the same key with nothing restarted
    Given james's runtime follows the channel in canary mode
    And the lineage release is earned and a migration commitment naming it is notarized, so the window is open
    When james's runtime next reconciles
    Then james's runtime installs v2 as a second installed app beside v1 under james's existing agent key, giving him a second cell for the role
    And james's passport shows the node-registry role with two cells — v1 reading, v2 authoring — and the same agent key on both
    And james's conductor process id is unchanged and his v1 chain is untouched

  # ── Station 4: self-carry keeps the notarization ──
  Scenario: Station 4 — james's own records cross with their original proof, and v2 checks it for itself
    Given james is dual-celled on the node-registry role, authoring on v2
    When james's runtime carries his v1 node-registry records into v2
    Then every carried record exists in v2 with the same entry hash it had in v1
    And each is covered by a witness whose v1 action and signature verify under v2's own validation
    And james's storage projection shows each record's notarized time and author as the v1 ones, with the v2 anchor beside them
    And the carry receipt's count equals james's v1 record count and its digest equals the digest v1 computes when asked to export those records

  # ── Station 5: held-carry keeps the notarization of someone who has not crossed ──
  Scenario: Station 5 — jessica's record is readable in v2 with jessica's signature intact, though jessica never moved
    Given james is dual-celled on the node-registry role, authoring on v2
    And jessica has not adopted the release and keeps running v1 only
    When james's bridge sweep runs
    Then jessica's v1 node-registry record is readable in v2 through a witness that names james as its courier
    And that witness carries jessica's original action and signature, and v2's validation accepts it
    And jessica's own chain has not been written to by anyone

  # ── Station 6: the bridge is honest about its direction ──
  Scenario: Station 6 — the window keeps both sides talking, and reports which way it can carry
    Given james is dual-celled on the node-registry role, authoring on v2
    And the window is open and jessica keeps authoring on v1
    When jessica creates a new node-registry record on v1
    Then james's bridge sweep carries it into v2 within two walks of jessica's chain, held with jessica's signature
    And james's passport reports backward carry as unavailable, because v1 does not carry the witness type
    And no peer reports jessica as stale — she is within the window

  # ── Station 7: revert is free until the sunset ──
  Scenario: Station 7 — before the sunset, the elohim revoke the path and every peer is back on v1 with nothing lost
    Given james has adopted v2 as the canary and self-carried his records, and matthew has followed and done the same after reading james's attestation
    And james has authored a new node-registry record on v2 during the window
    And jessica has not adopted v2
    And jessica has raised through her elohim that a record of hers was held-carried with the wrong courier named
    When the elohim notarize a revocation of the migration commitment, inside its revert horizon, naming jessica's raised concern as its cause
    Then the release is no longer held, and the channel's earned head returns to the prior release by re-election
    And james and matthew mark v1 authoring and v2 reading, disable their v2 cells, and uninstall nothing
    And every record any of them authored on v1 before or during the window is still on v1, untouched
    And james's record authored on v2 during the window is re-authored by james on v1 with the same entry hash, its v2 proof kept in the disabled cell as evidence
    And any v2-authored record not yet re-authored on v1 is reported by its author's passport as pending, never as lost
    And jessica's runtime never noticed anything but a head that moved and moved back

  # ── Station 8: the sunset is a separate notarized act, and it is the only irreversible one ──
  #
  # WHICH CHAIN THIS STATION SPENDS. A close is a sealing act, and the chain it seals is spent:
  # nothing may be written on it again — not a record, not a capability grant — and the write below
  # earns its author a permanent block from every neighbour that sees it. WHICH chain that is is a
  # DECLARATION, not a structural accident: a peer's runtime seals the v1 app the role is BOUND to.
  # A run that stages a predecessor made for the purpose — a run-scoped cell on the same rule
  # version, installed beside the base app under the same key, and bound before the crossing —
  # spends THAT one, and the household's own chain is only ever read. UNBOUND, which is every real
  # peer and every run of this feature that stages no such predecessor, the chain sealed here is the
  # household's own node-registry v1: the Stations that follow may still READ it, and the next run
  # of this feature is refused at the Background above until the household is rebuilt.
  #
  # The harness's post-close write below is the one write this story makes on purpose after a close,
  # and it is the measurement: the author's conductor accepts it (the substrate — the conductor and
  # its DHT — does not fence a closed chain), v2's validation refuses it where the close is known,
  # and james's neighbours warrant it (publish a record accusing the write) and block his v1 cell
  # (refuse to gossip with that chain again, permanently; his v2 cell is untouched). That last
  # consequence is ASSERTED here, read from a neighbour's own block list — which is exactly why the
  # chain this Station spends should be one nobody needs afterwards.
  Scenario: Station 8 — no sunset without its own commitment; with it the old chains close, stay readable, and no revocation reopens them
    Given a fresh migration commitment is notarized and all three peers are dual-celled — v1 reading, v2 authoring — and have attested their carry
    But no sunset commitment exists
    When each peer's runtime next reconciles
    Then no peer closes its v1 chain
    When the elohim notarize a sunset commitment naming the migration
    And each peer's runtime next reconciles
    Then each peer's runtime seals the close on its v1 cell naming v2, then the open on its already-running v2 cell naming that close, in that order
    And each closed v1 chain is still readable by every peer
    And each peer carries its own close into v2 as a proof, so v2 knows where every old chain ended
    And each peer's runtime routes the role's writes to v2 and marks v1 closed; the v1 cell stays enabled and readable, because the base app carries every other role
    And each peer's passport shows the node-registry role with v2 authoring and v1 closed
    When the test harness, holding james's key, writes a fact on james's closed v1 cell and offers it to v2 as a carried proof
    Then the v1 conductor itself accepts that write — the substrate does not fence a closed chain, as the epic's kernel test measured
    But v2's validation refuses the carried proof on every peer that sealed the close or already carries it in its own witness history, naming "after close" as its reason
    # The fence reaches only a peer whose own carry history already holds the close it is checked
    # against; a courier who never saw the close is not yet reached by it. The next line names that
    # plainly as a limitation still being closed, never as the network's intended shape — the fence
    # this Station rehearses grows to cover every courier, not fewer peers on purpose.
    And a courier who never saw the close is not yet fenced — a limitation still being closed, not the network's intended shape
    # The neighbour's own refusal, read out of its conductor's block list rather than inferred from
    # silence. This is the price of the sunset named in the vocabulary above, and the reason the
    # chain this Station spends should be a chain made for the purpose.
    And a neighbour's own block list names james's v1 cell, with the warrant "No more actions are allowed after a chain close"
    When a revocation of the migration commitment is notarized after the sunset
    Then nothing changes: the closed chains stay closed, and each peer's passport still shows the node-registry role with v2 authoring and v1 closed

  # ── Station 9: a forged witness is refused by every peer's own validation — so no one can be handed a fabricated history ──
  Scenario: Station 9 — a forged witness, whoever commits it, is refused by every peer's own validation, naming why
    Given the test harness joins the mesh as a fourth peer running v2
    When the harness commits a witness whose signature does not verify against the action's signer
    Then v2's validation on every peer refuses it, naming "signature invalid" as its reason
    When the harness commits a witness naming a parent rule version the v2 DNA does not declare in its lineage
    Then v2's validation on every peer refuses it, naming "lineage unrecognized" as its reason
    And neither refusal disturbs any record that was carried honestly

  # ── Station 10: a path signed below the bar is not a path ──
  Scenario: Station 10 — a commitment the roster did not hold is refused by every peer's own verification, whatever it claims
    Given the lineage release is earned on its channel
    And the household's declared council roster for the node-registry role is the bootstrap steward's key alone
    When the test harness records a migration commitment naming that release, signed by a key that is not on the roster
    And each peer's runtime next reconciles
    Then no peer installs v2, and each names "quorum unmet" as its reason
    When the harness records a migration commitment naming that release, signed by the steward's key but under a constitution root the v2 DNA does not declare
    And each peer's runtime next reconciles
    Then no peer installs v2, and each names "root mismatch" as its reason
    And the release itself is still earned and still admissible — only the path was refused
