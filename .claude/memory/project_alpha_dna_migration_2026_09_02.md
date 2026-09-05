---
name: project_alpha_dna_migration_2026_09_02
title: Alpha DNA re-genesis 2026-09-02
description: 2026-09-02 alpha re-genesis — installed DNAs predated every CI build since Jul 18; drift probe's reinstall tore the source chains; operator chose migration (baseline = holochain #1420 hashes, [dna:migrate]); DNA Hash Guard + happ_manager intent gate now stand
metadata:
  type: project
---

**What happened (2026-09-02).** The alpha fleet's installed lamad/imagodei/mishpat DNAs matched no CI
build since at least 2026-07-18 (archived-artifact walk, `hc dna hash` per build #1363..#1420): six
weeks of integrity work shipped but never installed ("DNA changes don't redeploy by default").
On the wave-4 roll the drift probe finally fired under the standing `ALLOW_DNA_REINSTALL=true`
(all 14 alpha StatefulSets), `uninstall_app` deleted the five source chains and tore on a 30 s DB
lock → every conductor panics `CellWithoutGenesis` 2.3 s into boot. Chains irrecoverable; nodes
re-key. The same day's build-script commit (03f331f21) moved every hash too — bounded, fixed by
removing the build.rs files (189061c6d; #1420 == #1414) — but it was a red herring for the fleet.

**Decision (operator, 16:1xZ): MIGRATE.** `elohim/holochain/dna/dna-hashes.baseline` = #1420's
hashes, committed `[dna:migrate]` (7d9096263). Fleet steps are operator-owned and SEQUENCED:
clear each conductor's full `databases/` tree ONLY AFTER the edge roll that carries #1420's hApp
and the new storage image — clearing earlier makes old storage install the incident hashes and
re-key twice. `DNA_MIGRATION_INTENT=<five hashes>` is needed only if cleared early.

**What now stands.** DNA Hash Guard stage (`scripts/ci/dna-hash-guard.sh`, baseline file, prints
`DNA-HASH <role> <hash>` per DNA; `[dna:migrate]` moves it). `happ_manager` (867e4bf9b): drift or
structural staleness on a node holding data never reinstalls without `DNA_MIGRATION_INTENT`;
`FORCE_DNA_REINSTALL=wipe` is the one destructive spelling; a torn uninstall is terminal.
`process_manager` (264ce8ce4): a dead child is reported with exit status + stderr tail at once.

**Why:** a standing flag armed a non-atomic destructive path on nodes holding data; the guard +
intent gate make both a silent hash move and a silent reinstall impossible.
**How to apply:** any integrity change → `[dna:migrate]` + baseline update in the same commit + a
planned re-genesis; `RESET_STORAGE` (genesis pipeline) deletes storage's content.db only, never the
conductor's PVC (separate StatefulSet since 9c9f9fc65) and its script still names pre-split
pods/containers. Escalation atom: `genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md`. See [[project_pipeline_dispatch_ordering]], [[project_adoption_ceremony_mesh_traps]].

**CORRECTION (21:45Z, k8s dev, on-cluster).** There was NO migration. `hc dna hash` (packed) ≠ the
conductor's installed DnaHash, which folds in the happ.yaml role modifiers (`properties:
{progenitor_pubkey: null}` on four roles; infrastructure `properties: null` — the only role where they
coincide). The "installed DNAs predate every CI build since July 18" finding was a packed-vs-installed
comparison and is retracted; #1420 == #1414 (packed) and the re-genesised nodes report installed lamad
`uhC0kkLdC…` = the hash they always ran, with `No coordinator-zome drift`. The incident was ONLY the
03f331f21 byte move (#1416) + the standing ALLOW_DNA_REINSTALL=true. My full-`databases/` clear
instruction discarded ~2.5 GB/node of DHT data that belonged to the SAME DNAs (conductor/ + ks/ alone
would have kept it) → a full re-seed is now required. The `dna-hashes.baseline` contract is PACKED
hashes only (its header now says so); the [dna:migrate] label on 7d9096263 was wrong. Also: a torn
conductor on the pre-fix image reads Ready (supervisor logs ready over a dead child; HTTP probe
passes) — "Ready" is not health until 264ce8ce4 is on the conductor pods.

**Re-seed ordering (operator decision, 0.7 guide §Why now + 2026-09-03 wipe authorization):** the
alpha re-seed after the conductor clear WAITS for the Holochain 0.7 cutover — the whole fleet is
wiped clean for 0.7 (one genesis, one re-seed); a 0.6 re-seed is churn. An untagged empty tip
(6c215786b) neutralizes the local `[build:genesis]` commit; the 0.7 cutover push (elohim-db's
`upgrade/holochain-0.7`, rebased on local dev) carries the queued commits and the seed.
