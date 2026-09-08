---
id: "backlog-doorway-test-surface-hosting-attestation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway test surface — an EPR-app publisher stages a deliverability probe as a delegated compute task the doorway operator's peer runs THROUGH its own doorway under a hosting commitment; the signed receipt is the deliverability attestation the doorway requires before serving a head as current (operator thought 2026-09-08; protocol-native successor of the CI boot gate)"
slug: "doorway-test-surface-hosting-attestation"
written: "2026-09-08"
author: "operator (via the deliverability session 2026-09-08)"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
  - "habit:operator-runtime-surface"
tags: [doorway, deliverability, delegated-compute, hosting-contract, rea, review, next]
---

**The thought (operator, 2026-09-08):** like sweettests are tests run by a peer, every doorway could carry a
test surface (think a SonarQube-shaped instance a review flow or IDE plugin hooks into) where a publisher
stages an integration test as a compute envelope against that doorway, as part of reviewing the hosting REA
contract between a peer and a doorway operator.

**Composition, nothing new minted:** T10 (`2026-09-08-peer-executed-stage-design.md`) already runs an a2o
feature as a delegated compute task attested by the provider (grant → accept → run → signed receipt → review;
measured on the household mesh 2026-09-07, receipt `genesis/a2o/reports/delegated-compute/mesh-20260907T094431Z/`).
This item is the same envelope with the roles turned: task kind `deliverability`, input = (EPR slug, browser
head, doorway id, probe executable), provider = the doorway operator's peer under a `hosting` mishpat
commitment (publisher ⇄ operator), execution = the probe run THROUGH the operator's doorway (today:
`scripts/ci/verify-served-shell.sh`; next: the headless-browser boot from `served-shell-boots.feature`), output
= a signed receipt binding (head, doorway id, verdict, reason). The doorway's bundle-heads reconciler
(spec `2026-09-08-epr-app-deliverability-through-doorway.md` D1/D2) then treats a head as `at-head` only when it
carries such an attestation from THIS doorway — the hosting review gates the shell, not a pipeline.

**Gate before this ships:** the CI floor from the same spec is green on the fleet (D4b/D4c hard gates), so the
peer-run version replaces a working gate rather than a missing one. Design gate: the p2p-design-gate
questions for the attestation record (Category A or B2? — it is an attestation about a served head by a named
doorway, so B2-shaped: private to the pair, attested) must be answered before any entry type is added.
