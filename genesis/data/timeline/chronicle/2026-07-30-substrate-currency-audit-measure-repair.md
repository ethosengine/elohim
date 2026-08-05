---
id: "chronicle-substrate-currency-audit-measure-repair"
kind: chronicle
status: noted
date: 2026-07-30
ceremony: substrate-currency
surfaces_rewritten:
  - CLAUDE.md
  - .claude/skills/rea-economics/SKILL.md
diff_review_verdict: RED-resolved
coherence_verdict: YELLOW-resolved
next_topic_sampled: "add a new economic event type end-to-end (Rust view → generated TS → shefa surface → CI)"
---

## What changed

Phase 1's ranking was measuring itself. `substrate-currency-audit.py` resolved cited paths against
the repo root ONLY, so the per-crate convention (`src/routes/mod.rs` inside
`doorway/doorway-service/CLAUDE.md`) and context-relative prose (`p2p/mod.rs` under an
elohim-storage heading) both read as missing. That does not degrade gracefully — it **reorders**:
`rust-architect.md` ranked #1 with 52 findings and ~zero real path drift, and every one of
`doorway/doorway-service/CLAUDE.md`'s 23 findings (#2) was a correctly-cited path. Repairing the
measure took 586 path findings → 95 and 98 drifted surfaces → 51. Two further defects: line numbers
were computed on the frontmatter-stripped body so they never matched the file, and the
process-status regex flagged `in-flight` as a proper noun ("the in-flight hook") and flagged lines
that **quote** the anti-pattern to teach it — making the discipline's own statement of itself its
top-ranked violation.

Rewrote the real top-2, both to 0 drift. **rea-economics/SKILL.md** (librarian-lens driver): its
Key Files table was substantially stale — REA models had graduated from the `shefa` pillar to
`elohim`, two services were renamed to the `*-api.service.ts` convention, and three doc rows
pointed at files that no longer exist. **root CLAUDE.md** (cartographer-lens driver): the CI table
claimed a Mishpat pipeline that does not exist — Mishpat *is* built, by the single DNA Jenkinsfile —
and pointed Steward at `steward/Jenkinsfile` instead of `steward/device/`.

Also found and defused a live data-loss trap: the root gospel's **package authority was stale
relative to its own projection**. Three lessons had been hand-written into `CLAUDE.md` and never
planted back (steward/node bin-only gate, "nextest is NOT installed", `cargo check` does not verify
a dependency bump). `replant.mjs`'s header names this as a tolerated baseline red, and the remedy it
prints (`project --write-runtime`) would have overwritten `CLAUDE.md` from the stale package and
destroyed all three. Planted live→package first, then fixed, then replanted: projection verify went
2-failed → **1113 passed, 0 failed**. Swept all 82 packages for the same divergence — this was the
only real one.

## Verification sampling (both Phase-4b lenses)

**Lens 1 — diff-regression review: RED, five confirmed defects, four in the resolver I had just
written.** Deciding "not a path" *before* attempting resolution was the root error: a blanket
leading-slash exclusion silently dropped genuinely-broken paths like
`/app/elohim-app/src/gone.ts` (and made the protocol-string and slash-command rules dead code),
and an all-lowercase vocabulary regex made `elohim/holochain/dna/bogus-dir` — the corpus's most
common citation style — entirely unauditable. Also: negation-suppression in the quoted-example
detector was line-wide, so "not finalized, and … currently under dev" suppressed a real finding.
All fixed by moving the non-filesystem-shape decision *after* resolution, guarding the vocabulary
rule with a two-segment reality test, and clause-scoping the negation window; each fix
reproduced-then-re-tested. The fifth defect was mine too: I deleted a research row whose content
is alive after two renames at
`elohim/elohim-storage/research/economic-systems-research.md` — restored, repointed. The lens's
one overreach: it called interior suffix-index fragments a bug; they are true suffixes of real
ancestor directories, which is the documented (and deliberate) permissiveness.

**Lens 2 — fresh-context coherence: YELLOW.** Confirmed the CI table clean, and correctly attacked
my own rewrite: framing the `elohim` pillar as where "REA vocabulary lives" invites reading
prototype-era TS mirrors as source of truth, when `elohim/sdk/domains/shefa/` owns the wire types
and `rea-bridge.model.ts` self-describes as prototype contracts. Reframed to name the substrate as
authoritative. Its mechanical findings — two dead service class names, a 7-of-24 REA action subset,
a stale `app/` prefix — closed inline. Its load-bearing gap (three coexisting type-generation
pipelines, none of the priming surfaces naming all three; plus `REA_ACTIONS` having no governing
schema) became one backlog entry rather than a rushed fix.

## Wisdom worth carrying forward

A drift audit can be the dominant source of its own findings, and the failure is not noise — it is
**reordering**, which puts the surface the tool misunderstands at the top and hides the drifted one
below. Spot-check the #1 surface against disk before letting any ranking drive work. The
corollary, learned the hard way in the same cycle: when repairing such a resolver, every
suppression must be applied *after* a resolution attempt, never before — and prefer
under-suppressing, because a residual false positive costs one check while a suppressed real one
goes unfixed. Recorded as [[feedback_verify_the_measure_before_the_ranking]].

Phase 4b earned its keep twice over: both lenses found real defects in work that had already been
self-reviewed, and Lens 1's RED was entirely about the repair rather than the rewrites. A ceremony
that fixes its own instrument must verify the instrument as adversarially as the surfaces.

## Horizon-scan reference

- **Latest scan**: `.claude/memory-kit/horizon-scans/2026-05-14.md`
- **Next recommended scan**: 2026-08-12 (90 days from `scanned_at`) — still current at this
  ceremony, so cartographer did not re-scan. The next ceremony after that date should.
