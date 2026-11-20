---
name: project-alpha-conductor-pvc-standalone-claims
title: Alpha conductor data PVCs are standalone claims
description: holochain-data-elohim-<human>-alpha-0 is created by hand and used by name (adam Retain, matthew Delete) — never recommend deleting it; empty it in place
metadata:
  type: project
---

The alpha conductor StatefulSets mount `holochain-data-elohim-<human>-alpha-0` by claim name; the claims were created with `kubectl create`, there is no volumeClaimTemplate, and the shem-zfs PVs are `Retain`. Deleting a claim leaves the conductor pod Pending ("PVC not found") until someone recreates it by hand, and the retained dataset keeps consuming shem-zfs quota (which has run out before).

**Why:** 2026-11-08 re-genesis 1 — I asked the operator to `delete pvc` to reset adam's keystore; the operator-side agent caught the trap.

**How to apply:** to reset a conductor's keystore/data, recommend the in-place wipe: scale the conductor StatefulSet to 0, mount the claim in a helper pod on the claim's node, optionally tar `ks/` for forensics, empty the volume, delete the helper, scale back to 1. Cluster acts stay operator-owned. Related: [[project-adam-conductor-own-manifest]].
