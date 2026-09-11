---
index: false
id: project-host-psu-power-interrupts-2026-09
name: project-host-psu-power-interrupts-2026-09
title: "project-host-psu-power-interrupts-2026-09"
description: Sept 2026 — the dev host loses power mid-session (PSU fault; 750 W replacement ordered 2026-09-07); every mesh, cargo gate, push and background agent dies at once, /tmp is wiped, /projects survives
metadata:
  type: project
---

The dev host suffers abrupt power interrupts (operator 2026-09-07: "I've ordered a replacement 750w PSU to help fix these disruptive interrupts"). One hit at ~21:50 UTC on 2026-09-07 killed the household mesh, a running pre-push gate, and every background agent and watch in one stroke; `/tmp` (mesh dirs, session task outputs) was gone, `/projects` (worktrees, cargo pool, job logs under /projects/.claude-config/jobs/) survived, and `berth` leases stayed held by dead agents.

**Why:** a whole-host restart is not the same as the devspace container recycle in [[project_devspace_recovery]] — nothing survives except disk, and the interrupt can land mid-gate.

**How to apply:** until the PSU is replaced, run every long leg re-entrant from durable state: logs under /projects/.claude-config/jobs/<id>/tmp, commits early and path-limited, `berth release` stale leases first thing after a restart (`berth status` shows holders that no longer exist), then re-run the push and re-dispatch mesh work fresh. Treat a sudden RAM-guard "committed 1.7G" at session start as the fingerprint. See [[feedback_agent_fleet_and_harness]].
