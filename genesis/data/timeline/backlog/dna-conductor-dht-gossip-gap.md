---
id: "backlog-dna-conductor-dht-gossip-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Entries authored on matthew's conductor never gossip to other conductors — the custody-convergence chain's last layer (Holochain DHT, not storage)"
slug: "dna-conductor-dht-gossip-gap"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, #1123 decisive read)"
status: "backlog"
priority: "high"
ci_status: red
jobs: [elohim-genesis]
tags: [dna, holochain, kitsune2, gossip, custody-convergence, epr-durability-arc, ceiling]
cites:
  - genesis/data/timeline/backlog/ci-substrate-commitment-projection-convergence.md
---

# Conductor-to-conductor DHT gossip gap

## Evidence (#1123 window, Loki)

adam's pod ran 2+ hours of 5-minute sweeps reporting
`ids_discovered=28, conductor_missing=28, healed=0` — libp2p inventory
showed him all 28 commitment CIDs, his LAMAD bridge was connected
(attempt 1), and his conductor returned NOTHING for any of them. The
entries authored on matthew's conductor are not present in adam's or
jessica's conductor DHT shards. Every storage-layer leg of the chain is
now proven green (#1123: mesh 7/7 incl. both adjacency directions; bytes
moved same-build, probe fetch 0s/1 attempt, fs 5→7); this gossip layer is
the LAST break behind propagation.custody-convergence.

Suspect: kitsune2 `Bootstrap overloaded, dropping put err="Full(..)"`
observed under sync load on matthew — DHT publish drops at the
bootstrap/gossip layer. Formation's `DepMissingFromDht` failures are the
same family (fresh entries not integrated/propagated when dependents
commit; seeder-side settle-retry landed as mitigation).

## Why ceiling

The fix surface is conductor/DNA-layer (kitsune2 config, gossip/publish
tuning, bootstrap capacity, possibly conductor version) — operator +
rust-architect investigation territory, not an overnight storage patch.
A wrong move here partitions DHTs (ALLOW_DNA_REINSTALL gotcha).

shift_objective: |
  Determine why DHT ops authored on one conductor don't reach sibling
  conductors (kitsune2 bootstrap overload? gossip arc config? network
  seed mismatch?), fix at the conductor layer, and prove it by
  propagation.custody-convergence going green with healed>0 or
  conductor_missing=0-with-local_total>0 on replicas.

## MECHANISM CANDIDATE (live, fresh Gen-3 pods ~18:50Z 06-11): broken conductor DHT DB

jessica's freshly-restarted conductor: `kitsune2_gossip: Failed to update
DHT "no such table: DhtOp"` — the gossip layer cannot WRITE ops because
the conductor database is missing the DhtOp table. A conductor whose
gossip store is schema-broken cannot propagate or integrate ops,
explaining both the entries-never-reach-siblings pattern AND formation's
DepMissingFromDht class. Prime suspect: the conductor data dir is a
persistent PVC — a holochain version change with a stale DB schema
(never-migrated/incompatible) leaves kitsune2 writing into a table that
doesn't exist. Sibling wrinkle, same window: matthew's lamad bridge
cycling at attempt 82 on `DepMissingFromDht(InitZomesComplete)` — cell
init itself failing the dep lookup.

Disposition: operator-gated — fixing means conductor DB migration or a
controlled reset (ALLOW_DNA_REINSTALL family: both genesis-pair pods
together or DHT partition). The diagnosis is now concrete enough for a
targeted operator session: check conductor holochain version vs PVC DB
schema; decide migrate-vs-reset; alpha keys need lineage care.
