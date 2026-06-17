---
title: Decision-Record Discipline — tunable → measured-effect → verdict
kind: backlog
status: active
tags: [decision-record, discipline, observability, design-decision-toolkit]
occurred_at: 2026-06-17
---

# Decision-Record Discipline (design-decision toolkit P3)

The toolkit's instruments (P0 metrics surface, P1/P2 samplers) produce *evidence*. This discipline makes the **decisions** that evidence supports **durable** — so a settled design call (e.g. "arc is falsified", "the OOM is a conductor heap leak") is recorded once, with its evidence and the instrument that produced it, and is not re-litigated months later from memory.

It formalizes the RCA's "tune → document → report" loop (`matthew-edge-resiliency-rca-fanout-2026-06-15.md` §3): every tunable we turn, or hypothesis we test, gets a record with the measured effect and the verdict.

## When to write one
- A lever was turned and its effect measured (`target_arc_factor=0` → no memory bound).
- A hypothesis was confirmed or **falsified** by an instrument (anon-vs-cache split → heap leak).
- A design fork was decided with evidence (RAM bump rejected; cache cap rejected).
A decision-record is NOT a task or a plan — it's the *settled conclusion* + its proof. Living conclusions live as `backlog` entries; once fully settled and stable, the historian may graduate them to `chronicle`.

## The shape (copy this template)
```markdown
---
title: <Decision name — the conclusion, not the question>
kind: backlog
status: confirmed | falsified | proposed | superseded
tags: [decision-record, <domain>]
occurred_at: YYYY-MM-DD
---
# <Decision>

**Lever / hypothesis:** <the tunable turned, or the claim under test>
**Instrument:** <which toolkit instrument produced the evidence — the exact metric/Loki query, so it's reproducible>
**Measured effect:** <the numbers the instrument showed>
**Verdict:** <the conclusion — what it rules IN and what it rules OUT>
**Brake / action:** <the chosen response that follows from the verdict>
**Lineage:** <cites to the spec/plan/prior records this descends from>
```

Why these fields: **Instrument** makes the decision *reproducible* (re-run the query, get the same answer) — the antidote to "we think we decided X once". **Verdict** captures what's ruled OUT, which is what prevents re-litigation (you don't re-propose a RAM bump if the record says anon-leak). **Brake** links the decision to its consequence.

## Records to date (the first conformant instances)
| Decision | Lever/hypothesis | Instrument | Verdict | Record |
|---|---|---|---|---|
| Arc-shrink does not bound conductor memory | `network.target_arc_factor=0` (leecher) | jessica soak: `container_memory_working_set_bytes` sawtooth, arc=0 vs arc=1 | **falsified** — arc is the wrong lever; OOM is arc-independent | `arc-shrink-ineffective-memory-soak.md` |
| The conductor OOM is an anon-heap leak in the conductor child | "is it leak / page cache / corpus?" + "which process?" | cadvisor `container_memory_rss` vs `container_memory_cache`; the per-process + smaps memory-attribution sampler (`elohim_node_*` metrics / Loki `per-process rss split`) | **confirmed** — anon-heap leak; ruled OUT: page cache, corpus, arc, our storage code; located in the holochain conductor child | `conductor-memory-attribution-verdict.md` |
| Conductor anon leak is mmap-count accumulation (mechanism) | which §4 mechanism: H1 in-place buffer / H3 per-op mapping / H4 arena / H2 trigger | Loki `conductor anon smaps breakdown` line (heap & largest flat; `other_anon_bytes` + `anon_mapping_count` rising; threads ~flat) + cadvisor sawtooth | **confirmed** — many discrete large-`mmap` allocs accumulate; ruled OUT: in-place buffer (**H1 falsified**), arena slope (**H4 not-the-slope**), thread stacks; **H3 leading** (buffer unpinned), **H2 trigger proposed** | `conductor-anon-leak-mechanism-smaps-verdict.md` |

## Pending backfill
- [ ] Re-shape the two records above to carry the exact template frontmatter fields verbatim (they already carry the content; this is cosmetic alignment).
- [ ] As the RCA instrument suite (toolkit P2) lands more instruments, each tunable it measures (`db_max_readers`, doorway watchdog budget, saturation) gets a record here when turned.

## Lineage
- `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md` (P3)
- `.claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md` (§3 tune→document→report)
- `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md`
- `genesis/data/timeline/backlog/arc-shrink-ineffective-memory-soak.md`
