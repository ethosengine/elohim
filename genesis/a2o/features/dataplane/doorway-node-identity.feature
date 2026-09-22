# A doorway's own name, read across a restart of that doorway, on a household
# that owns the doorway processes.
#
# Wire detail: steps/dataplane/doorway-node-identity.steps.ts owns the boot-line
# read, the published-key reads, the configured-name read and the restart arm.
# The identity itself is doorway/doorway-service/src/node_identity.rs; what
# publishes it is doorway/doorway-service/src/routes/federation.rs (the key set)
# and routes/identity.rs (the DID document).
#
# Doorway-failover's habit atom owns graduation, and this file is a STATION on
# that habit rather than a second telling of it: the habit promises a person
# reaches the site through the loss of either doorway, which presumes the pair
# still accept each other's signatures — and until 2026-09-19 an ordinary
# restart silently broke exactly that. So this file proves the PREREQUISITE
# (identity survives a restart) and says nothing about a doorway's LOSS. The
# pair's behaviour while one doorway is unavailable is doorway-failover.feature
# and doorway-apex-transition.feature. A pass here cannot discharge either.
@e2e @dataplane @regression @concern:doorway-failover @act:i @requires:owned-substrate
Feature: A doorway is still the same doorway after it restarts
  A STATION, not a story of its own: this file is tagged to the doorway-failover
  concern because it proves the PREREQUISITE that concern's promise rests on —
  two doorways can only cover for each other if each still accepts the other's
  signatures after an ordinary restart. It does not prove failover, and a pass
  here discharges nothing in doorway-failover.feature or
  doorway-apex-transition.feature.

  A doorway is not only an address. It is also a NAME that other parties have
  learned to trust: a sibling doorway that accepts the tokens this doorway
  signs, and a person's browser that was handed a session by it. That trust is
  anchored to a single Ed25519 SIGNING KEY the doorway holds. Nobody outside
  ever sees that key's secret half; what they see is the PUBLIC half the
  doorway publishes, and everything the doorway signs is checkable against it.
  FEDERATION, wherever this file says it, is that relationship and nothing more
  exotic: doorways accepting each other as siblings — each verifying the other's
  signatures and honouring the tokens the other issued — instead of each
  standing alone.

  So a doorway that comes back from an ordinary restart holding a DIFFERENT key
  has quietly become a different doorway wearing the same address. Nothing
  announces it. A sibling that remembers the old key starts rejecting this
  doorway's signatures as forgeries, and it is right to — refusing to swap a
  remembered key for a different one is exactly how impersonation is kept out.
  The cost lands on a person who did nothing wrong: they are signed out, or
  their request is refused across the pair, by a restart they never saw.

  That is not hypothetical. Until 2026-09-19 every doorway boot minted a fresh
  key unconditionally, so every restart rotated the doorway's federation
  identity in silence. The cure is a FILE the doorway keeps its key in, named to
  it at boot: present, it loads the identity it already had; absent, it mints one
  and writes it there for next time. This feature is the standing check on that
  cure. A doorway told no file still mints a throwaway key every boot — an
  EPHEMERAL identity — which is honest but unusable for federation. No scenario
  below exercises that case: there is nothing to keep, so there is no continuity
  to prove. It is instead the first PRECONDITION each scenario checks, and a
  household whose doorway was told no file fails at that step, naming the
  ephemeral identity, rather than passing a continuity check it never performed.

  THE THREE SURFACES this identity is read from, and why it takes all three:

  1. The BOOT LINE. At startup the doorway writes one line naming the file it
     resolved its identity from, a FINGERPRINT — a short, non-secret label for
     the public key, the first eight bytes of it in hex — and a MODE: `loaded`
     (the file was there, this is the identity it already had) or `generated`
     (no file, so it minted a new one). The mode is what tells apart "kept" from
     "silently replaced"; the fingerprint is what says WHICH identity.
  2. The KEY SET, published at `/.well-known/doorway-keys`. This is the document
     any sibling reads to learn this doorway's public key. Each entry carries a
     KEY ID: the name the reader files that key under. This doorway's key id is
     its own configured name, which matters more than it looks — a sibling's
     cache is keyed by it, so two doorways sharing one key id would have one
     entry claimed by two keys, and whichever arrived second would be refused.
  3. The DID DOCUMENT, at `/.well-known/did.json` — the doorway's own W3C
     `did:web` record, naming itself and where it answers identity questions.
     Today it deliberately carries NO key material, so it shows continuity of
     the NAME only; the key's continuity is read from the key set. Both are
     checked here because a restart that kept the key but renamed the doorway
     would break a sibling just as thoroughly.

  The boot line and the key set are checked AGAINST EACH OTHER, not just each
  against its own past. A doorway could log a fingerprint it does not actually
  serve, and a self-report nobody joins to the wire has misled here before: a
  doorway that was refusing every request with a 503 once answered its own
  startup page with every upstream circuit reported healthy. So the fingerprint
  the doorway names at boot must be the key a reader can fetch from it.

  THE HOUSEHOLD is the group of people who run these doorways themselves, on
  hardware they hold. Where a scenario says the household restarts a doorway,
  it is that group acting on its own process: the doorway is stopped and started
  again with the environment it was running with, which is an ordinary
  operational act and the one this feature measures. The two doorways are named
  here as the household's own fixture names them, "alpha-A" and "elohim.host" —
  labels for two machines, not public names a visitor types.

  A RECAST is a different act entirely: the household is fully stopped and its
  state deliberately erased — conductors, storage, doorway accounts and the
  doorway identity files with them — so what comes back is not the same
  household recovered but a new one standing in the old one's place, with
  doorways that are new parties nobody has trusted yet. There is no succession
  and nothing is carried over; that is the point of choosing it. The last
  scenario names that promise and is NOT exercised — see the note above it.

  WHAT IS NOT CLAIMED. These scenarios do not present a token minted before the
  restart to a sibling after it; the reason is given where that scenario says so.
  They do not prove that a sibling has actually cached this doorway's key, that
  a browser session survives, or that a key file is safe against a hostile local
  process. They induce no fault beyond the restart itself, and they say nothing
  about what either doorway serves while the other is unavailable.

  RUNNER PREREQUISITES. Both doorways answer, and this run owns their processes:
  real process identifiers on this host and real log files on this disk, which
  is what `just mesh start` gives and a deployed fleet does not. Against a fleet
  the scenarios are HELD (skipped), never failed, because a restart nobody here
  may perform proves nothing either way — that is what `@requires:owned-substrate`
  says. `@act:i` names the lane that owns its substrate, the household one, as
  opposed to the deployed fleet. `@wip` says something else again: the scenario
  states a promise whose steps are deliberately not wired, so it is skipped
  rather than failed.

  # The shared address-resolution step calls every addressable mesh participant
  # a "peer"; the two named here are doorways, and the scenarios call them that.
  # Each is named twice: a scenario label, then the label the harness resolves to
  # its actual address — from this run's household manifest, or the
  # `E2E_DOORWAY_*` variable the lane exported. One label happens to look like a
  # hostname and the other does not; both are fixture labels, neither is an
  # address a visitor types, and nothing here hard-codes one.
  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: A doorway comes back from its own restart as the same doorway
    # The whole cure in one pass: the doorway says it LOADED rather than minted,
    # it names the identity it actually serves, and a reader who fetched its key
    # before the restart would get the identical document afterwards.
    Given doorway "alpha-A" keeps its identity in a file of its own
    And I record what doorway "alpha-A" publishes as its identity
    When the household restarts doorway "alpha-A"
    Then doorway "alpha-A" says at boot that it loaded the identity it already had
    # Fingerprint = the first eight bytes of the public key in hex: a short,
    # non-secret label for it, which is why a log may safely carry one.
    And the fingerprint doorway "alpha-A" named at boot is the key it publishes
    And doorway "alpha-A" publishes the same key set, byte for byte, under the same key id
    And doorway "alpha-A" names itself the same in its DID document

  Scenario: Restarting one doorway leaves the pair's two identities distinct and untouched
    # The pair's question, not the doorway's: two doorways must be two parties.
    # If both published one key, or one key id, "unchanged" would be satisfied by
    # a constant baked in somewhere and the check would prove nothing. And a
    # restart of one must not disturb the other's identity at all.
    #
    # DEFERRED, with its reason: the sharpest form of this — doorway "alpha-A"
    # signs a token, is restarted, and the token still verifies at doorway
    # "elohim.host" — is not asserted here, because on this household nothing
    # signs with the node key. A token minted here is signed with a secret both
    # doorways already share, so one crossing the pair would prove that secret
    # survived, not that this doorway's identity did. Switching the pair to
    # key-signed tokens mid-scenario would measure differently configured
    # doorways, so the leg waits for a lane that boots them that way; the
    # setting and the code that reads it are named in the step definitions. What
    # IS asserted is the document a sibling reads and the id it files it under —
    # which is the whole of what a sibling has to go on.
    #
    # Every Given below is a precondition this scenario VERIFIES before acting,
    # not a claim about the restart: the pair must already be two parties for
    # the restart's "unchanged" to mean anything at all.
    Given doorway "alpha-A" keeps its identity in a file of its own
    And doorway "elohim.host" keeps its identity in a file of its own
    And I record what doorway "alpha-A" publishes as its identity
    And I record what doorway "elohim.host" publishes as its identity
    And doorways "alpha-A" and "elohim.host" publish different keys under different key ids
    # The key id is meant to BE the doorway's own configured name. A doorway that
    # was configured with none falls back to a shared placeholder, and then two
    # doorways would claim one entry in a sibling's cache — so the step above
    # could pass today and collide the moment a doorway boots unconfigured.
    And each of doorways "alpha-A" and "elohim.host" publishes its key under its own configured name
    When the household restarts doorway "alpha-A"
    # Both doorways are read on ONE surface here, the key set, and deliberately:
    # it is the only surface a sibling actually consults. The restarted doorway's
    # own self-report and its name are checked on all three surfaces in the
    # scenario above; repeating them here would re-prove that scenario rather
    # than this one. The boot line stays in, because "it loaded" is the fact
    # that distinguishes an identity kept from a coincidence.
    Then doorway "alpha-A" says at boot that it loaded the identity it already had
    And doorway "alpha-A" publishes the same key set, byte for byte, under the same key id
    And doorway "elohim.host" publishes the same key set, byte for byte, under the same key id

  @wip
  Scenario: A recast household gives its doorways new identities
    # NOT EXERCISED, AND DELIBERATELY UNWIRED. A recast discards the household's
    # whole state — conductors, storage, doorway accounts — and the doorways that
    # come back are new parties with new names. That is a legitimate thing for the
    # household to choose and the right counterpart to the scenarios above: the
    # promise is that identity persists across a RESTART and only across a
    # restart, so an operator who wants a clean party knows how to get one and an
    # operator who does not cannot get one by accident.
    #
    # It is written as a promise and left without step definitions on purpose.
    # Wiring it would put "erase this household's state" one tag filter away
    # from a routine lane, and the household these scenarios run on is shared.
    # The reading below is what the household performed by hand on 2026-09-20;
    # what is missing is a lane with a household root of its own — a second,
    # disposable root, which the mesh tooling already supports — so that the
    # erasure costs nobody anything. Until that exists, a recast stays an
    # operator's deliberate act.
    Given I record what doorway "alpha-A" publishes as its identity
    When the household is fully stopped and recast, discarding the identities of the household it replaces
    Then doorway "alpha-A" says at boot that it generated a fresh identity
    And doorway "alpha-A" publishes a key that is not the key the retired household published
