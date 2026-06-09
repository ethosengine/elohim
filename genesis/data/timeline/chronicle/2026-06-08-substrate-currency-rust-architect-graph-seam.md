---
kind: chronicle
status: noted
date: 2026-06-08
ceremony: substrate-currency
surfaces_rewritten:
  - .claude/agents/rust-architect.md
  - .claude/agents/librarian.md
coherence_verdict: GREEN
next_topic_sampled: elohim-storage content-graph computed-edge discovery signal (extending ContentGraphResolver)
---

## What changed

`rust-architect.md` rewritten — the **cartographer's substrate-coverage gap** on the content-graph seam was the dominant driver: the graph section described only the Cozo `graph/engine.rs` EPR-projection engine and was silent on the native `graph_engine.rs` `ContentGraphResolver` trait seam the current branch (`feat/native-content-graph-seam`) introduced. The rewrite now disambiguates the **two distinct graphs** (Cozo owns EPR-projection only; native owns content↔content), re-seats the truth-layer decision at the read-only trait seam (graph.json = model, resolver = one engine over it, native today / Cozo-or-datalog future behind the same trait), and carries the Category-A/Category-C edge distinction, the no-write-by-design offline invariant, the MASTERY_OF whitelist exclusion, RELATES_TO vocab-drift caution, and the ContentGraph ts-rs view. The **storyteller** caught a surprise closed libp2p 0.53→0.54 skew (both crates now 0.54); the **historian** supplied 6 materialized-citation gaps (incl. re-pointing the two dead Cozo graph cites and three dead git-discipline cites to their materialized successors). `librarian.md` rewritten light-touch — operationally current, so only the L303 "now operational" process-status fix, two managed-surface hooks added to the hook list, `focus-baseline.py` added to the budget tier, and the flag→agent→canon→stasis pattern named; historian/cartographer converged on the managed-surface-edit and automation-pattern citations.

## Coherence-check sampling

Sampled topic: a downstream sprint adding a second Category-C computed-edge discovery signal to the `ContentGraphResolver` seam. Fresh-context Explore agent rated **GREEN** — the rewritten rust-architect.md graph section agrees with `project_content_graph_native_rust_not_cozo_apollo` and the `2026-06-08-native-content-graph-seam-design` spec on all five load-bearing claims (Cozo-EPR-only / native-content, read-only-no-write, Category-A/C, MASTERY_OF exclusion, RELATES_TO intersection); all eight newly-added cites materialized; no stale citations introduced.

## Wisdom worth carrying forward

The substrate-currency audit's raw drift ranking was badly polluted by **relative-path false-positives**: `doorway/doorway-service/CLAUDE.md` ranked #5 at 19 findings but every flagged `src/routes/*.rs` exists — the audit just couldn't resolve `src/` against the subdir. The real high-value pick (rust-architect.md) had to be found by checking *which* drift was substantive, not by the count. Separately, the librarian prologue surfaced a **catalog-wide dead-`[[slug]]`-citation cluster** (42 in rust-architect, 12 in librarian, ~4 propagating across the memory-team catalog) — confirmed genuinely-unmaterialized forward-links (no `name:` frontmatter anywhere; materialized store is exactly the 42-file `.claude/memory/` set). Per memory convention an unresolved `[[name]]` is "fine, not an error," so the rewrites left them untouched and re-pointed only the semantically-wrong ones. Two follow-ups belong on a future pass, NOT this ceremony's 2-surface scope: (1) root `CLAUDE.md` carries the same closed libp2p-0.53 drift just fixed in rust-architect.md (operator-gated gospel edit); (2) the catalog-wide dead-cite cluster warrants a dedicated `memory-coherence-audit` hygiene pass to decide which graduated-away forward-links should re-point vs. be authored.
