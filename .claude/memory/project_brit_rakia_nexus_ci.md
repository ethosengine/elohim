---
index: false
id: project-brit-rakia-nexus-ci
name: project_brit_rakia_nexus_ci
title: brit/rakia Nexus CI wiring
description: rakia+brit crates publish to Nexus cargo-internal; hosted repo is auth-required so cargo needs a read token; committed paths override hard-errors in CI.
metadata: 
  node_type: memory
  type: project
  originSessionId: 4ae8aa4f-4ae1-4f6e-a6da-9a193f5a7f33
---

brit's standalone GitHub CI was red since April because `brit-cli` (the `rakia`
binary) path-dep'd a SEPARATE private repo `ethosengine/rakia` (`../../rakia/...`,
absent in CI). Fix shape (landed 2026-06-30 on branches `ci/standalone-fixes` in
brit + `ci/nexus-publish-pipeline` in rakia, operator integrates):

- **rakia gets its own GHA pipeline** (`.github/workflows/ci.yml` + `scripts/ci/cargo-publish-rakia.sh`,
  mirrors `elohim/epr/Jenkinsfile`'s publish) that publishes `rakia-core`/`rakia-brit`
  to the internal Nexus cargo registry (`elohim` = `cargo-internal`). Idempotent/409-safe.
  brit then consumes them as registry deps (`{ version="0.1", registry="elohim" }`),
  same as it already consumes `elohim-epr`. rakia 0.1.0 published 2026-06-30.

- **THE GOTCHA: `cargo-internal` is a Nexus HOSTED repo with `config.json` `auth-required:true`.**
  The cargo CLIENT refuses to read it without a credential EVEN THOUGH Nexus serves
  the content anonymously over raw HTTP (curl on the index → 200; missing crate → 404,
  never 401/403). And **hosted cargo repos have NO `requireAuthentication` knob** — that
  attribute exists only on cargo proxy/group repos (confirmed against the instance's own
  swagger `CargoAttributes`), so it can't be flipped in UI or REST (Nexus 3.89). The brit
  `.cargo/config.toml` comment "anonymous read resolves dependencies" was WRONG for cargo.
  Fix = give cargo a read credential: `CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer <NpmToken>"`
  (or a Basic `cargo-ci:pass` b64) + `CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS=cargo:token`.
  Wired into brit `ci.yml` top-level `env:` (reaches container/32-bit jobs too). Env
  `NPM_TOKEN` (ethosenginebot, cargo-deployer role) works for BOTH read and publish.

- **A committed `paths` override for rakia was TRIED and REMOVED — it breaks standalone CI.**
  `paths = ["../rakia/rakia-core", "../rakia/rakia-brit"]` in brit `.cargo/config.toml`
  was the original ci/standalone-fixes design for local monorepo path-dev. **THE CLAIM THAT
  "cargo silently ignores override entries whose dir is absent" IS FALSE** — cargo hard-ERRORS
  `failed to update path override … No such file or directory` when the dir is absent, failing
  EVERY cargo job. It only ever "worked" in the dev env (sibling `../rakia` present); the first
  real GitHub CI run (2026-07-01, after merge to brit main) went fully red (test-doc, journey,
  msrv, lint, cargo-deny, test-fast, test-32bit). Classic host-green≠CI-green. **FIX (commit
  1437f82fe3): removed the override entirely** — rakia is published, so brit resolves
  rakia-core/rakia-brit from the `elohim` registry everywhere (Cargo.lock already pinned the
  registry source; the override was only a build-time redirect). Local co-dev of rakia+brit
  needs a GITIGNORED override (e.g. a parent-dir `.cargo/config.toml`), never the committed one.

Remaining operator steps to green: provision brit secret `CARGO_REGISTRIES_ELOHIM_TOKEN`
(read token) + rakia secret `NEXUS_NPM_TOKEN` (for future pipeline republishes); integrate
both branches. Also fixed on the brit branch: cargo-machete unused deps (lint) and
persist-credentials:false on sync-upstream.yml (check-no-persist-credentials). Sibling:
[[project_brit_next_gen_epr_meta_foundation]].

**UPDATE 2026-07-01 — brit crates now PUBLISHED to Nexus too (reverse direction).**
Secrets provisioned: brit has `CARGO_REGISTRIES_ELOHIM_TOKEN` (read) + `NEXUS_NPM_TOKEN`
(write); rakia has `NEXUS_NPM_TOKEN`. The brit-publish set is **15 crates** (spec/plan
on branch `feat/publish-brit-crates`): 4 brit-authored @ **0.1.0** (`brit-epr`,
`brit-graph`, `brit-build-ref`, `brit-cli`) + 11 forked `gix-*` @ **upstream versions**
(the `gix-object` path-closure: gix-object 0.58.0, gix-hash, gix-actor, gix-date,
gix-error, gix-features, gix-hashtable, gix-path, gix-trace, gix-utils, gix-validate).
`brit-verify` stays `publish=false` — it's the ONLY crate path-dep'ing the FULL forked
`gix` (~67 crates); excluding it collapses the closure from ~72 to 15. Key mechanics:
(1) forked gix keeps upstream versions — the `cargo-internal` HOSTED registry is a
namespace distinct from the crates.io MIRROR, so forked `gix-object 0.58.0` @ elohim
never collides with upstream; **immutability** = a re-diverged already-published gix
version needs a bump. (2) Every intra-set dep carries dual `path` + `version`+`registry="elohim"`
(path wins local; registry drives published metadata — WITHOUT it cargo defaults the dep
to crates.io = silently upstream). brit-cli's DIRECT `gix="0.81"` stays on the mirror
(upstream) by design; dev-deps untouched (cargo strips path-only dev-deps at publish).
(3) `scripts/ci/cargo-publish-brit.sh` = idempotent/409-safe/topological/`--no-verify`
(the `test`+`lint` CI job is the build gate); `crate_version()` MUST use awk `match`/`substr`
(NOT `gsub` w/ `\1` — awk gsub has no backrefs). (4) brit CI publish job gated on push:main,
`needs:[test,lint]`, ADDED to `EXPECTED_NONBLOCKING_JOBS` (else the `check-blocking` gate
fails), and the step sets `CARGO_REGISTRIES_ELOHIM_TOKEN: ""` so the script derives the WRITE
Bearer from `NEXUS_NPM_TOKEN` (the workflow's top-level env exports a read-intent token to
all jobs; rakia has no such export so its job "just works"). ALL 15 verified live on the
sparse index; standalone-consumer proof passed (`cargo add brit-epr --registry elohim` +
build = EXIT 0, resolves brit-epr + gix-object from cargo-internal). Interdependent-set
gotcha: `cargo publish --dry-run` only works for leaf crates — dependents fail resolution
until their deps are actually live, so topo order at real publish is the only full validation.
