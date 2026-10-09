# WHY THIS FILE EXISTS, AND WHICH ACT IT IS. Harvested 2026-10-09 from a real incident on the
# deployed fleet. Two peers, adam and eve, had their conductors' datasets consumed by something
# outside the protocol: the host's snapshot tooling counted its snapshots against the same quota
# the conductors' databases live under, so every snapshot run silently shrank the space left for
# the conductor until its database could not be opened at all. The conductors crash-looped for
# hours. Nothing in the protocol noticed, because nothing measured that space, and nothing told
# "the disk is full because the peer is holding a lot" apart from "the disk is full because
# something outside the protocol is eating it". A person found it by reading logs, and a person
# cleared it by hand. The plan written the same day says this is exactly the kind of thing that
# should look, to the network, like an outside problem with a peer's integrity — and should make
# the unaffected peers step in, diagnose it, and hold the load on that peer's behalf.
#
# This file is the harvest of that incident in three parts: the failure, preserved as a
# regression anchor so it can never again happen silently; the capability the unaffected peers
# need, which turns out to be a subtraction of two measurements that already exist; and the
# observability gate that turns an opaque database error on a doorway into a named peer, a named
# band and a number.
#
# It is ACT I: every scenario runs on the household mesh this test run OWNS and may write to,
# because the attack is simulated by writing into a peer's own volume, which no test may do on the
# shared fleet. @requires:owned-substrate marks the scenarios that do that, and they also require
# A2O_ALLOW_DESTRUCTIVE=1 as the operator's separate consent to actually degrade a peer; without it
# their mutating steps answer "skipped". It is BORN RED on purpose and carries no @wip, for the
# reason stated in operator-answers-pain.feature's header: an undefined step is the honest red
# naming a missing capability, and a @wip chapter would measure nothing. The habit that owns this
# file is pain-is-answered.
@e2e @dataplane @concern:pain-is-answered @requires:multi-node @act:i
Feature: An outside process consuming a peer's dataset is detected, diagnosed and carried by the unaffected peers, and resolved when the cause is cleared

  A household is a small set of named peers, each a device in someone's home. On this mesh they
  are matthew, jessica and james. Each peer runs two programs. Its CONDUCTOR holds the signed
  records of the SHARED NETWORK — the distributed record store every conductor in the household
  participates in, where any record a peer published can be fetched from any other peer that
  holds a copy — and the rules for reading them; it keeps those records in a database on the
  device's disk. Its STORAGE process holds the household's BLOBS — the files and data objects the
  records point at — and answers other peers' questions about them; it is a separate program and
  can keep running after the conductor has stopped. A HEAD is a peer's answer to "which version
  of this record is current": a signed pointer to one published revision. A conductor verifies a
  head; a storage process only remembers the last head its conductor verified, which is why a
  storage process can go on answering with heads after its conductor has stopped — and why such
  an answer is stale, not verified.

  A BAND is the share of a device's capacity its steward enrolled for one purpose; the STEWARD is
  the person who enrolled the device and is responsible for it, the same person the Refer below
  names as the HARDWARE STEWARD. The RUNTIME BAND is the share set aside for the conductor's own
  database and working files. Every band has a DECLARED LIMIT, a number of bytes the steward's
  enrollment promised, and the protocol measures three things against it. NAMED BYTES are the
  bytes the peer can account for: every blob its storage process holds has one named reason for
  being held, and the conductor's own database files are walked and sized the same way.
  AVAILABLE BYTES is what the disk under the band says is still free. The UNACCOUNTED REMAINDER
  is what is left when named bytes and available bytes are both subtracted from the limit. On a
  healthy peer that remainder is near zero and stays there. When it is large and growing while
  the named bytes are flat, something the protocol did not do is consuming the band. That is an
  EXTERNAL IMPOSITION: an outside process, a host policy, a snapshot, a log, anything the
  protocol neither wrote nor named.

  A design principle this file depends on, stated here because the regression scenario below
  rests on it: the protocol does not, and should not, enforce the host's disk. It cannot stop an
  outside process from writing under a band, because the device belongs to its steward and the
  protocol has no authority over the host's other programs. What the protocol owns is noticing,
  naming, carrying and asking. So the first scenario, which reproduces the failure on purpose, is
  expected to keep reproducing; if it ever stops, the protocol has begun enforcing a floor on
  the host, and that is a change of authority to be revisited, not a fix to be celebrated.

  A SWEEP is the periodic pass in which one peer asks another which heads it holds and verifies
  each answer with its own conductor. Fetching a copy of a blob works the same way outside a
  sweep: the fetching peer takes the bytes from any peer on the shared network that holds them
  and VERIFIES them with its own conductor against the record's signed head, so a copy is never
  trusted because of who handed it over. A TICK is one pass of a peer's own measurement loop; a
  scenario that says "within one tick" means by the next time that loop runs. The ADOPTION
  CHANNEL is the path by which a peer takes on a new release of the household's software; it is
  asked on every sweep and answers "conductor unavailable" when the peer's conductor cannot be
  reached. A DOORWAY is the web front door a household runs so ordinary browsers can reach it;
  its APP LISTING is the page that asks a conductor which applications it serves. HALF-ALIVE
  describes a peer whose storage process still answers while its conductor cannot verify
  anything; the honest answer for such a peer is UNVERIFIABLE, never a remembered head.

  An APPROACHING state is what a peer writes on its own enrollment promise when a band nears its
  limit: "keep an eye on me". On this mesh it fires when available bytes fall below 15 percent
  of the band's declared limit, or when the unaccounted remainder exceeds 5 percent of the limit
  and has grown for three consecutive ticks, whichever comes first; a tick on the household is
  five minutes. The SEAT is a software agent, not a person: it runs on one peer's device, watches
  the household's enrollment promises for approaching states, and decides what the household
  does about them, within bounds its own enrollment promise sets. It records what it decides as
  a GATE-DECISION, one of allow, block or pending, with its reasoning. On this mesh the seat runs
  on matthew's device. HEADROOM is the space a conductor's database needs merely to open — its
  write-ahead log and shared-memory files — a small, fixed number of bytes the conductor itself
  reports; the incident failed at zero, which is below it.

  The scenarios below are facets of one incident, not a timeline in file order. Detection,
  carrying and the status page all happen WHILE the outside process is still growing, before
  the conductor fails; the regression scenario shows the end state that is reached when nothing
  intervenes, and it comes first only because it is the anchor the rest are measured against. When a remedy lies outside what the protocol can do by
  itself — a disk quota set by the host, a snapshot policy — the seat does not guess: it writes a
  REFER naming the layer that can act, here the hardware steward, and it hands that person the
  diagnosis so their first look is already the answer. Meanwhile the unaffected peers HOLD THE
  LOAD: each makes its own promise to keep a copy of the bytes the struggling peer promised to
  hold, so nothing the household depends on rests on a peer that may be about to fail.

  Operational parameters from the incident, kept here so the numbers survive: the quota under
  each conductor was 20 GiB; the conductor's live use was about 6 GiB; the outside consumption
  grew at about 1.4 GiB per day; the conductors failed when available space reached 0 bytes, with
  the database error "unable to open database file" (sqlite code 14); the doorway that read them
  saw that error on every app listing from 09:21Z to 09:59Z; the adoption channel answered
  "conductor unavailable" for five consecutive sweeps; the conductors restarted nine and ten times
  before a person cleared the volume at about 11:40Z. These inform the runtime band's declared
  limit and the headroom a database needs to open at all, the approaching thresholds stated
  below, and how many ticks of growth count as "growing". On this mesh jessica plays the part
  adam and eve played in the incident.

  One piece of test vocabulary: an ACT names the substrate a scenario is measured on. Act I runs
  against a household mesh the test run owns and may write to; Act II runs against the deployed
  fleet, which it may only read. This file is Act I throughout.

  Background:
    Given the household mesh is running with peers "matthew", "jessica" and "james"
    And each peer's runtime band has a declared limit its enrollment promised
    And each peer measures its named bytes, its available bytes and its unaccounted remainder every tick
    And "matthew" is the household's seat

  @regression @requires:owned-substrate
  Scenario: An outside process consumes jessica's runtime band until her conductor cannot open its database, and she is left half-alive
    # Constraint preserved: a conductor's dataset can be consumed by something the protocol
    # neither wrote nor named, and the failure that follows is total for the conductor while the
    # storage process beside it goes on answering as if nothing were wrong. The design principle
    # in the description says why this must keep reproducing.
    Given "jessica" is the only peer holding some of the bytes the household has promised to keep
    And a process outside the protocol begins writing into the volume under "jessica"'s runtime band
    When that process has consumed everything but less than the headroom her conductor's database needs to open
    Then "jessica"'s conductor fails to open its database with the error "unable to open database file"
    And the doorway reading "jessica"'s conductor reports that error on its app listing
    And "jessica"'s adoption channel answers "conductor unavailable" on five consecutive sweeps
    # Five is the count observed in the incident; no protocol threshold depends on it.
    And "jessica" is half-alive: her storage process is still running and still answers "matthew"'s sweep
    # What her storage answers WITH is not asserted here on purpose: that is the gap the
    # "unverifiable" scenario below fills, and it will change while everything above persists.

  @requires:owned-substrate
  Scenario: The unaffected peers tell an outside consumer from the peer's own growth by the unaccounted remainder
    # The diagnosis is a subtraction of two measurements that already exist: the bytes a peer can
    # name, and the bytes its disk says are free. Only the free-space measurement under the
    # conductor's volume was missing, and it failed open to "all free" when it could not read.
    Given "jessica"'s named bytes have been flat for three ticks
    And a process outside the protocol is writing into the volume under "jessica"'s runtime band
    When "jessica"'s available bytes fall for three consecutive ticks while her named bytes stay flat
    And her unaccounted remainder exceeds 5 percent of her declared limit on the third tick
    Then "jessica"'s unaccounted remainder is greater than zero and grew on each of those ticks
    And "jessica"'s enrollment promise carries an "approaching" state naming the runtime band, the disk, and the remainder as unaccounted
    And "matthew" reads that state within one tick and records a gate-decision whose reasoning names an outside consumer of "jessica"'s runtime band, not "jessica"'s own growth
    And "matthew" records a Refer to the hardware steward carrying the declared limit, the named bytes, the available bytes and the remainder

  @requires:owned-substrate
  Scenario: The unaffected peers hold the load before jessica's conductor fails
    Given "jessica"'s enrollment promise carries an "approaching" state naming an unaccounted remainder
    When "matthew" and "james" read that state
    Then each of "matthew" and "james" makes its own promise to hold a copy of the bytes "jessica" promised to hold
    And every byte "jessica" promised is held by at least two peers other than "jessica" before her available bytes reach zero
    And no byte was taken from "jessica"'s storage process for those copies; each was fetched from another peer on the shared network and verified by the fetching peer's own conductor

  @requires:owned-substrate
  Scenario: When jessica is half-alive, her storage says "unverifiable" instead of answering with remembered heads
    Given "jessica" is half-alive: her conductor cannot open its database and her storage process is still running
    When "matthew" sweeps "jessica" for the heads she holds
    Then "jessica"'s storage process answers "unverifiable" for every head
    And "matthew" adopts nothing from "jessica" in that sweep
    And "matthew"'s record of "jessica" says her conductor is unreachable, not that her bytes are absent

  Scenario: The household reads the diagnosis on its status page instead of finding a database error in a log
    # Observability gate: the diagnostic that made the incident findable by hand must be a thing
    # the household can read without reading logs.
    Given "jessica"'s conductor has failed to open its database
    When a person opens the household's status page without reading any log
    Then the page names "jessica" as the peer whose conductor cannot open its database
    And the page names the runtime band and shows its declared limit, named bytes, available bytes and unaccounted remainder
    And the doorway's app-listing failure for "jessica" links to that same reading rather than showing the database error alone

  @requires:owned-substrate
  Scenario: Clearing the imposition closes the loop without a person deciding anything new
    Given the outside process consuming "jessica"'s runtime band has been stopped and its bytes removed
    When "jessica"'s conductor opens its database again
    Then "jessica"'s enrollment promise carries a "recovered" state within one tick
    And the gate-decision "matthew" recorded is marked resolved citing that state
    And the hardware steward is informed that the Refer is closed
    And the copies "matthew" and "james" hold remain until "jessica" herself releases her own promise
