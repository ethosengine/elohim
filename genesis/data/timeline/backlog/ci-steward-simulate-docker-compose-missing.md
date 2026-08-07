---
id: "backlog-ci-steward-simulate-docker-compose-missing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steward simulate.sh P2P simulation leg 127s in CI — docker-compose absent from ci-builder"
slug: "ci-steward-simulate-docker-compose-missing"
written: "2026-08-07"
author: "hoot-owl integrator shift"
status: "open"
priority: "low"
area: "ci"
domain: "code"
tags: [ci, steward, docker-compose, edge-pipeline]
---

# steward simulate.sh needs docker-compose the CI image doesn't ship

Observed edge #1314 (2026-08-07, first edge build in a while whose changeset touched
steward/node): `./simulate.sh test` → `./simulate.sh: line 44: docker-compose: command
not found` → `ERROR: script returned exit code 127`. The stage is tolerated (build
continued to Build Storage / Deploy), so this is a silent no-op leg, not a red — but the
P2P simulation it's supposed to run has been provably not running in CI.

Options when picked up: (a) install docker-compose (or switch the script to `docker
compose` v2 syntax if the image ships the plugin) in ci-builder; (b) gate the leg on
tool presence with an explicit SKIPPED banner so absence is visible; (c) retire the leg
if the sweettest/a2o layers supersede it. Decide against what simulate.sh actually
proves today.
