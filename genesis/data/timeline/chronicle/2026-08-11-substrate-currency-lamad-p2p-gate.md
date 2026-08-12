---
id: "chronicle-substrate-currency-lamad-p2p-gate"
kind: chronicle
status: noted
date: 2026-08-11
ceremony: substrate-currency
surfaces_rewritten:
  - elohim/sdk/domains/lamad/CLAUDE.md
  - .claude/skills/p2p-design-gate/SKILL.md
  - CLAUDE.md
diff_review_verdict: RED (resolved)
coherence_verdict: RED (resolved)
next_topic_sampled: adding a lamad content type + renderer; designing a PeerEndorsement entity
---

## What changed

`elohim/sdk/domains/lamad/CLAUDE.md` rewritten — the librarian's not-found set was the driver: the lamad pillar was extracted to a standalone app at `app/lamad/`, falsifying every path claim. The cartographer surfaced seven undocumented substrate concerns (the `types/` wire-crate lane, gates-as-closed-world, the gate-process step-graph family, `attestations` retiring entry types, `routeClaims`), the historian six missing canonical-discipline citations on a file carrying zero. The storyteller's synthesis resolved the tension between "document more" and "stop transcribing" with the rewrite's organizing principle: **document the pattern and the consequence; point at code as read-canon for every enumerable list** — generalizing the one original claim (`see OUTPUT_DIRS in scripts/codegen.mjs`) that never went stale. Deliberately did not replace stale counts with fresh counts.

`.claude/skills/p2p-design-gate/SKILL.md` rewritten — the storyteller's scarcity-inversion finding was the driver. The gate's only quantitative branch asked "is there DNA headroom (~73/~100)"; the numerators were stale and **the ~100 ceiling was sourced to nothing** (real: `EntryDefIndex(pub u8)` = 256/zome). Naively correcting it would have flipped guidance from reuse-because-crowded to create-because-roomy, so headroom was removed as a branch condition entirely and demoted to a footnote that refuses to print a tally. Categories gained self-describing names; a back-fill detector replaced three repetitions of "do not skip ahead."

Root `CLAUDE.md` §P2P Design Gate corrected — its condensed gate taught the headroom heuristic the rewritten skill now classifies as an anti-pattern, and it is always in context while the skill is deferred-loaded, so the anti-pattern primed first.

## Verification sampling (both Phase-4b lenses)

**Diff-review (`/code-review` lens)** went RED on both surfaces and earned it. On lamad it caught a dangerous inversion: the rewrite claimed three attestation kinds replaced DNA entry types, but only `ContentAttestation` was retired — `ContentSuccession` and `CustodianCommitment` remain live with 17 call sites and explicit `KEPT` comments. On p2p it caught a **fabricated worked example** (`create_stewardship_allocation` exists in no zome); the replacement states the verified truth, which is more instructive — that entity is classified DHT-notarized but written by a direct HTTP→SQLite path with `dht_anchor_hash` permanently NULL.

**Coherence (fresh-context Explore)** went RED on both. It confirmed the gate does interrupt the reflex (it killed a UUID PK, a route-first design, and a notarized reputation score before they were written, and the head-plane budget changed a classification from entry to link) — but found the *primed set* incoherent: root gospel instructing the anti-pattern, a fictional zome name derivable from the template, and two required output fields unfillable without violating another gospel file. All in-scope findings were fixed and re-verified; cross-file findings are recorded below.

Both surfaces re-verified after fixes. Two rounds of fixes were needed on each — the first round of fixes on both surfaces introduced a fresh contradiction with an untouched passage, caught only by re-verification.

## Wisdom worth carrying forward

**A rewrite's own fixes need a second verification pass.** On both surfaces, round-one fixes contradicted untouched sibling passages: lamad's corrected codegen-lane prose sat above a products table still crediting the wrong script, and p2p's split classification test contradicted a flowchart that still gated on the old logic. A fix changes one passage; coherence is a property of the whole file.

**Two lenses disagreed, and the tie had to be broken against source.** On whether `route-claims.ts` is a `lamad:codegen` product, the diff-review said yes (it read the file's existence as proof of its producer) and the coherence read said no. The producing script contains no such string. Verify producers by reading the producer, not the artifact.

**A number with no source is worse than a missing number.** The `~100` entry-type ceiling appeared in at least three documents, propagating as fact for long enough that a design gate branched on it. Nothing anywhere sourced it. When a gospel surface states a limit, it should cite where the limit is enforced.
