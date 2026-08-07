---
id: "backlog-susan-overnight-image-heal-miss-stale-layer-two-generations"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "susan's storage missed the 2026-08-07 overnight image heal — stale layer served across TWO pod generations while matthew self-cured in ~11min"
slug: "susan-overnight-image-heal-miss-stale-layer-two-generations"
written: "2026-08-07"
author: "hoot-owl integrator shift"
status: "open"
priority: "medium"
area: "dataplane"
domain: "cluster"
tags: [susan, matthew, iroh, image-heal, statefulset, containerd, relay_url, self-heal, cluster-manifest, operator-owned]
cites:
  - genesis/data/timeline/backlog/iroh-lane-bootstrap-publish-dark.md
  - genesis/manifests/habits.yaml
---

# susan's storage missed the overnight image heal — matthew self-cured, susan crash-looped ~13h

The relay_url-skew boot loop (the deserialize-fatal class named in
`iroh-lane-bootstrap-publish-dark.md` — `unknown field relay_url, expected one of
name, description, roles, allow_deferred_memproofs, bootstrap_url, signal_url`)
self-cured differently on two peers after the anchor tag repoint, and the
difference is itself the finding.

## What happened

- **matthew**: self-cured in ~11min via kubelet restart + `imagePullPolicy: Always`
  picking up the repointed anchor tag — the CI pull-policy discipline
  ([[feedback_ci_pull_policy_always_freshness]]) worked exactly as intended here.
- **susan**: crash-looped for **~13h** (23:40Z Aug 6 → 12:11Z Aug 7) across **TWO**
  pod generations — UID `87cc28a2` then, after a scheduling event recreated the pod,
  UID `c803d212` (restarts 52-53 at 12:11Z) — and the RECREATED pod **still pulled
  the stale image**. Only a third pod, UID `842fb3cb`, finally landed the
  compatible image (~12:11Z+) and cleared the loop.
- Evidence: Loki shows the byte-identical `unknown field relay_url` deserialize
  fatal at every restart boundary from 23:40Z Aug 6 through 12:11Z Aug 7 —
  same error text, same class, across all three pod UIDs until the third one
  finally got the healed image.

## Open question

Why did susan's node serve stale layers across TWO separate scheduling events
(the crash-loop restarts of `87cc28a2`/`c803d212`, AND the pod recreation that
produced `c803d212` itself) when `imagePullPolicy: Always` should have re-resolved
the tag on every pull? Candidates, none yet confirmed:

- node-local containerd image cache serving a cached layer despite `Always`
  (digest lookup succeeding against a stale cached manifest rather than
  re-pulling from the registry)
- digest-vs-tag resolution drift — if the registry served a new digest under the
  same mutable tag, a stale local digest resolution would short-circuit the pull
- StatefulSet update-strategy interaction — whether susan's STS pod
  (as opposed to matthew's) was actually re-scheduled onto a fresh image pull at
  all, vs. kubelet reusing a locally-cached layer across the crash-loop restarts
  AND across the pod-recreation boundary

## Class and ownership

**Cluster/manifest ceiling — operator-owned investigation.** Per CLAUDE.md, the
live cluster is not this repo's cleanup surface; node-local containerd cache
state and kubelet pull behavior are read via Jenkins MCP / cluster manifests, not
`kubectl` from the dev environment. The repo-side lead is comparing STS specs and
image references between matthew and susan in the alpha manifests
(`genesis/manifests/`) to check whether the two peers' manifests actually diverge
(different `imagePullPolicy`, different tag pins, different update strategy) or
whether the divergence is purely node-local runtime behavior with identical
manifests — that comparison is the next repo-side step before handing the
containerd-cache/digest-resolution question to the operator.
