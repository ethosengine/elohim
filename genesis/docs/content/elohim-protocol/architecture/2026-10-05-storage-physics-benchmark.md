---
title: What the genesis corpus should cost to hold — a storage benchmark at five holders, and what the fleet holds instead
id: storage-physics-benchmark
date: 2026-10-05
status: Benchmark draft — corpus and household measured; fleet read from Prometheus; per-head floor is an estimate
author: Fable session 2026-10-05 (shem)
cites:
  - "conductor-store-growth-report | the measured per-entry DHT cost this benchmark prices heads with | path: genesis/docs/content/elohim-protocol/architecture/2026-09-24-conductor-store-growth-report.md"
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
---

# What the genesis corpus should cost to hold

**The question.** The seed corpus is about 30 MB. If every blob is held by five
peers (the high-availability benchmark), and the notarized heads are small text
claims over those blobs, how many bytes should the whole test network persist?
And how far is the alpha fleet from that?

**The short answer.** About **0.5 GB across the whole fleet** for data, plus a
fixed code cost per conductor. The four alpha peers we can read per-volume hold
**6.0–6.6 GB each in the conductor alone**. Storage is 5–40× over its budget;
the conductor is roughly 100× over the head budget. Almost none of the excess
is the corpus.

One correction to the starting intuition: for *this* corpus the heads are not
magnitudes smaller than the blobs. The typical item is a 1.5 KB markdown
scenario, and one notarized head costs 4–12 KB per holder. Heads are small only
relative to large blobs, or when one head covers many items.

## 1. The corpus (measured, 2026-10-05)

| What | Count | Bytes |
|---|---|---|
| Content items (`data/lamad/content`) | 3,495 | 17.0 MB as seed JSON: 6.2 MB bodies, 9.7 MB metadata |
| Item size | | median 3.8 KB, p90 6.8 KB, p99 25 KB, max 121 KB |
| Content relationships | 3,193 | 1.0 MB |
| Presences, humans, devices, collectives, paths | ~330 | ~2 MB |
| Unique blob bytes in one full blob store | 3,574 blobs | **31.8 MB** |
| of which four app bundles over 1 MB | 4 | 26.7 MB |
| of which everything else (content bodies) | 3,570 | 5.1 MB, mean 1.4 KB |

Metadata outweighs bodies (2.8 KB against 1.8 KB per item), and 84% of the blob
pool is four application bundles. The "30 MB corpus" is 27 MB of app code and
5 MB of words.

## 2. The benchmark

Unit costs for heads come from the 2026-09-24 inspection of a stopped household
conductor: 3 ops per action, about 2 receipts per op at full arc, and **about
12 KB of main database file per record** for a 1.27 KB entry with two index
links. The entry body is a minor part of that; a reference-only head on the
same op model costs nearly the same.

| Term | Per holder | Basis |
|---|---|---|
| Blobs | 32 MB | measured unique bytes |
| Row projection (sqlite) | ~20 MB | estimate: 9.7 MB metadata plus indexes; not measured on a fully seeded peer |
| Heads, as built | ~46 MB | 3,800 heads × 12 KB (measured unit cost) |
| Heads, floor | ~15 MB | estimate: reference-only entry, one index link, bounded receipts, ≈4 KB |
| Code (`wasm.db`) | ~150 MB floor | estimate from a 9.7 MB packed hApp; measured value is in §3 |

| Fleet total | Five holders | All seven peers hold everything |
|---|---|---|
| Blobs | 160 MB | 224 MB |
| Rows | 100 MB | 140 MB |
| Heads, as built / floor | 230 MB / 75 MB | 322 MB / 105 MB |
| **Data total** | **~335–490 MB** | **~470–690 MB** |
| Code, fixed (7 conductors) | ~1.05 GB at the floor | same |

Per peer, the data budget is **about 70–100 MB**. The fixed code cost is
separate because it does not scale with the corpus or with replication.

## 3. What is held (measured)

**Alpha fleet, Prometheus, 2026-10-05 16:49 UTC.**

| Peer | Conductor volume | Storage volume | Storage process's own figure |
|---|---|---|---|
| adam | 6.56 GB | 2.22 GB | 825 MB |
| eve | 6.32 GB | 0.31 GB | 270 MB |
| gertrude | 6.38 GB | 0.30 GB | 270 MB |
| susan | 5.95 GB | 0.32 GB | 281 MB |
| matthew | not readable | not readable | 1,250 MB |
| jessica | not readable | not readable | 277 MB |
| james | not readable | not readable | 280 MB |

Matthew, jessica and james sit on a shared node filesystem; the volume gauge
reports the whole filesystem (587 GB) for each, so there is no per-peer
conductor figure for them. No metric exposes conductor database size, DHT op
count, blob count or row count on any peer.

| | Budget per peer | Held per peer | Ratio |
|---|---|---|---|
| Storage (blobs + rows) | ~52 MB | 270–280 MB; adam 0.8–2.2 GB; matthew 1.25 GB | 5× to 40× |
| Conductor (heads + code) | 46 MB + code | 6.0–6.6 GB | ~9× with today's code cost granted; ~130× against heads alone |

**Household, three peers, lightly seeded (65 content rows), offline read.**

| Component | Per peer | Note |
|---|---|---|
| `wasm.db` + WAL | 675 MB + 132 MB | Code, not data. Was 514 MB on 2026-09-24; rows are never deleted. Gertrude's was 3.16 GB on 2026-10-01. |
| All five DHT databases + WALs | ~60–90 MB | lamad 20–33 MB |
| Blob store | 31.8 MB of bytes, **130 MB allocated** | 3,574 small files on a 128 KB-record filesystem: about 4× slack |
| Extraction cache | 27 MB (41 MB apparent) | |
| `content.db` + WAL | 3 MB + 5–9 MB | |
| Kept coordinator bundle | 9.3 MB | |

**Alpha fleet, storage volume by file class, 2026-10-05 19:28 UTC** (first
read of the §6 gauges, after edge #1547; MB, file length).

| Peer | Blobs | Blob files | Second blob store (`blobs_iroh`) | Extraction cache | Content db + log | Release adoption | Known holders per shard: 1 / 2 / 3 / 4 / 5+ |
|---|---|---|---|---|---|---|---|
| matthew | 1,192 | 273 | 1,110 | 1,141 | 34 | 47 | 266 / 4 / 3 / 0 / 0 |
| adam | 787 | 176 | 663 | 966 | 30 | 25 | 172 / 2 / 3 / 0 / 0 |
| jessica | 264 | 99 | 1.5 | none | 14 | 25 | 93 / 3 / 2 / 2 / 0 |
| james | 267 | 43 | 1.5 | none | 12 | 25 | 38 / 3 / 3 / 1 / 0 |
| susan | 268 | 42 | 1.5 | none | 9 | 25 | 38 / 1 / 3 / 0 / 0 |
| eve | 257 | 39 | 1.5 | none | 9 | 21 | 36 / 2 / 1 / 2 / 0 |
| gertrude | 257 | 39 | 1.5 | none | 9 | 12 | 34 / 4 / 1 / 2 / 0 |

What this read establishes:

- **The fleet's blob stores are not the seed corpus.** A quiet peer holds
  257–268 MB in about 40 files, roughly 6.5 MB a file. The household's full
  store is 31.8 MB in 3,574 files. The fleet holds few, large blobs; the
  small per-content blobs are not there.
- **Small-file slack is not the fleet's problem.** Allocated bytes match file
  length on the blob stores. The §4 item that suggested otherwise is withdrawn
  for the fleet; it stands for the household's filesystem only.
- **Matthew and adam hold their blobs about three times.** Once in the blob
  store, once more in a second store of nearly the same size, and once
  unpacked in an extraction cache. Matthew's storage is 3.5 GB of which
  1.2 GB is the blobs themselves.
- **No shard on any peer is known to five holders.** Almost every shard is
  known to one.
- **The conductor split is still unread.** The conductor pods did not restart:
  they take a new image only when the conductor pin moves or an operator asks.
  No conductor pod exports the new gauges yet.

**Where the growth is (14-day history of each peer's used bytes, against
build logs).**

- **Matthew accumulates app releases.** Each app build that changes content
  publishes four zips (two apps, browser and server) totalling 23.66 MB, read
  from the app pipeline's log. Matthew grew from 801 MB to 1,250 MB between
  2026-09-21 and 2026-10-03, in steps that are whole multiples of 23.66 MB:
  19 releases, 449 MB. Identical bundles deduplicate; nothing was seen to
  delete an old release. With the second blob store and the extraction cache
  tracking it, the volume grows about three times that.
- **The quiet peers do not grow.** Gertrude and eve held 269.7 MB on
  2026-09-21 and 269.9 MB today. App releases do not replicate to them. Their
  ~40 files match the ~41 shards each registers, mostly with one known
  holder. What those shards are has not been identified: no stored-blob log
  line, CID or source was found.
- **Adam** rose from 636 MB to 825 MB between 2026-09-21 and 2026-09-24; not
  investigated.

So the storage-side runaway is one peer's release history, and it is the
ten-release window of §7 with no bound: 19 releases held where 10 are
declared. The quiet peers' 270 MB is a separate, static question.

## 4. Where the excess is

Ranked by measured bytes. The first three are owed regardless of design.

1. **Code stored as data, never collected.** `wasm.db` holds raw and compiled
   wasm for every zome version a conductor has ever run. Each coordinator
   hot-swap adds a set; nothing removes one. One conductor's code store is
   20–100× the entire corpus.
2. **Writers that are not the corpus.** The September inspection found 27,317
   of 30,787 lamad entries were doorway health attestations. That writer is now
   bounded, but the fleet's 6 GB conductors have not been re-inspected, so what
   fills them today is not established.
3. **Write-ahead logs and snapshots.** Household WALs ran to 1.4 GB before;
   gertrude's volume carried 14.7 GB of snapshots against 5.3 GB live.
4. **Small-file allocation.** A blob per 1.4 KB body costs a filesystem record
   each. The 270 MB every quiet fleet peer reports is consistent with this
   (not confirmed: the fleet's record size is not known here).
5. **One head per item, two links per head, receipts from every full-arc
   peer.** This is the design term. A head that covers a collection (one
   notarized root over the 2,946 scenarios) would move the head plane from
   tens of MB to under 1 MB per holder; that is where "magnitudes smaller"
   actually lives.
6. **Everyone holds everything.** Seven full-arc conductors and seven full
   blob stores is 1.4× the five-holder benchmark. It is the smallest term.

## 5. Bounds the stories can assert

Stated for a freshly seeded fleet at rest; each needs a gauge that does not
exist yet (§6).

| Claim | Floor | Today's honest bound |
|---|---|---|
| Blob bytes held fleet-wide ÷ unique corpus bytes | 5.0 | ≤ 7.0 (everyone holds) |
| Storage volume per peer | ≤ 60 MB | ≤ 150 MB until small-file slack is fixed |
| DHT bytes per notarized head, per holder | ≤ 4 KB | ≤ 12 KB |
| Conductor data (all DHT databases + WALs) per peer | ≤ 20 MB | ≤ 60 MB |
| Code store per conductor | ≤ 150 MB | ≤ 700 MB, and flat across coordinator swaps |
| Growth at rest over 24 h, any volume | 0 beyond a declared heartbeat budget | same |

## 6. Gauges that close the gap (written 2026-10-05, not yet on the fleet)

`elohim-storage/src/services/store_footprint.rs`, sampled every 300 s:

| Gauge | Reads | Answers |
|---|---|---|
| `elohim_node_store_bytes{scope="conductor",store,part,dna,measure}` | every file under the conductor data directory, classed `code` (`wasm.db`), `dht`, `cache`, `authored`, `peer_meta`, `conductor`; `part` = main/wal/shm; `dna` = short DNA hash | heads against code, per DNA, with write-ahead logs separate |
| `elohim_node_store_bytes{scope="storage",…}` | every file under the storage directory, classed by top-level entry (`blobs`, `content_db`, `cache`, …) | blob bytes, row projection size |
| `measure="apparent"` against `measure="allocated"` | file length against blocks on disk | small-file slack |
| `elohim_node_store_files{…}` | file count per class | blob count |
| `elohim_shard_known_holders{holders="1".."5+"}` | `shard_locations`, distinct peers per shard | how many holders each node *knows* of |

Bytes per head per holder is then
`elohim_node_store_bytes{scope="conductor",store="dht",measure="apparent"}`
summed over parts, divided by `elohim_node_corpus_docs` (already exported).

**Holder knowledge, household, offline read of matthew's database.** 3,549 of
3,574 shards have exactly one known holder (matthew itself); 16 have two and 9
have three. Jessica holds the same 32 MB of blobs, so the bytes are replicated
but the record of who holds them is not. The five-holder claim is unobservable
because no node knows it, not only because no gauge exports it.

## 7. The retention floor for code

Rollback needs old versions to exist somewhere. It does not need every
conductor to keep every compiled module it has ever run. Three forms of the
same code have very different costs and very different owners:

| Form | Size per version | Derivable from | Who must hold it | Floor |
|---|---|---|---|---|
| Packed release bundle | ~10 MB | nothing: this is the source | the network, as content at the five-holder benchmark | every release still inside a supported upgrade or rollback window |
| The bundle a peer last applied | ~10 MB | the network, if reachable | each peer, locally (`coordinators/last-applied.happ` today) | current plus the one before it, so a peer can step back with no network |
| Raw WASM in the conductor | a few MB | the bundle | the conductor | what installed DNA definitions reference |
| Compiled module in the conductor | tens of MB | the raw WASM, by recompiling | the conductor | what is in use; it is a cache |

**The rule.** Durability of old versions belongs to the content plane, where
it costs 10 MB a version and is held by five peers. A conductor holds what it
runs. Holding a superseded compiled module in every conductor buys no
resilience the bundle does not already give, at 20–50× the bytes.

**What this costs at the floor.** Twenty retained releases at five holders is
1 GB network-wide, 143 MB per peer if spread over seven. A conductor at rest
holds one raw and one compiled set: the 150 MB estimate in §2.

**What the conductor does today.** `put_dna_def` replaces a DNA's zome rows on
every coordinator update and leaves the superseded `Wasm` and `CompiledWasm`
rows in place. Nothing deletes them. This is read from the fork's source
(`holochain_data/src/wasm`); the database is encrypted, so the row counts on a
grown store have not been read.

**The bound written 2026-10-05 (fork, untested at time of writing).** After
each DNA definition write, delete WASM no definition references, keeping the
newest unreferenced rows up to three times the count in use. The margin is
required, not a convenience: an install stores its WASM before the definition
that references it, so the newest unreferenced rows may belong to an install
in flight.

**Release retention is a mishpat (operator, 2026-10-05).** The network keeps
the ten latest releases of a channel, matching what the build server retains
today. At five holders that is 500 MB network-wide, about 70 MB per peer over
seven.

**Who may change what a peer runs (operator, 2026-10-05).** Holochain's stance
is that code is local-first so that no upgrade or rollback can be forced: the
software a group relies on is there when needed and nobody can take it away.
The Elohim Protocol keeps the second half and differs on the first. It *can*
compel an upgrade or a rollback, and the authority to do so comes from the
shape of trust and standing around the person (their own governance, a family
member, a workplace, a church, a library), not from a publisher or an
operator. Three storage consequences follow:

- **Nothing a peer is running is ever deleted from that peer.** Retention
  rules remove superseded versions only. A compelled change arrives as an
  adoption of another version; deletion follows adoption, never precedes it.
- **The retention window bounds how far back governance can compel.** A
  rollback target outside the ten retained releases cannot be compelled,
  because nobody is obliged to still hold it.
- **The holders of a release should be the holon with standing to compel it.**
  Five arbitrary holders make a rollback available; holders inside the
  compelling holon make it available *to the people it binds* when the wider
  network is unreachable.

**Retention depth follows what the device is for (operator, 2026-10-05).**
The ten-release window is held by the peers whose purpose is to hold it. A
personal device keeps only enough to step once in either direction.

| Device purpose | Releases held | Bundle bytes | Example |
|---|---|---|---|
| Hub (holds for others) | the full window, 10 | ~100 MB | adam's and matthew's hubs |
| Personal device | current, plus one either side when present | ~20–30 MB | a chromebook |

"One either side" means the previous release, and the next one once it has
been fetched ahead of adoption. A personal device therefore upgrades and rolls
back one step at a time; a larger step goes through a hub it can reach. The
five-holder benchmark for release bundles is met by hubs, not by every peer,
so a holon's rollback reach is the number of independent hubs it has.

**Whose concern the hub-held window is (operator, 2026-10-05).** Release
artifacts held in trust by hubs are brit and rakia concerns. Those two subsume
every repository-shaped concern for artifacts that need that level of
integrity: package registries (npm, cargo and the like), the image registry,
and the build server's own artifacts. The window therefore covers dependency
packages and container images as well as hApp bundles; only hApp bundles have
a size figure in this document. So the ten-release
window, its holders and its integrity checks are declared there; the storage
dataplane carries the bytes and reports what it holds, and does not decide
what a release is or when one leaves the window.

No field carries this today. The fleet's node types (`operations`, `edge`,
`performance`, `remote`) describe where a peer runs, not what it retains.

**Open for the upgrade and rollback epic.**
- Where retention depth is declared: a device capability the peer states, a
  policy its holon assigns, or both with the smaller winning.
- Whether retention is counted per channel or per holon, and what orders "the
  ten latest" when holons have adopted different heads.
- A peer that steps back past its two local bundles depends on reaching a
  holder. Whether two is the right local depth for a household that is offline
  for weeks is a resilience question, not a storage one: each extra step costs
  10 MB.
- Existing stores do not shrink when rows are deleted. Reclaiming the fleet's
  grown files needs a one-time rewrite of each database with free space equal
  to its live size.

## 8. Declared defaults for the nascent network

The dataplane cannot read trust yet, and will not until elohim agents are
mature enough to help write the stories that declare it. Until then the
developers answer each gap explicitly, in the genesis stories, with a fixed
default. Each row is a declaration, not a measurement and not a governance
outcome; the last column names what replaces it.

| Gap the floor cannot read | Default for now | Source | Replaced by |
|---|---|---|---|
| How many peers hold a blob | 5 | operator: the high-availability mishpat | a custody declaration per object |
| Which peers hold it | any five; on today's seven-peer fleet, all seven | this document | holders chosen from the holon with standing |
| How many releases the network retains | the 10 latest per channel | operator: matches the build server | channel or holon policy |
| What a hub retains | all 10 | operator | declared device purpose |
| What a personal device retains | current, plus one either side | operator | declared device purpose |
| What a conductor retains of superseded code | three sets of unreferenced WASM | this document, from the operator's "last three" | unchanged: this is a safety margin, not a trust question |
| Who may compel an upgrade or rollback | the release channel the peer follows | existing adoption controller | standing within the person's holons |
| How far back a rollback can be compelled | the 10 retained releases | follows from retention | follows from retention |
| What is never deleted | whatever the peer is running | this document | never replaced |

Two rows are estimates still owed a measurement before a story can assert
them: the per-head cost floor (4 KB) and the code floor per conductor
(150 MB), both in §2.

## 9. Proposed defaults per device archetype (proposal, not adopted)

From a read-only review on 2026-10-05. The archetype list is
`genesis/data/devices/devices.json` (15 ids, wired per person in
`deployments.json` as `deviceArchetype` and loaded by storage at boot).
Budgets are arithmetic on §2's unit costs, which are partly estimates, and
count the hApp class only.

| Archetype | hApp releases kept | Commons blob holder | Head plane | Conductor | Budget, MB |
|---|---|---|---|---|---|
| family node (base, extended), dedicated server, home NUC, Raspberry Pi 4 | 10 | always | full arc | yes | 317–348 |
| gaming desktop, recycled laptop | 3 (previous, current, next) | when plugged in and unmetered | zero arc | yes | ~233 |
| education chromebook | 3 | never; holds what its person uses | zero arc, reads through a hub | light | ~193 |
| 2019 Android phone | 0; app bundle previous/current/next | never | hosted | no | ≤25 |
| Kubernetes pod (256 MB) | 2 (previous, current) | never; drawn cache only | zero arc | yes | ≤223 |
| thin client, fob, camera, microphone array, sensor | 0 | never | none | no | 0 |

**Operator direction on this proposal (2026-10-05).**
- Archetype defaults are set to the ideal the trust and physics story calls
  for, not to what the substrate can do today. Where today's substrate cannot
  express the ideal, the story names the stand-in.
- The head plane should end as a gradient, not full-or-zero. Holochain
  sharding is the expected vehicle and does not exist yet. Until it does, "zero
  arc" in the table above is the stand-in for "a small arc sized to the
  device", and "full arc" for "the arc a hub is trusted to carry".
- The two further hubs the benchmark needs are enabled by resolving the
  inefficiency this document measures, not by a separate decision. The
  runaways are what has kept the fixture below the cast its stories need.

**What enabling a hub costs, today against the floor.** A fleet peer persists
about 6.6 GB today (6.0–6.6 GB conductor volume plus 0.3 GB storage, §3)
against 317–348 MB at the floor. Two more hubs are 13 GB today and 0.7 GB at
the floor. Proposed enabling condition, to be read from the §6 gauges on the
existing seven peers before a hub is unsuspended: the conductor code store is
flat across a coordinator swap, and no conductor volume grows at rest over
24 hours.

**The fixture cast does not meet the five-holder benchmark.** Of the seven
active peers, four are always-on hubs (adam, matthew, eve, gertrude), two are
laptops (jessica, susan) and one is a chromebook (james). Five holders is met
only while a laptop is plugged in; the ten-release window at five holders is
not met. Unsuspending one hub-class peer (pete, daniel or frank) meets both.
The three-peer household mesh cannot meet five by headcount.

**Conflicts found in the existing vocabulary.**
- The household mesh declares every peer a family node; `deployments.json`
  declares jessica a laptop and james a chromebook.
- On the fleet, jessica and james run full arc today; this proposal puts them
  at zero arc.
- The archetype design note and `devices.json` disagree on capability levels
  (NUC, family node), and on whether a phone or chromebook runs a conductor.
- Partial arc is not deployable: the arc factor takes only 0 or 1.

**Fields that do not exist.**
- Retention window and holder count per channel: nearest is
  `adoptionDiscipline` in rakia's release manifest schema. Proposed home:
  brit/rakia.
- A peer's device purpose and local release depth: nearest are `alwaysOn` and
  `canSteward` in `devices.json` and `[stewardship]` in the peer policy.
  Proposed home: storage's peer policy, defaulted from the archetype.
- A previous or next release slot: storage keeps exactly one
  `last-applied.happ`.
- Plugged-in and unmetered conditions; a holder target per object.
- Sizes for container images, binaries, npm packages and cargo crates.

## 10. What cannot be read yet

- Conductor database sizes and op counts per peer: no gauge. The 6 GB figure is
  a volume total.
- Per-peer volume bytes on the shared-filesystem node (matthew, jessica, james).
- Holder count per blob: no gauge, so the "five holders" claim itself is
  unobservable on the fleet.
- Row projection size on a fully seeded peer.
- The per-head floor (4 KB) and the code floor (150 MB) are estimates, not
  measurements.
