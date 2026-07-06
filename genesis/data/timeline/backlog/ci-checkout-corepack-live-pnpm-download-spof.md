---
title: CI Checkout stage does a live corepack pnpm download — network SPOF that flakes whole builds
status: open
ci_status: open
severity: medium
discovered: 2026-07-06
discovered_by: shift/overnight-genesis-pipeline-stabilize (iteration 2)
domain: ci-cd
pipelines: [elohim, elohim-genesis, elohim-edge (any job whose Checkout runs corepack)]
---

## What

The `Checkout` stage of the elohim app Jenkinsfile (and likely siblings) runs
`corepack enable` + `pnpm --version` inline, which triggers corepack to
**auto-download pnpm 10.30.3 from `registry.npmjs.org` on every build**. When the
Jenkins agent pod's egress to the Cloudflare-fronted npm registry blips, the
download fails (`ETIMEDOUT`/`ENETUNREACH` across all 12 edge IPs) and the whole
build dies at Checkout — surfacing as a confusing "Checkout failed / script
returned exit code 1" that looks like an SCM/submodule problem but is a network
flake.

## Evidence

- elohim/dev **#1591** FAILED at Checkout on the corepack pnpm-10.30.3 download
  (all IPv4/IPv6 Cloudflare edge addresses timed out). Quoted in
  ci-investigator run against #1591.
- **#1587–#1590** ran the identical corepack line and downloaded pnpm cleanly →
  single-build transient, not a code regression. Retrigger clears it.

## Why it matters

Checkout-time network egress to a public registry is a single point of failure
outside our control. Every build gambles on registry.npmjs.org reachability; a
blip reads as a red build and burns a CI cycle + triage time.

## Durable fix (candidate — NOT done)

Pin/prefetch pnpm **in the CI image** (ci-builder / ci-playwright) so no per-build
download occurs — bake the exact pnpm version into the image (corepack prepare +
cache, or a pinned global install), matching the `imagePullPolicy: Always` +
digest-pin discipline already used for toolchain drift. Image work lives in
che-devworkspaces (image infra), so this is a cross-repo follow-up, not a
genesis-pipeline change.

## Immediate mitigation

Retrigger the build (fresh dev push / `[build:*]` empty commit). ~80% of recent
builds cleared the download on the first try.
