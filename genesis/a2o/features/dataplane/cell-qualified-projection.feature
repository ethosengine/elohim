# BORN @wip, EVERY STATION. The qualifier this contract requires does not exist yet: no
# cell-qualifier column on the storage `content` projection row, no qualifier on the
# Automerge `sync` doc, no comparison step in the re-author path that would let a
# RECEIVING peer refuse a clone-authored write into its base cell, no partition-visible
# state for a second-clone or unheld-cell row, no per-partition rebuild, no UNKNOWN
# provenance on an unqualified `store_acquired_record` ingest, no migration default for
# rows written before this contract, and no proof that clone content stays live under its
# own authoring peer once the refusal above exists. Each scenario below is bound to one
# numbered station in the contract; a station's scenario stays @wip until that station's
# Rust lands, and its own delta line (this feature's habit atom, once one exists) is the
# only thing that flips it.
#
# Contract (read this first; every assertion below cites a section of it):
#   genesis/docs/superpowers/specs/2026-09-08-cell-qualified-projection-and-sync-contract.md
# `genesis/a2o/scripts/fixtures-clone.mjs` stays the one-peer preparation and inventory
# step (modes prepare, install, grow, measure, authorize, inventory; single --db, single
# --storage, no diff or assert mode) — this scenario invokes it on each peer to stage
# clones and dump the six inventories, and does the cross-peer comparison itself.
@e2e @dataplane @concern:cell-qualified-projection @act:i @requires:multi-node
Feature: A clone-authored row never escapes into a peer's base cell, on the peer that receives it
  Jessica's household runs a `lamad` clone alongside its base `lamad` role — a private
  practice space her family owns, not the shared corpus everyone else reads. She writes
  into the clone. On her own machine that write never leaves the clone's DHT: every
  source-chain and DHT view she can inspect locally stays clean. But the write still has
  to travel two more ways before it is safe — through her own storage's projection and
  sync rows, which any peer who talks to her, clone or not, will read; and, once another
  peer's storage receives that projection or sync row, through whatever comparison
  decides whether to author it into ITS OWN base `lamad` cell. That second peer, Matthew's,
  is where the actual 2026-09-07 household-mesh escape was caught: two of Jessica's
  clone-only rows landed unqualified in the shared `content` projection, two docs landed
  in the shared `elohim` sync namespace, and eight minutes later Matthew's storage
  re-authored one of those docs into his own base `lamad` cell under a fresh DHT action —
  content that was never his family's to hold moved from unqualified bookkeeping into a
  living record on a machine that never joined the clone. Fixing the four planes upstream
  of that comparison — where the qualifier comes from, whether the row carries it, whether
  the doc carries it — only matters because it lets Matthew's peer make that comparison
  correctly and refuse. Everything downstream of the refusal — showing a conflict rather
  than swallowing it, rebuilding one partition without disturbing another, treating an old
  peer's unqualified record as UNKNOWN rather than trustworthy, defaulting an existing row
  to UNKNOWN when this contract lands under it, and proving Jessica can still heal her OWN
  clone corpus once the refusal exists — is what keeps the fix from being either useless
  (the row still escapes) or a lie (a contract that "passes" by refusing everything,
  including Jessica's own legitimate clone traffic).

  Vocabulary. `lamad` is the household's learning-content role: its base cell is the
  shared corpus every peer reads, and a `lamad.fixtures` (or `lamad.fixtures-2`) CLONE is
  a second cell under that same role name, with its own DNA hash and its own DHT, that
  only the operator who installed it holds. A CELL is one agent's participation in one
  DNA's validation context and gossip network. The SIX PLANES are a cell's three
  source-chain views (`*-actions`, `*-ops`, `*-content`), the DHT `targets` view, the
  storage `projection` rows, and the `sync` documents — `fixtures-clone.mjs inventory`
  dumps all six per peer. The QUALIFIER is the cell a projected row or sync doc came from,
  sourced from the conductor's own `Signal::App { cell_id, .. }` — never the payload's
  self-reported claim; that same signal also resolves the CLONE-BOUND `HcClient`, the
  conductor handle scoped to that one cell that every later read or write against the
  clone must go through. The projection row's WIDENED KEY is `(content id, cell_dna_hash)`
  rather than content id alone, so two cells can each hold their own row for what looks
  like the same content without one overwriting the other. A `clone_id` SEGMENT would mean
  a sync doc's own id embeds which clone wrote it; this contract qualifies the doc by
  DNA hash instead, so no doc id ever needs to leak which clone exists. cell_provenance is
  a projection row's own verified-or-not marker: `verified` (qualifier traced to a signal
  this peer received), `unknown` (an ingested record with no such signal — e.g. from an
  old peer or `store_acquired_record`), or `unheld` (names a cell this peer has not
  joined). A PARTITION is the set of rows sharing one qualifier; REBUILD is a from-scratch
  replay of one partition's rows, docs and cursors, done independently of every other
  partition. AN ANCHOR is the base-cell row a clone-authored write would need to already
  exist as, for the RE-AUTHOR PATH — the comparison a peer runs, before authoring anything
  into a base cell, between an incoming row's qualifier and the cells that peer itself
  holds — to treat it as an update rather than a fresh authorship. `upsert_with_anchor` is
  the storage write that inserts or updates one projection row, matching on the widened
  key rather than content id alone. The NULL-ANCHOR SWEEP is the periodic pass that looks
  for an anchor for every unqualified row and finds none — "NULL" names the anchor lookup's
  result, not a null value in the row. HEALING is the authoring peer re-anchoring its OWN
  clone corpus after a clone write, never a foreign peer's.

  Roles held for every station below. Jessica runs a `lamad` clone; every station's write
  originates on her peer. Matthew never joins Jessica's clone — he is the RECEIVING peer
  whose base `lamad` cell stations 4 and 5 must show untouched, and the peer whose own
  migration and UNKNOWN-ingest stations exercise pre-existing and old-peer traffic. Adam
  is a second household peer used only where a station needs a peer other than the
  authoring or receiving one (partition naming, rebuild isolation).

  Background:
    Given peer "jessica" at "E2E_STORAGE_JESSICA" with a "lamad" clone cell installed
    And peer "matthew" at "E2E_STORAGE_MATTHEW" holding only the base "lamad" cell
    And peer "adam" at "E2E_STORAGE_ADAM" holding only the base "lamad" cell

  # ---------------------------------------------------------------------------------
  # Station 1 — the qualifier is sourced from the signal, not the payload (contract §1)
  # ---------------------------------------------------------------------------------
  @wip @station-1
  Scenario: A clone write and a base write on the same peer are distinguished by the signal's own cell id, and a forged payload claim never overrides it
    Given Jessica's peer has both a "lamad.fixtures" clone cell and the base "lamad" cell
    When Jessica writes a content record into the "lamad.fixtures" clone
    And Jessica writes a content record into the base "lamad" cell
    Then the `ContentCommitted` handler for the clone write reports a different `cell_dna_hash` than the handler for the base write
    And that same signal resolves a clone-bound `HcClient` scoped to the "lamad.fixtures" cell
    When a write into the "lamad.fixtures" clone arrives with a payload-declared `cell_id` that disagrees with the signal's own `cell_id`
    Then the qualifier check fails that payload-declared claim, and the row is qualified by the signal's `cell_id`, never the payload's

  # ---------------------------------------------------------------------------------
  # Station 2 — the projection row carries the qualifier (contract §2.1)
  # ---------------------------------------------------------------------------------
  @wip @station-2
  Scenario: A clone-authored row lands in the projection with a non-NULL qualifier and its own key
    Given Jessica's peer has a "lamad.fixtures" clone cell installed
    When Jessica writes a content record into the "lamad.fixtures" clone
    And Jessica's peer's `fixtures-clone.mjs inventory` dumps her `projection.json`
    Then the row for that record carries a non-NULL `cell_dna_hash`
    And `upsert_with_anchor` matched that row on its widened key, not on content id alone

  # ---------------------------------------------------------------------------------
  # Station 3 — the sync doc carries the qualifier (contract §2.2)
  # ---------------------------------------------------------------------------------
  @wip @station-3
  Scenario: A clone-authored sync doc is qualified by the clone's own DNA hash and carries no clone_id in its id
    Given Jessica's peer has a "lamad.fixtures" clone cell installed
    When Jessica writes a content record into the "lamad.fixtures" clone
    And Jessica's peer's `fixtures-clone.mjs inventory` dumps her `sync.json`
    Then the doc for that record is qualified under the clone's own DNA hash, not the base "lamad" cell's
    And no doc id in `sync.json` contains a `clone_id` segment

  # ---------------------------------------------------------------------------------
  # Station 4 — the re-author refusal fires on the RECEIVING peer, at the plane where the
  # 2026-09-07 escape appeared (contract §3, D-9)
  # ---------------------------------------------------------------------------------
  @wip @station-4
  Scenario: Matthew's peer refuses to author Jessica's clone-qualified row into his own base cell
    Given Jessica's peer has a "lamad.fixtures" clone cell installed
    When Jessica writes a content record into the "lamad.fixtures" clone
    And Matthew's peer receives that record's projection row and sync doc
    Then Matthew's peer's re-author path compares the row's qualifier against his own held cells before authoring anything
    And Matthew's peer refuses to author the record into his base "lamad" cell, marking it `foreign-cell`
    And Matthew's `fixtures-clone.mjs inventory` shows his base-`lamad` `*-actions` and `*-ops` diffs EMPTY for that record

  # ---------------------------------------------------------------------------------
  # Station 5 — the remaining base planes stay empty on the receiver too (contract §3)
  # ---------------------------------------------------------------------------------
  @wip @station-5
  Scenario: No derived base-cell state follows the refused write on Matthew's peer
    Given Jessica's peer has a "lamad.fixtures" clone cell installed
    And Jessica has written a content record into the "lamad.fixtures" clone that reached Matthew's peer and was refused as `foreign-cell`
    When Matthew's peer's `fixtures-clone.mjs inventory` dumps his remaining base-cell planes
    Then Matthew's base-`lamad` `*-content` diff is EMPTY for that record
    And Matthew's base-`lamad` `targets` diff is EMPTY for that record

  # ---------------------------------------------------------------------------------
  # Station 6 — partition states are shown, not swallowed (contract §4)
  # ---------------------------------------------------------------------------------
  @wip @station-6
  Scenario: A second clone's write over the same content yields a conflict-visible id, and an unheld cell's row reads pending
    # Two independent partition-visibility probes of the same station: neither swallows a
    # state the other peer needs to see. The first is Jessica's own peer, holding two
    # clones of the same content; the second is Adam's peer, which was never given either
    # clone and must not guess at what it cannot verify.
    Given Jessica's peer has a "lamad.fixtures" clone cell installed
    And Jessica has written a content record into the "lamad.fixtures" clone
    When Jessica's peer installs a second clone, "lamad.fixtures-2", and writes over the same content id there
    Then Jessica's peer's projection shows a conflict-visible id for the two clone-qualified rows, not an overwrite of one by the other
    When that same projection row reaches Adam's peer, naming a cell Adam's peer has never joined
    Then Adam's peer reads that row as `pending — foreign cell`

  # ---------------------------------------------------------------------------------
  # Station 7 — rebuild is per-partition and atomic (contract §5)
  # ---------------------------------------------------------------------------------
  @wip @station-7
  Scenario: Rebuilding one partition changes no row of another and clears its own rows, docs and cursors together
    Given Jessica's peer holds a base "lamad" partition and a "lamad.fixtures" clone partition, each with committed rows, sync docs and cursors
    When Jessica's peer rebuilds only the "lamad.fixtures" partition
    Then every row, doc and cursor of the base "lamad" partition is unchanged
    And the "lamad.fixtures" partition's prior rows, docs and cursors are cleared together before the rebuilt ones are written
    And the rebuilt "lamad.fixtures" partition's rows match its source cell exactly

  # ---------------------------------------------------------------------------------
  # Station 8 — an unqualified record from an old peer lands UNKNOWN (contract §2.3)
  # ---------------------------------------------------------------------------------
  @wip @station-8
  Scenario: A record ingested with no claim through store_acquired_record is provenance-unknown and never auto-authored
    Given a `ContentRecord` with no cell claim, shaped as an old peer still on the pre-qualifier wire format would send it
    When Matthew's peer ingests that record through `store_acquired_record`
    Then the resulting projection row's `cell_provenance` reads `unknown`
    And Matthew's peer's NULL-anchor sweep leaves that row un-authored into any base cell

  # ---------------------------------------------------------------------------------
  # Station 9 — migration defaults existing rows to UNKNOWN and resolves by anchor (contract §6)
  # ---------------------------------------------------------------------------------
  @wip @station-9
  Scenario: A pre-existing clone-authored row migrates to UNKNOWN, is never re-authored on no anchor, and later accepts a verified qualifier
    Given Matthew's peer holds a projection row written before this contract, known to have been clone-authored but carrying no qualifier column
    When Matthew's peer applies the migration
    Then that row's `cell_provenance` reads `unknown`
    When Matthew's peer's anchor-resolution sweep runs and finds no matching row in any base cell
    Then the migrated row is never re-authored into a base cell
    When a later write supplies a verified clone qualifier for that same row
    Then Matthew's peer accepts the verified qualifier and the row's `cell_provenance` reads `verified`

  # ---------------------------------------------------------------------------------
  # Station 10 — clone content stays live for its own authoring peer (contract §3 case 1)
  # ---------------------------------------------------------------------------------
  # This is the one station among 4-10 where a write completes rather than being refused —
  # the proof that the refusal contract does not also block Jessica's own legitimate clone
  # traffic.
  @wip @station-10
  Scenario: Jessica's own peer re-anchors and heals her clone corpus after a clone write, using her clone's own resolved client
    Given Jessica's peer has a "lamad.fixtures" clone cell installed and has written a content record into it
    When Jessica's peer runs its own healing sweep over the "lamad.fixtures" clone corpus, using the clone-bound `HcClient` resolved from the `ContentCommitted` signal for "lamad.fixtures"
    Then Jessica's peer re-anchors and heals the record within its OWN "lamad.fixtures" clone
    And the healed row lands qualified under Jessica's own clone
    And Jessica's peer serves the healed record from her "lamad.fixtures" clone without any qualifier refusal
