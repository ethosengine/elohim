---
id: "backlog-ci-as-steward-channel-participant"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Jenkins as a participant in the steward channel: build verdicts as crossing events, dispatch under the jenkins-dispatch lease"
slug: "ci-as-steward-channel-participant"
written: "2026-10-23"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: "ci / berth / steward channel"
domain: "D5"
relatedNodeIds:
  - "habit:devices-share-one-berth"
tags: [ci, jenkins, berth, steward-channel, complementary]
---

# Jenkins as a steward-channel participant

Complementary to `workspaces-cooperate-as-devices-plan` (spec §3.6). That plan makes `jenkins-dispatch`
a steward-scoped berth resource that a pushing device claims. Jenkins itself still learns only from
the push. This item makes it a reader and a writer of the channel:

- The orchestrator writes each build verdict (job, build, commit, result, stage) as a crossing event,
  so a workspace's `berth status` shows what CI made of its landing without polling Jenkins.
- The orchestrator honours a held `jenkins-dispatch` (or `alpha-fleet`) lease from another device
  before a `[build:edge]` deploy, instead of relying on the pusher's hook alone.

Open question: Jenkins pods are not devices of a human. Does a CI runner enter the roster as a
delegate (the ci-runner-stewarded story's second steward), or does it write through a device's agent?
Answer it through the p2p-design-gate before building anything.
