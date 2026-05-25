# Elohim Protocol Architecture Canon

This directory holds cross-cutting principles that inform every spec and implementation in the protocol. Where specs are concrete designs ("how do we project content through doorway?") and plans are implementation work ("write this code task-by-task"), the canon is the foundational philosophy: what we mean by stewardship, by authority, by recovery, by capability.

Every spec author should read the canon before designing identity, authority, recovery, or capability surfaces. Every implementer should read the canon before touching cryptographic primitives, key custody, or access gates.

## Canon documents

Canon documents use `epr:` URIs for cross-references (the protocol's content-addressing convention). In an EPR-aware client these resolve to live content; on GitHub the link text is still readable.

| Document (EPR) | Filesystem | What it codifies |
| --- | --- | --- |
| [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) | `stewardship-over-sovereignty.md` | The protocol's foundational philosophical inversion. Why "hold your own keys" is not the protocol's notion of sovereignty. What stewardship, agency, and authority mean here. The grandma standard. |
| [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) | `rea-compute-commitment-primitive.md` | The substrate primitive (`Mishpat::Commitment` with `delegates-compute` action). One shape, instantiated across deploy / hosting / household chores / qahal moderation / content authorship / DePIN compute / recovery quorum. |
| [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) | `cradle-to-grave-capability-gradient.md` | Life-stage capacity transitions (child, adolescent, adult, senior, end-of-life). Graduated recovery authority (4-layer stack). Elohim-agent mediation (specialists + counsel + co-steward). |
| [content-pipeline](epr:content-pipeline) | `content-pipeline.md` | The content seeding pipeline (genesis → seed-data → DHT). |

## Relationship to other docs

- **Canon** (this directory) — cross-cutting principles, citeable from any spec.
- **Specs** (`genesis/docs/superpowers/specs/`) — concrete designs of features and subsystems, citing canon.
- **Plans** (`genesis/docs/superpowers/plans/`) — bite-sized implementation tasks, citing specs.
- **Memory** (`.claude/memory/`) — agent-side condensed forms of canon for fast recall during work.

## Reading order for new contributors

If you are new to the protocol, read in this order:

1. **[stewardship-over-sovereignty](epr:stewardship-over-sovereignty)** — the foundational lens. Everything else assumes this.
2. **[rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive)** — the substrate primitive you will use whenever designing an authority delegation surface.
3. **[cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient)** — how the primitive instantiates across human life-stage capacities. Why the protocol must serve a child, a grandmother, and a recovering-from-cognitive-decline elder with the same fundamental shape.

After the canon, read the specs that touch your domain. The canon will have already given you the vocabulary; the specs apply it.

## Citing canon from specs

When a spec depends on a canon principle, cite the canon doc explicitly:

```markdown
## §0 — Canon References

This spec depends on:
- [`stewardship-over-sovereignty.md`](../../architecture/stewardship-over-sovereignty.md) — for the no-anonymous-publish rule.
- [`rea-compute-commitment-primitive.md`](../../architecture/rea-compute-commitment-primitive.md) — for the `bounded_by` back-reference pattern.
```

This makes the dependency graph between concrete designs and foundational principles legible to future contributors.

## Why "canon" and not "docs" or "principles"

Canon is the strongest word for "this is the foundational text." It signals: these documents are load-bearing, mutually consistent, and resistant to change without deliberate community process. Updating canon is a meta-change to the protocol's identity; updating a spec is a normal design iteration.

If you find yourself wanting to update canon, first ask: am I extending the existing canon (adding a new doc), or am I changing what the canon says? The former is normal; the latter requires the same care as a protocol-level governance act.
