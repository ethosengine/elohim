---
title: hc:seed reads ../../scripts/local-dev/.hc_ports — a path hc-start.sh never writes
created: 2026-06-10
domain: process-meta (build-and-test; dev stack)
source: arc plan Task 2.1 spec review (2026-06-10)
severity: low
---

`app/elohim-app/package.json:41` (`hc:seed`) greps `../../scripts/local-dev/.hc_ports`
while `hc-start.sh` writes the ports file at `elohim/holochain/local-dev/.hc_ports`
(line ~280). Pre-existing drift — the seed script is reading a path that is never
written, so whatever fallback it takes is the de-facto behavior. Reconcile to one
path (the hc-start location is canonical; storage-start.sh:101 and justfile:25
already read it). Sibling of `hc-start-storage-dir-dead-override.md`.

UPDATE 2026-06-10 (papercut sweep, commit 87230a459): `hc:seed` fixed — now reads
the canonical `elohim/holochain/local-dev/.hc_ports`. RESIDUAL dead paths in the
same package.json, still pointing at `../../scripts/local-dev/`: `doorway:start`
(line 42, with a `|| echo 4444` fallback masking the miss), `hc:stop` (line 36)
and `hc:reset` (line 37) cleaning state files at the dead location. Same one-line
fix shape each; `doorway:start` also references a doorway binary path worth
verifying (`../../doorway/doorway-service/target/release/doorway` — see the
deprecation triage's sibling note on stale doorway paths).
