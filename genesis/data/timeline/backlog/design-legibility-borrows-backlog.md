---
id: "backlog-design-legibility-borrows-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Design-legibility borrows backlog — survey-sourced devices for making quantities felt (Playnet / Free-Association)"
slug: "design-legibility-borrows-backlog"
written: "2026-08-05"
author: "claude (research mint pass, operator-directed clustering)"
status: "backlog"
priority: "medium"
tags: [design, graphos, legibility, dataviz, shefa, value-scanner, research-derived, cross-pollination]
cites:
  - genesis/research/playnet-free-association-cross-pollination-2026-08-05.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
---

# Design-legibility borrows backlog (research mint pass, 2026-08-05)

Externally-sourced **devices for making an economic quantity legible to an ordinary person**,
harvested from the [Playnet / Free-Association survey](epr:playnet-free-association-cross-pollination-2026-08-05).
Sibling of [measure-family-borrows](epr:measure-family-borrows-backlog) — that cluster decides *what
is measured*, this one decides *how it is seen*. Several rows are the render-half of a measure-half
there; those pairs are named per row and should land together or not at all.

The survey's grounding observation: Playnet's client renders **twelve faces as one instrument**, and
ours renders one dashboard fed by zero-filled views with 10 of 21 routes as
`ShefaPlaceholderComponent`. The gap is not effort — it is that they solved *coherence* once,
centrally, and we have not. **Fold new survey-sourced legibility borrows here — do not mint
siblings.**

| # | Borrow | Source + what it fixes | Gate/blocker | Owner shape |
|---|--------|------------------------|--------------|-------------|
| 1 | **Domain-keyed palette registry** — one hue per *domain concept*, resolved from a single registry, never per-component | [Playnet](epr:playnet-free-association-cross-pollination-2026-08-05) §3.8. It is the entire reason their faces read as one instrument: nourishment-coral is the same coral on the fragility wheel as on the plan. Direct countermeasure to all six dead-binding classes we have already catalogued (ghost names, inline hardcodes, `setProperty` clobber, inert kebab attrs). **Highest value-per-hour item in the survey**, ~1 day, and it touches nothing economic or political | None — pure design-system work | graphos-designer |
| 2 | **Soft/hard tension render primitive** — a limit drawn so that *bounded-and-trading* and *diverges-near-limit* are visually unmistakable | Playnet's load-bearing semantic distinction. Render-half of [measure-family](epr:measure-family-borrows-backlog) row 5 — **land together**: a hard limit must *look* unbuyable, which is what makes an ecological or dignity floor legible rather than merely enforced. Library A primitive (blank-slate, no brand) | Needs the measure-side semantics first | component-architect → graphos-designer |
| 3 | **Area-preserving surface with a conservation test** — cells whose *area* (never radius) encodes quantity, summing to the declared total, asserted in test | Playnet `EQ-2.6/2.7`. Render-half of [measure-family](epr:measure-family-borrows-backlog) row 2. The household analog is a care surface; the invariant is what makes it honest rather than merely suggestive. Must respect the Value Scanner's privacy boundary — a partial share renders as *withheld*, never as zero (honest absence) | Pairs with measure row 2; privacy boundary is a hard requirement | component-architect + shefa |
| 4 | **Show the trajectory, not only the state** | The survey's sharpest *negative* finding: **nothing in Playnet's 93 pages plots a quantity against time.** For a section titled *The living plan*, whose whole claim is continuous metabolisation, that absence is diagnostic — a system whose legitimacy rests on convergence published no convergence plot. Our surfaces inherit the same temptation. Rule to adopt: any surface claiming a system is *converging* must be able to show the path | None | graphos-designer |
| 5 | **Two-tone realized vs unrealized fill** — committed/expected and actual rendered in one mark | Playnet's plan/calendar faces distinguish declared, offered, eligible, and realized by fill and dash rather than by separate charts. Cheap, and it is exactly the distinction our commitment-vs-event views need | None | graphos-designer |
| 6 | **A headline metric that names its own binding lever** — e.g. `min(free-time, satisfaction)`, which tells you *which* half is currently limiting | Playnet's dial pairs `f` (free time) and `Φ` (satisfaction) so the binding constraint is readable at a glance rather than inferred. Prevents the single-number dashboard that says how you are doing but not what to do | Composes with measure row 7 (harmonic-mean aggregation) | graphos-designer + shefa |
| 7 | **A retirement discipline for superseded surfaces** | Convergent evidence, not a borrow: Playnet still serves a dark Scheme-REPL app from a previous pivot on the same origin, and their PWA manifest still describes it. Structurally identical to our shipped shefa dashboard, whose token/USD vocabulary predates the current framing. **Both projects ship a stale surface from an earlier vision because retiring it is nobody's job.** Needs an owner, not a critique | Needs an owner assigned | operator decision |

## Method note carried forward

Frontend review here is **eyes-first**: these rows were derived by *rendering* Playnet's client
headless (14 views) rather than by reading its source, which is how the shop face's live
`v_cons · f · worked` readout was found at all — it appears in no repository. Any row above should be
verified the same way before it is called done (`pnpm look`, graphos `sheet`).
