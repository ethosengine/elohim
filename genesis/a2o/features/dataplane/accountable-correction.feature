# BORN @wip, EVERY STATION. Slice 1's coordinator surface does not exist yet: no
# `amend_content` extern, no `get_content_lineage` extern, no `feedback_application`
# projection, no `feedback_operations` outbox, no `ELOHIM_FEEDBACK_NOTIFY` test switch.
# Today `create_feedback_signal` and `create_vouch` exist but a correction still debits
# standing immediately on filing (the opposite of §7's "zero-only correction leaves the
# subject's aggregate absent"), `update_content` has no author gate at all (§5.3), and
# nothing tracks an operation id (§8). Each scenario below is bound to one numbered
# station in the contract; a station's scenario stays @wip until that station's Rust
# lands, and its own delta line (this feature's habit atom, once one exists) is the
# only thing that flips it. A station is one contract section's worth of guarantee,
# and a couple of stations (1 and 2) carry two closely-related sub-claims — a base
# claim and the failure mode right next to it (discovery, then a late arrival against
# that same discovery; acceleration, then a forged envelope trying to exploit it) —
# rather than a broader claim like "reaches and settles" split needlessly in two.
#
# Contract (read this first; every assertion below cites a section of it):
#   genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md
# Env this feature needs beyond the household mesh: README-accountable-correction.md
# (ELOHIM_FEEDBACK_NOTIFY, and why Jessica/James/Matthew hold the roles they do).
@e2e @dataplane @concern:accountable-correction @requires:multi-node @act:i
Feature: A correction reaches the person it names, only that person can settle it, and the settlement holds
  Jessica wrote something. James, reading it later, believes it is wrong — not a matter
  of taste, a factual correction. He can say so: any peer in their shared household may
  raise a correction against any record in it, because letting only a record's author
  flag a problem with their own record would make every mistake self-policing. But an
  allegation is not a verdict. Filing a correction must never, by itself, cost Jessica
  anything, and only Jessica — the one who wrote the record James is correcting — can
  decide the correction is right and publish the fixed version. Nobody else's say-so
  moves her record, and her say-so is the ONLY thing that can. And once she has settled
  it, that settlement has to actually hold: survive a crash mid-write, stay visible as a
  named disagreement if two of her own fixes collide, and reproduce identically if the
  bookkeeping behind it is ever rebuilt from scratch.

  Vocabulary. A CORRECTION is a witnessed claim (a `FeedbackSignal` of kind `correction`)
  that a named record is wrong; filing one is an ALLEGATION, not a finding. A VOUCH is a
  signed record one peer authors ABOUT another peer's act; its kind names what it is for.
  ACCEPTANCE is one vouch kind — the root author's OWN `accept-correction` vouch naming
  that correction — and it is the one act that turns an allegation into something a reader
  may treat as settled. A SUCCESSOR is
  the amended record the root author then publishes, naming the record it replaces by its
  exact predecessor action, not merely "the current head" (a head can move between the
  correction being filed and being accepted). STANDING is a durable tally kept PER AUTHOR
  (never per record, and never per reader): filing a correction never touches anyone's
  tally; only an ACCEPTED correction changes the record's root author's own tally, exactly
  once. Before any acceptance a subject's standing is UNKNOWN — no row exists yet, which is
  different from a row that exists and reads zero. The ROOT AUTHOR of a record is whoever's
  Create action started its lineage — resolved by walking the record's Update history back
  to that Create, never read off a content-author string or the current head. Two records
  can be about the "same" content and still have different root authors; a correction and
  its acceptance are checked against the ONE predecessor action's own root, not the id in
  general. Every correction and every content record belongs to a shared network space,
  identified by a DNA HASH; a peer only trusts a notification that names its OWN space,
  never a FOREIGN one belonging to some other network entirely. DISCOVERY is a peer's own
  periodic re-scan of every correction that has been linked, on the shared ledger, to a
  record that peer stewards or already holds — it needs no message from anyone, which is
  what makes it durable; a NOTIFICATION is a direct, optional message that lets a peer
  learn sooner than its next scan would have. A peer APPLIES an act when it has fetched and
  verified the signed record and durably recorded, once, what that act contributes — which
  is the step BEFORE, and separate from, the tally change it feeds; that is the gap the
  crash station opens. A peer's PROJECTION is the local read model those records live in:
  derived from the shared ledger, rebuildable from it, and never authoritative over it. The
  SERVED HEAD is the version of a record a peer hands to anyone who asks it for that
  record. CONTESTED marks a record, visibly to any
  reader of it, as having two branches that both claim to follow the same prior version —
  a fact that stays true even after a deterministic tie-break has picked one branch to
  actually serve. A GENERATION is one complete, independently-built copy of the standing
  bookkeeping; its CANONICAL ROWS are the tallies it produces from every RETAINED
  (still-available) correction and acceptance, and two generations built from the same
  retained history must agree on them exactly. Standing's tally counts AGAINST the root
  author: an accepted correction is a mark that something of theirs needed fixing, never a
  reward. An OPERATION ID is a value the filer's own device — their FIRST-PARTY VIEW, the
  one place that both files the correction and later has to explain to its own user what
  happened to it — mints before it ever asks a peer to file anything, so that filing the
  SAME correction twice — because the first answer never arrived — is still recognized as
  one attempt, not two.

  Roles held for every station below (see README-accountable-correction.md for why):
  Jessica is the root author of the record every station corrects. James files every
  correction and, once, wrongly tries to accept one. Matthew never authors, corrects, or
  accepts anything here — he is the durable-discovery witness, the peer nothing notifies.

  Background:
    Given peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"
    And Jessica has authored a content record visible to all three peers

  # ---------------------------------------------------------------------------------
  # Station 1 — discovery is durable and does not depend on being told (contract §3)
  # ---------------------------------------------------------------------------------
  @wip
  Scenario: Matthew's peer finds James's correction on its own, and a late arrival never double-applies
    # Notifications OFF on James's peer (README's ELOHIM_FEEDBACK_NOTIFY=0): the only way
    # Matthew's peer can learn of the correction is its own periodic re-scan (§3), never
    # the notification message of §4.
    Given James's peer has peer-to-peer feedback notifications disabled
    When James files a correction against Jessica's record with no notification sent
    Then Matthew's peer discovers James's correction within its next discovery scan, unnotified
    # The late arrival. A Holochain action cannot be backdated, so "older" is not staged
    # by timestamp: it is staged by PUBLICATION ORDER — an earlier correction whose
    # TargetToFeedbackSignal link only surfaces after a later correction has already been
    # discovered and applied. §3's fair rotation across scans and §7's exactly-once
    # application together mean order of arrival never becomes order of effect, and
    # nothing is counted twice.
    Given James has filed an earlier correction whose target link is not published until after the later one has been applied
    And Matthew's peer has already applied the first correction it discovered
    When the earlier correction's link surfaces to Matthew's peer on a later scan
    Then Matthew's peer applies the earlier correction exactly once
    And Matthew's peer still shows the first correction applied exactly once, undisturbed by the late arrival

  # ---------------------------------------------------------------------------------
  # Station 2 — notification accelerates discovery, and never substitutes for it (§4)
  # ---------------------------------------------------------------------------------
  @wip
  Scenario: A notification reference speeds Matthew up, but a foreign-DNA envelope is refused rather than trusted
    Given James's peer has peer-to-peer feedback notifications enabled
    When James files a correction against Jessica's record with a notification sent
    Then Matthew's peer applies the correction before its next scheduled discovery scan would otherwise have found it
    # The notification carries a REFERENCE (origin DNA hash, action hash, routing key), never
    # the semantic payload as evidence on its own — Matthew's peer must still fetch and verify
    # the actual signed record (§1) before applying anything the envelope claimed.
    And Matthew's peer's applied correction matches the fetched, verified record James actually authored
    # A notification naming a DIFFERENT origin DNA hash is a mismatch this peer must catch,
    # not trust — accepting it would apply a correction addressed to some other content space.
    When a feedback notification arrives at Matthew's peer naming a foreign origin DNA hash
    Then Matthew's peer rejects the foreign-DNA notification without applying anything from it
    And Matthew's peer's own content cell DNA hash is unchanged by the rejected notification

  # ---------------------------------------------------------------------------------
  # Station 3 — only the root author settles it (§5.2, §5.3, §6)
  # ---------------------------------------------------------------------------------
  @wip
  Scenario: Jessica alone can accept James's correction and publish its successor; James's own attempt is refused
    Given James has filed a correction against Jessica's record
    # Acceptance is Jessica's own vouch — `create_vouch` with vouch_kind accept-correction,
    # naming James's correction action as its target (§5.2). Nobody else's vouch counts as
    # acceptance no matter what integrity admits, because every consumer re-derives the root
    # author from the target's lineage and requires the vouch author to equal it.
    When Jessica accepts James's correction with an accept-correction vouch
    Then Jessica's acceptance vouch is a distinct, visible record from James's correction
    # The successor names its EXACT predecessor action — not "whatever the head is now" —
    # via the coordinator extern `amend_content { predecessor_action_hash, content }` (§5.3).
    When Jessica publishes the amended content naming the corrected record's exact predecessor action
    Then Jessica's successor is a third record, distinct from both the correction and the acceptance
    And all three of Matthew's, Jessica's, and James's peers adopt Jessica's successor as the served head
    And a dependent view reading the record re-renders the successor's content
    # James is not the root author of the record he corrected — his own attempt to accept
    # his OWN correction must be refused, not silently accepted by a permissive integrity rule.
    When James attempts to accept his own correction with an accept-correction vouch
    Then James's acceptance attempt is refused because he is not the record's root author
    And Jessica's earlier acceptance and successor are unaffected by James's refused attempt

  # ---------------------------------------------------------------------------------
  # Station 4 — a crash window never double-applies and never loses the correction (§7)
  # ---------------------------------------------------------------------------------
  Scenario: A storage restart between application and aggregate update rolls back and reapplies exactly once
    Given James has filed a correction against Jessica's record
    And Jessica has accepted James's correction with an accept-correction vouch
    # The bookkeeping row that marks the correction applied and the standing tally it
    # feeds commit TOGETHER, in one transaction (§7); a restart landing between the two
    # must find no half-finished write to resume from — both move together, or neither does.
    When Jessica's peer's storage is restarted after the correction is marked applied but before her standing tally reflects it
    Then Jessica's peer's projection shows neither a correction marked applied with no matching tally change, nor a tally change with no correction marked applied
    When Jessica's peer's discovery resumes after the restart
    Then the accepted correction is applied exactly once to Jessica's standing tally
    # Replaying an already-settled correction (§7) must not change the tally again — settling
    # something twice is still settling it once.
    When the same accepted correction is replayed against Jessica's peer after it was already settled
    Then Jessica's standing tally is unchanged by the replay

  # ---------------------------------------------------------------------------------
  # Station 5 — an allegation costs nobody; only acceptance moves standing (§7)
  # ---------------------------------------------------------------------------------
  @wip
  Scenario: James is never debited for filing, and Jessica's standing is Unknown until her correction is accepted
    # Filing costs the FILER nothing — standing_impact is a proposal, never an effect (§5.1).
    Given James has filed a correction against Jessica's record
    Then James's own standing is unaffected by the correction he filed
    # And it costs the SUBJECT nothing either: an unaccepted allegation must be
    # indistinguishable, to a reader, from no allegation ever having been filed (§7).
    # Read exactly: her tally is what it was before he filed, this correction's own group
    # carries a zero contribution, and it creates no aggregate row of its own — so a
    # subject whose only signal is an unaccepted correction stays Unknown, with no row at
    # all. Standing is kept PER AUTHOR, so Jessica also carries whatever earlier stations
    # settled against her; "unchanged" is the claim that survives every run order, and a
    # per-scenario generation cannot manufacture a cleaner one (a generation is
    # (evaluator, policy)-scoped and its replay covers all retained history).
    And Jessica's standing is exactly what it was before he filed, and the unaccepted correction adds no row and no weight of its own
    When Jessica accepts James's correction with an accept-correction vouch
    # Only NOW does a row exist, reflecting exactly the one accepted contribution — repeated
    # acceptances of the SAME correction still count once (§7).
    Then Jessica's standing now exists and reflects exactly one contribution
    When Jessica accepts James's same correction with a second, redundant accept-correction vouch
    Then Jessica's standing still reflects exactly one contribution

  # ---------------------------------------------------------------------------------
  # Station 6 — same-root conflict is named, not silently resolved away (§6)
  # ---------------------------------------------------------------------------------
  Scenario: Two of Jessica's own updates naming the same predecessor leave a contested fork nobody's pick clears
    Given James has filed a correction against Jessica's record
    And Jessica has accepted James's correction with an accept-correction vouch
    # Two Updates by Jessica (the SAME exact root author) naming the SAME predecessor action —
    # via `get_content_lineage`'s exact-root, exact-predecessor candidate set (§6) — is the
    # slice-1 definition of conflict.
    When Jessica publishes two amended successors that each name the same predecessor action
    Then Jessica's peer marks the record contested, visibly listing both branches
    # The deterministic pick (newest action timestamp, action-hash tiebreak) still resolves
    # a served head for readers who need one — but picking a winner must NOT clear the
    # contested marker; a fork stays visibly named as a fork even once something is being served.
    When the deterministic pick resolves one of Jessica's two branches as the served head
    Then the record is still marked contested after the deterministic pick
    # A SEQUENTIAL amendment — one that names the PRIOR head as its predecessor rather than
    # racing it — is ordinary succession, not conflict, and must never be marked contested.
    When Jessica publishes a further amendment naming her already-resolved served head as its predecessor
    Then this sequential amendment is not marked contested

  # ---------------------------------------------------------------------------------
  # Station 7 — a rebuilt generation matches the live one, row for row (§7)
  # ---------------------------------------------------------------------------------
  @wip
  Scenario: A freshly built generation reproduces the live generation's canonical rows exactly
    Given James has filed a correction against Jessica's record
    And Jessica has accepted James's correction with an accept-correction vouch
    # A fresh generation is an independent, from-scratch replay of every retained correction
    # and acceptance into new standing tallies, built alongside the live one and then swapped
    # in — it must land on the exact same tallies the live generation already holds.
    When a fresh standing generation is built from scratch from every retained correction and acceptance, then published
    Then the fresh generation's tallies are identical to the live generation's, excluding storage layout and operational timestamps
    And readers of the fresh generation see it as still rebuilding until every retained correction and acceptance has replayed, never a partial tally

  # ---------------------------------------------------------------------------------
  # Station 8 — one intended act survives a lost response (§8)
  # ---------------------------------------------------------------------------------
  Scenario: James's lost response and retried resubmission still produce at most one contribution
    # James's own device mints an OPERATION ID before it ever asks a peer to file anything
    # (§8) — the value that lets a retried filing be recognized as the same attempt.
    Given James's first-party view files a correction against Jessica's record under one operation id it minted
    # The commit actually succeeds on James's peer, but the HTTP response never reaches his
    # view — the exact gap the operation id exists to survive.
    When the response to James's filing is lost before his view receives it
    And James's view reloads and retries the filing with the same operation id
    Then James's peer reports the operation as resolved to exactly one correction, or unresolved, but never two
    And the retried filing produces at most one contribution toward Jessica's eventual standing change
    # The same underlying record, read through two different presentations (James's
    # first-party filing view and, say, an ordinary content reader), must show the same
    # subject and the same operation outcome — not two different stories about what happened.
    Then a second presentation reading the same subject agrees with James's view about what was filed
