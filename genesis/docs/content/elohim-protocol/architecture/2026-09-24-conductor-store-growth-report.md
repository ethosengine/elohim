---
title: What grows in a conductor store — the 2.3 GB matthew inspection, its driver, and the limits it asks for
id: conductor-store-growth-report
date: 2026-09-24
status: Measured report — household mesh; fleet numbers not yet taken
author: Fable session 2026-09-24, from an offline read of the stopped Dowell household conductors
cites:
  - genesis/data/timeline/backlog/conductor-publish-livelock-fk787.md
  - "substrate-trust-contract-runbook | the runbook whose per-red decision tree this report extends with a storage-growth probe | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - app/elohim-app/scripts/conductor-store-sniff.py
  - doorway/doorway-service/src/services/federation.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs
---

# What grows in a conductor store

**The question.** The persisted household's matthew conductor holds a 2.3 GB
store. The 2026-09-23 handoff read that as "accumulated over weeks" and as the
local analog of the fleet's post-restart window. What is in it, what is
growing, and what does that mean for performance, I/O, durability and the
limits we have to design in?

**The short answer.** The store is two days old, not weeks: the household was
recast at 2026-09-22 03:54 and inspected at 2026-09-23 22:04, a 42-hour window.
Of the 2.3 GB, 0.5 GB is a fixed per-pin wasm cache and 1.7 GB is the lamad
DHT: 322 MB of database plus a 1.4 GB write-ahead log that has never been
checkpointed back to zero. Nearly all of the lamad growth is one writer: the
doorway peer-health probe, which every 150 seconds records a **new, immutable
content node** for every doorway ever registered, dead or alive. 27,317 of the
30,787 entries in matthew's lamad database are `attestation:device-health`
records; 25,973 of them say `unreachable`, attesting to 160-odd lane doorways
that were torn down within minutes of being born. Nothing prunes the roster and
nothing prunes the records. Everything below hangs off that one fact.

## 1. What the 2.3 GB is

Measured on the stopped matthew conductor (`conductors/matthew/databases/`):

| Component | Size | Nature |
|---|---|---|
| `wasm.db` | 514 MB | `Wasm` (raw) + `CompiledWasm` (serialized wasmer modules) for every zome in the hApp. Identical (~539 MB) on all three peers. Fixed per conductor pin; grows only when a pin or a zome changes, and never shrinks (rows are never deleted). |
| lamad DHT, main file | 322 MB | 82,503 pages. Row payload is 158 MB; the rest is index and page overhead. |
| lamad DHT, WAL | **1,407 MB** | ~350k WAL frames against 82.5k pages: every page has been rewritten ~4× since the last WAL reset. Jessica's is 1,681 MB. |
| infrastructure DHT | 24 + 10 MB | Doorway registry, `active|<ts>` links. |
| imagodei / mishpat / node_registry DHTs | 18, 9, 2 MB | Small. |
| `conductor.db`, peer-meta stores | < 25 MB | Negligible. |

The three peers differ only in how far each one's WAL has run away:

| Peer | lamad main | lamad WAL | Actions | Entries | ChainOps | Receipts |
|---|---|---|---|---|---|---|
| matthew | 322 MB | 1,407 MB | 93,326 | 30,787 | 280,350 | 107,977 |
| jessica | 531 MB | 1,681 MB | 90,712 | 29,927 | 271,956 | 108,087 |
| james | 473 MB | 414 MB | 78,367 | 25,842 | 234,775 | 101,193 |

Full-arc replication (`target_arc_factor: 1`) means each peer holds
everyone's ops, so the counts converge; the main-file sizes differ by free-page
and index history, not by content.

## 2. What is in the lamad database

Row counts and payload on matthew, lamad DHT:

| Table | Rows | Payload | What it is |
|---|---|---|---|
| ChainOp | 280,350 | 42 MB | 3 ops per action (StoreRecord, RegisterAgentActivity, plus StoreEntry or RegisterAddLink). All integrated, all valid. |
| Entry | 30,787 | 38 MB | avg 1.27 KB, max 11 KB. |
| ValidationReceipt | 107,977 | 35 MB | ~2 per op that requires one: the other two full-arc peers each send a receipt. |
| Action | 93,326 | 32 MB | 62,093 CreateLink · 30,836 Create · 342 Update · 54 genesis. |
| ChainOpPublish | 133,309 | 7 MB | one row per authored op; `receipts_complete` is NULL on every row. |
| Link | 62,093 | 5 MB | see below. |

**Who wrote it.** 19 authors. Three of them — the storage peers' own agents,
which the doorways call zomes through — wrote 93,252 of the 93,326 actions.
The sixteen hosted humans wrote 3–5 actions each: their genesis. Hosted humans
are not the growth; they cost conductor heap, not disk.

| Author (agent key prefix) | Actions | Who |
|---|---|---|
| `uhCAk55Z0…` | 44,211 | matthew's storage agent (doorway-a calls through it) |
| `uhCAkagHb…` | 28,754 | jessica's (doorway-b) |
| `uhCAkZ2Mw…` | 20,287 | james's (doorway-c) |

**What the entries are** (by content type string inside the blob):

| Kind | Entries | Bytes each |
|---|---|---|
| `attestation:device-health` | 27,317 | 1,320–1,322 (unreachable) / 1,223–1,228 (online) |
| REA `provide` economic events (`replicates-commons`) | 2,608 | ~865 |
| doorway registrations, canonical-head declarations, consent, paths, collective patterns | ~860 | varied |

**What the links are.** Each attestation creates two: an `AttestationToSubject`
link on the subject's string anchor tagged `device` (27,092), and a content-id
index link (27,588 untagged, link type 0). Per subject anchor that is now
600+ links and rising, and the attestation content-type anchor holds all
27k. Those are hot bases: a `get_links` on either returns the whole history.

## 3. The driver, and why it is unbounded

`doorway/doorway-service/src/services/federation.rs` runs a heartbeat task per
doorway at `heartbeat_interval_secs` (30 s). Every fifth tick (150 s) it walks
`cached_peers` — the roster from `get_all_doorways`, which is every doorway
ever registered in the infrastructure DNA — probes each with a 10 s timeout,
and calls `record_health_attestation` with the result. The infrastructure zome
(Stage C, "no local create_entry") bridges that into the content_store zome,
whose `create_attestation` does `create_entry(Content)` plus the two links.
There is no update chain, no `expires_at` enforcement (the field exists in
metadata only), no roster expiry, and no deregistration on lane teardown.

The a2o serving receipt (`epr-deliverability-doorways-*`) casts two scenario
doorways per run and tears them down four minutes later. Their registrations
stay. The doorway logs show the roster at 10, 11, 18, 20, 29 and **178** peers
at different moments; the lamad database holds attestations for **179 distinct
subjects**, of which three are the real household doorways and the rest are
dead scenario doorways. So the attestation rate is
`doorways × registered peers / 150 s`, throttled only by the sequential 10 s
timeouts against peers that no longer exist — which is why the measured idle
rate (Sep 22 05:00–11:00, no lanes running) is a flat **648 Create + 1,296
CreateLink per hour**, and why it doubles to 2,000–2,100 Creates/hour whenever
a lane adds two more probing doorways and two more subjects.

That is quadratic in the number of doorways that have ever existed, on an
append-only substrate, replicated to every full-arc peer, and each record costs
roughly 12 KB of main-file per peer once its 9 ops, 6 receipts, 9 publish rows
and index entries are counted. 27k attestations ≈ 322 MB. Left alone, the
household writes about 0.5 GB/day of main file per peer at the idle rate and
more under lanes; the WAL adds its own multiple on top.

The other writers are healthy: 2,608 REA provide events over 42 hours is what
the deliverability lane is supposed to produce, and the canonical-head and
registration records are one-per-event.

## 4. Why the WAL is four times the database

The fork opens every database in WAL mode with SQLite's default auto-checkpoint
(1,000 pages) and never issues an explicit checkpoint or `journal_size_limit`
(`holochain_data/src/lib.rs`, `create_pool`; grep finds no `wal_checkpoint`
anywhere). A WAL can only restart from frame zero when a checkpoint has copied
every frame and no reader still holds a snapshot. On this conductor there is
always a reader:

- the integration workflow re-triggers itself ("Work incomplete, re-triggering
  workflow - integration", 100 times in the log) and the queue consumers fire
  thousands of workflow runs;
- gossip time-slice queries walk `ChainOp` by `storage_center_loc` and time;
- elohim-storage's 60 s health tick calls the admin `storage_info`, which runs
  `SELECT sum(pgsize) FROM dbstat` **twice** per DNA — a full page walk of the
  database, measured at 1.1–4.2 s on the earlier 2026-09-20 household and up to
  76.8 s on the worst statement. That probe alone holds a read snapshot for
  seconds every minute and its cost is linear in database size.

Under those readers the WAL never resets; each publish cycle and each
validation-status update rewrites pages into it, so the WAL accumulates ~4
copies of every page. Consequences:

- **Restart.** SQLite recovery on open reads the whole WAL to rebuild the
  index (`-shm` is 2.8 MB = 350k frames × 8 B), then the first write
  checkpoints 1.4 GB into the main file. That happens before the cell is
  RUNNING and before the kitsune DHT-model rebuild starts — it is part of the
  post-restart window the handoff could not see.
- **I/O and durability.** Every logical page write is amplified: WAL frame
  now, checkpoint copy later, and on this host's worn QLC mirror
  (`reference_node_hardware_ethosengine_shem`) that is write endurance spent on
  attestations of dead doorways. `synchronous=Normal` means a crash loses the
  last un-fsynced WAL frames but not integrity.
- **Memory.** The `-shm` index is mmapped; 350k frames is small, but the
  page cache churn from a 1.4 GB working set is not.
- **Read performance.** Every read on a WAL-mode database consults the
  wal-index for each page; with 4 frames per page the lookups get slower and
  the OS cache holds four versions of the same page.

The 2026-09-20 household — the one that hit the publish storm — showed the
same shape in its slow-statement log: 1,029 statements over the 1 s threshold
in one hour, dominated by `SELECT ChainOp.hash … dht_hash` (publish query),
`SELECT (SELECT COUNT(*) …` and the dbstat walk. The publish-livelock backlog
(`conductor-publish-livelock-fk787`) already names the query; this report names
what feeds it: `ChainOpPublish` holds 133,309 rows with `receipts_complete`
NULL on every one, so every publish cycle reconsiders every op the peer has
ever authored, and that set grows at the attestation rate.

## 5. What this predicts for the fleet

Alpha runs the same doorway code, the same 150 s probe, and a roster that
includes every doorway that ever registered against it (the household saw 178
because lane doorways register there too). Its lamad store is weeks old. So
the fleet's matthew conductor should be assumed to hold hundreds of thousands
of attestation nodes and a WAL in the multi-GB range, and the 3h20m
"restart to RUNNING" the handoff measured is the sum of WAL recovery, WAL
checkpoint, and the per-sector DHT-model rebuild over an op table that is
~90 % health attestations. The pin bump to `7e553f9c3` (bounded chain reads,
sql timing) speeds up the scan; it does not change the input size. The
measurement the handoff asks for (restart-to-RUNNING per role, current pin vs
new) is still the right measurement, and this report gives it a controlled
variable: run it once on the store as is, once after checkpointing the WAL
(`PRAGMA wal_checkpoint(TRUNCATE)` on the stopped database), and once after
the attestation writer is bounded, so the three costs separate.

## 6. How to observe it — the sniffer

`app/elohim-app/scripts/conductor-store-sniff.py <conductor-data-root>` reads a
stopped conductor offline and prints, per DNA: main/WAL/shm sizes and their
ratio, rows and payload per table, actions by type and per hour, top authors,
op mix, receipts-per-op, top link types and tags, and the dominant entry kinds
by content-type string. `--json` emits the same as a document for a habit
check. It decrypts `databases/db.key` the way `holochain_data/src/key.rs` does
(argon2id over the sandbox passphrase, XSalsa20-Poly1305 secretbox) and opens
with sqlcipher pragmas mirrored from `apply_pragmas`, read-only. Dependencies
are two wheels (`pynacl`, `sqlcipher3-binary`); the docstring has the pip line.
Never point it at a running conductor: a long read snapshot is exactly the
checkpoint starvation described above.

This is the "space sniffer" shape any node operator could run: it needs no
conductor, no admin port, and no knowledge of zome internals — only the data
root and the passphrase they already own. The numbers it prints are the ones a
per-DNA growth budget would be written against. The fleet equivalent is the
same script run inside a pod against a copied data dir, or the conductor
exporting the same rows as metrics (see §7).

## 7. What to design in — limits and system awareness

These are the decisions the inspection puts in front of us. The first two are
immediate; the rest are the design questions the numbers raise.

1. **Bound the attestation writer (immediate).** Per the p2p-design-gate
   question the record never answered: a peer-health observation is
   **Ephemeral (class C)** or at most an **attested-private (B2)** sample with
   a retention window. It is not a notarized Content node. Options in
   increasing cost: (a) probe only the live roster — expire registrations
   whose `active|<ts>` link is older than N heartbeats and skip them;
   (b) collapse the history per (attestor, subject) into an update chain
   (`update_entry` on the previous attestation) so the DHT holds the latest
   plus a bounded tail; (c) move the sample off the DHT entirely into the
   peer-meta store or a storage-side table and notarize only state
   transitions (online→unreachable, unreachable→online). Any of these turns
   quadratic growth into linear-in-real-doorways. The a2o serving receipt
   should deregister its scenario doorways on teardown regardless.

2. **Let the WAL reset (immediate, fork).** Add an explicit
   `PRAGMA wal_checkpoint(PASSIVE)` after each integration batch and a
   `TRUNCATE` checkpoint at idle or shutdown; set `journal_size_limit`. And
   replace the `dbstat` walk in `storage_info` with
   `page_count × page_size` (O(1)) plus the WAL file size, so the health
   probe reports the thing that is actually hurting and stops being a reader
   that starves the checkpointer.

3. **A per-DNA growth budget as a Mishpat limit.** The register we lack is
   "how many ops per day may a role write, and who is allowed to make that
   number go up." That is a bounded_by commitment in Mishpat vocabulary and a
   system-awareness measure in `.epr-meta`: declared ceiling per DNA on
   `actions/day`, `links per anchor`, and `WAL/main ratio`, with the sniffer
   (or the conductor's own metrics) as the probe and a red flip when
   exceeded. The limit-raise rule from
   `feedback_limit_raises_are_design_signals_not_capacity` applies exactly:
   a writer that needs the ceiling raised is a design loop, not a capacity
   ask. Suggested first values, from this data: lamad ≤ 5k actions/day/peer
   at idle; ≤ 100 links per subject anchor; WAL/main ≤ 1.

4. **Hot anchors are a read-side limit too.** 600+ links per subject anchor
   and 27k per type anchor means `get_links` cost is already linear in history.
   Any anchor that grows with time needs bucketing (time-sharded anchors) or a
   cap; the content-type index link per attestation should not exist at all.

5. **Full arc multiplies every mistake by N.** At arc factor 1 each peer
   stores and validates every op and sends receipts for each; a writer's cost
   is paid by the whole household, and on the fleet by every full-arc peer.
   The growth budget belongs at the writer, not at the replica.

6. **The wasm cache is fixed, but never shrinks.** 514 MB of compiled modules
   is acceptable per pin; a conductor that has seen ten pins holds ten. Worth a
   `retire-when` on old `CompiledWasm` rows, low priority.

## 8. What this report does not establish

- Fleet numbers. Nothing here was measured on alpha; §5 is inference from
  identical code and roster behaviour.
- Whether the `receipts_complete = NULL` on all 133k publish rows is the
  livelock backlog's FK-787 symptom or a separate never-completes path.
  Worth one query on the fork's `record_published_op_hashes`.
- The exact wall-clock split of the post-restart window between WAL recovery,
  checkpoint and DHT-model rebuild. §5 names the three-run measurement that
  would give it.

**Method note.** Every number above came from `conductor-store-sniff.py` (or
its scratch predecessor) against the stopped household at
`genesis/local-dev/household-dowell/conductors/`, plus grep over the doorway,
storage and conductor logs and the fork source at pin `7e553f9c3`. No conductor
was started and no database was written.
