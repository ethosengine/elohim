---
name: feedback_multi_agent_coherence_take_leave
title: "Multi-agent integration = coherence, not a hedge (take/leave)"
description: "Integrating a Codex/Gemini branch: judge take/leave/reshape against the trajectory; compose-don't-reinvent; done = composes, not compiles."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: c0e48b05-291d-4d51-9b7b-b1c616c500dd
---

When integrating another agent's contribution (a Codex/Gemini branch, an imported design) the **task is coherence, not defense**. The operator corrected this framing twice: multi-agent collaboration (Fable orchestrating / integrating Codex + Gemini) is *how you learn from each other and converge on one coherent system* — it is not primarily a hedge against being rate-limited or vendor-locked. "It's not the problem, it's the task."

**Why:** a lone agent works in its own cockpit without the shared substrate in view, so it reaches locally-reasonable answers that *reinvent* things the protocol already solves canonically (Codex's eprfs reinvented the CID codec, the `.epr-meta` engine, and the rule classes — all of which exist canonically in `elohim/epr`, `_lib/epr_meta.py`, and brit). That drift IS the incoherence multi-agent work risks; integrating without judgment just accretes each agent's local optima into a pile.

**How to apply:**
- The **review-the-trajectory-first** step (grounding pass, /atlas-grounding, a p2p-runtime review) exists precisely so you can render the integration judgment. Do it before touching the branch.
- Judge every idea against the trajectory: **TAKE** what advances it (Codex's projection layer, manifest/awareness/overlay split), **LEAVE** local reinventions of a canonical thing (String-newtype CID → real `cid::Cid` from the same lean base crates), **RESHAPE** directionally-right-but-forked work onto the canonical (route through `elohim/epr` / brit, not a third parser).
- **Respect deliberate deferrals.** A missing dependency may be an intentional decoupling for a clean future merge (Codex kept eprfs-core dep-free on purpose). The reshape is then "make the deferred coupling now, the *right* way" (consume the lean canonical slice, not the heavy atom codec) — not "fix a mistake."
- **Compose, don't reinvent** is the criterion. Integration isn't done when it's green; it's **done when it composes**.
- Apply the same discipline to your OWN proposed work: don't build a guard/parity gate ahead of need — a resolver that doesn't yet *enforce* can't drift, so pinning it is premature.
- Endgame: the coherence rule can become mechanical — `.epr-meta`'s `dedupe-of` ("this concern already lives at X") can encode "consume the canonical, don't reinvent," so the substrate governs architectural coherence, not just commit-frontmatter. See [[project_epr_meta_compose_gate]], [[project_brit_next_gen_epr_meta_foundation]], [[feedback_delegate_narrow_tasks_to_cheaper_tiers]].
