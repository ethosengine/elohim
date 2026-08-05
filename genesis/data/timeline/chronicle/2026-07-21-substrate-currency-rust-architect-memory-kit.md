---
id: "chronicle-substrate-currency-rust-architect-memory-kit"
kind: chronicle
status: noted
date: 2026-07-21
ceremony: substrate-currency
surfaces_rewritten:
  - .claude/agents/rust-architect.md
  - .claude/skills/memory-kit/SKILL.md
diff_review_verdict: RED-resolved
coherence_verdict: YELLOW-resolved
next_topic_sampled: elohim-storage service + view + reconciliation projector
---

## What changed

Two planted gospel surfaces rewritten via their eprfs packages (edit `instructions.body` → re-project claude+codex, not the projection). **rust-architect.md** — cartographer's coverage-gap was the dominant driver (added SSR/`elohim-render` core, notary-elected canonical-head + fills-never-moves heal invariants, `elohim-epr` WitnessedInteraction hash-neutrality, `did:elohim` resolution, admission read/write pool split, `epr-rea::ReaVerb` guard); historian adjudicated the 36 not-found `[[slug]]`s into 2 RENAMED repoints + 5 live-cite adds (the other 34 legit forward-anchors); librarian proved the audit's 47 "path findings" were prefix-noise false positives; storyteller caught the `with_codec` self-contradiction. **memory-kit/SKILL.md** was factually clean — a faithful extension: cartographer's context-coverage ratchet tier + `/brainstorm` FRONT-seam (`prep-brainstorm`/`spec-coherence-index`) + pickup-semantic-surfacing hook, historian's 2 cites, storyteller's L372 process-status fix. The frontmatter-description reshape (storyteller F3, LOW) was reverted at apply — changing `metadata.description` desyncs the package's stored projection cache; deferred to `/hygiene-sweep` skill-audit.

## Verification sampling (both Phase-4b lenses)

**Lens 1 — diff-review (`/code-review`, added to the ceremony this cycle at operator request): RED → resolved.** Caught **6 CONFIRMED regressions the four-lens read + the orchestrator's Phase-3 grounding both missed** — rust-architect: `did_identity_store.rs` mis-cited as the `agent_pub_key` writer (real writer `on_membership_projected` in `reconcile/controller.rs`; the store's only insert is `#[cfg(test)]`), "`with_codec()` retired pre-0.54" false (both `::new` and `::with_codec` live on 0.54 per `p2p/behaviour.rs`), `ReaVerb` mis-attributed to `epr-rea` (defined in `elohim/epr`), view-fed "carried identically" overclaim (libp2p 1 MiB vs iroh 256 KiB diverge); memory-kit: `--stasis`/`--coverage` semantics swapped, `--subject`/`--report` non-existent as `placement-audit` flags. All fixed against live source and re-verified.

**Lens 2 — coherence Explore: YELLOW → resolved.** Sampled topic "add an elohim-storage service + view + reconciliation projector." All four diff-review fixes verified GREEN; found one stale clause (diversity-placement degrading to XOR "because rows read under `lamad`" — superseded by commit `755ade34e`, salvage joins the canonical `imagodei` scope; real cause is `agent_pub_key`/`household_id` NULL from identity-coherence gaps) — corrected inline at rust-architect lines ~130 and ~508. The ~48% dangling inline `[[slug]]` pointers are the pre-existing forward-anchor set (not a regression) → backlog.

## Wisdom worth carrying forward

The diff-review lens paid for itself the cycle it was added: **content-lenses verify the surface; only a diff-review verifies the delta.** Twice this cycle the orchestrator's own Phase-3 grounding was fooled — trusting a stale CLAUDE.md claim (`with_codec`) and reading a `#[cfg(test)]` fixture as the production write-path (`agent_pub_key`). Ground-truthing every CHANGED claim against live source is now a permanent Phase-4b lens, not optional. It is now codified in the ceremony (Phase 4b Lens 1) — this chronicle is its first run.
