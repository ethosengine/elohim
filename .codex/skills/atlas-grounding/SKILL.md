---
name: atlas-grounding
description: Use when starting a planning or brainstorm pass on a feature or subsystem (or grounding external prior-art) and you need current ground-truth across the architecture's relevant seams before designing — "we're exploring X, ground my understanding", "map the seams for X", or as the grounding step before /brainstorm, /shift, or app-port.
metadata:
  runtime: codex
  sourceRuntime: claude
  sourcePath: .claude/skills/atlas-grounding/SKILL.md
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/atlas-grounding"
---

# Atlas Grounding

## Overview

Route a subject — an internal feature OR external prior-art — into the architecture's seam-space using the concern-routing atlas, fan out a read-only grounding agent per relevant seam, and synthesize a seam-map slice + grounded briefs. **The atlas is the map; parallel grounding agents are the method.** This is the agentic-architect search primitive that `app-port` composes for prior art.

## When to Use

- Before a `/brainstorm` or `/shift` on anything spanning more than one seam.
- "We're exploring X" — you need which seams X touches AND their current reality.
- As `app-port`'s grounding step over a prior-art repo + our substrate.

Not for: a single-file lookup (just read it); a concern that clearly lives in one known seam (go straight there).

## The Atlas (the map)

`genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md` — the seam catalog (§3), the concern-routing table (§4), the device spectrum (§2). The atlas is durable/positive; **current build-state lives in the dated assessments in `.claude/data/`** (linked from atlas §6) — read those for "what's wired now."

## Method

1. **Decompose, then locate.** First split the subject into *atomic concerns* — one clause ≈ one concern. The atlas §4 routing table is keyed by single concerns, so a multi-clause subject ("syncs across devices AND survives offline AND stays private") must be broken down, or you'll match only the loudest concern and silently miss the rest. Route *each* concern through the §4 table + §3 seam catalog. List every seam touched (composition-stack + role seams + device rungs), and **use each seam's "confusion-to-avoid" note (§3) while routing — to reject misroutes and bound the seam count** (a durability concern wearing a hub costume; a dataplane fact read as a doorway one). **Name your exclusions** and why.
2. **Fan out** one read-only grounding agent per seam. Each grounds *current reality*: what exists (file:line, from the seam's §3 Home) and its **build-state** — the dated assessment if the seam has one (atlas §6), **else fall back to the seam's §3 Home source files** and separate wired-vs-spec directly. §6 is NOT total — confirm each routed seam's build-state source before fanning out. Use **prose** returns, not schemas (schema'd agents hang on empty payloads).
3. **Verify** the load-bearing claims adversarially — separate wired from spec-only — especially for build-state.
4. **Synthesize** a seam-map slice (the subject's seams + how they compose, with the cross-seam couplings) + per-seam grounded briefs.
5. **Hand off** to `/brainstorm`, `/shift`, or `app-port` with the briefs as the grounded backdrop.

For ≥3 seams, run the fan-out as a Workflow (parallel ground → adversarial-verify pipeline → synthesis).

## Quick Reference — the seams (atlas §3)

hardware · OS/packaging · runtime/footprint · mod/plugin · SDK grammar · bridge · app-manifest/domain · client · doorway projection · peer-hoster dataplane · aggregation · hub cluster ops · confidentiality · temporal · resource-governance.
Tracks: **T1** DHT-notary · **T2** substrate · **T3** spoke · **T4** doorway.

## Composition

- `app-port` runs this on external prior-art (+ our seams) before decomposing.
- Reads the **dated assessments** for current build-state (unlike `concept-mapping` / `app-port`, which read the durable atlas).
- `concept-mapping` translates any conventional concept the grounding surfaces into its elohim seam + placement.

## Common Mistakes

- **Matching the whole subject to one §4 row** — decompose into atomic concerns FIRST, or you'll catch sync and miss privacy + custody.
- Grounding from memory or source-reasoning instead of dispatching agents that read current reality.
- Routing to too many seams — use the §3 confusion-to-avoid notes to reject misroutes; name exclusions.
- Assuming atlas §6 lists every seam's build-state — it doesn't; for an unlisted seam, ground from its §3 Home source files.
- Schema'd grounding agents — they loop on empty payloads; use prose returns + a synthesis pass.
