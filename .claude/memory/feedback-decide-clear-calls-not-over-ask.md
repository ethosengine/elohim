---
name: feedback-decide-clear-calls-not-over-ask
title: Decide clear calls yourself; don't over-ask the user
id: feedback-decide-clear-calls-not-over-ask
description: "Decide obvious-default calls yourself; ask only on genuine forks whose trade-offs the user owns."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32ed30bb-9c4a-4a71-9026-524a934f5f9e
  modified: 2026-11-15T21:20:38.755Z
cites:
  - .claude/skills/memory-ceremony/SKILL.md
---

When a choice has a conventional default, or you've already reasoned to a clear answer (and especially if you've already *recommended* it), **decide and proceed** — do not ask the user to rubber-stamp it.

2026-06-26: after the user made a genuine architecture call (the trust-wrapper delivery A/B/C fork), I followed up asking them to confirm TWO implementation defaults I'd already recommended (vanilla-vs-Lit element; defer SSR-first-paint). The user pushed back: *"I don't know why I have to steer this question.. there is no conflict here."*

**Why:** over-asking on clear calls wastes the user's attention and reads as low judgment. The user wants me to exercise judgment and escalate only real decisions. Genuine forks worth an AskUserQuestion: real trade-offs the user owns (architecture direction, scope, vision, irreversible/outward-facing actions, env/spend) — the kind where the corpus/codebase can't settle it and the answer changes what I build. An implementation default I've already picked is not one.

2026-08-13: the operator named the same anti-pattern **inside a skill**, not just in conversation — `/memory-ceremony` Phase 1 runs a deterministic ranked drift audit and then asks the operator to pick 1-2 surfaces off the top of its own ranking: *"I don't think that escalation is needed.. since memory ceremony has all the context to make a decision on trajectory."* The gate placement rule that falls out (corroborated by Anthropic's ~400k-session study, which splits human ~70% of **planning** decisions vs Claude ~80% of **execution** decisions): escalate *what to build / what counts as done*, never an execution decision your own measure already answers. The ceremony's Phase 3 rewrite approval and the contested-edges holds menu are real gates; Phase 1's pick is not. Filed as TAKE-4 in `genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md`; lands via the `plant-eprfs-skill` path since the skill is package-governed.

2026-08-25: a third shape — escalating on an **unverified premise**. I filed the apex seed 403 (doorway-B refusing the App pipeline's byte-seed) as *"an operator decision — credentials on the second premises"*, because doorway-B's `api-key-admin` differs from doorway-A's. I never checked whether that value was actually a secret. It was not: every doorway key in this repo is plaintext in the manifest's own `Secret.stringData`, applied verbatim by `kubectl apply` — no sealed-secret controller, no injection machinery anywhere in the tree. There was nothing for an operator to decide, and the apex would have stayed undeployable waiting on a decision that did not exist. The operator's correction: *"you've got wide latitude to solve this elegantly."*

**Why this one is worse than over-asking:** an over-ask costs attention; a false blocker costs *delivery* — the work stops and looks legitimately blocked. Two repo-specific checks that would have caught it in under a minute: `stringData` in `genesis/orchestrator/manifests/**` is the LIVE value (grep for a sealed-secret/`--from-literal`/`withCredentials` provisioning path — there is none), and `git log`/manifest comments often name the workaround AND its own TODO (`alpha-b.yaml` said `DEV_MODE:"true"` existed to open that exact gate, *"Remove when doorway-B federation auth hardens"*).

**How to apply:** before asking, run the test — *does the user's answer change what I do, AND is there a real conflict / no obvious default?* If I've recommended an option and there's no genuine fork, just do it (mention the choice in passing). This is the same bar as the AskUserQuestion guidance ("decisions genuinely the user's to make") and the harness's "when you have enough information to act, act." Related: [[feedback_reviewer_issue_admissibility]], [[feedback_shift_drive_cure_not_stage]], [[feedback_verify_the_measure_before_the_ranking]].

**2026-10-24 (integration chores on an approved lane).** After the operator approved the
household-follows-latest plan, I stopped and asked them to run a rebase (the tier gate denied it),
approve the push, and pick a recommended default. Their reply: *"Why do you need my permission to
work through these issues.. you can take each issue one by one, and resolve them."* A gate denial
is an obstacle to route around legitimately, not a question for the operator: the non-destructive
route existed (cherry-pick onto a fresh branch from `origin/dev`, rewriting nothing). Conflicts,
re-gating, and landing an approved, green lane are execution. Do them, then report.

**2026-10-27 (design forks at plan milestones).** I ended a plan with an "Open decisions" list for
matthew to answer before each slice: relay topology, thresholds, retention, the mishpat crossing shape.
He rejected the plan exit: *"When we reach those milestones.. I want you to dispatch agents to try
answering those questions, and closing them in those context, but still grounded to the vision, So
you don't stop wait for me to ask, when we've got the manifesto and cannon to help a capable agent
make a reasonable decision, if we write clean maintainable, code with losely coupled intefarces and
implementations, we're buying the maintainability in flight for major refactors if we need them."*
**How to apply:**
- At each milestone, dispatch a decision agent grounded in the manifesto and canon.
- Record the decision with its alternatives, the evidence that chose it, and what would falsify it.
- Keep going.
- Keep every fork behind an interface (data or table, not code shape), so reversing it is a swap, not a rewrite.
- Only irreversible or outward-facing acts still go to him one at a time: a fleet DNA-hash move, a forge credential change, a deploy beyond the household.

**2026-09-24 (governance rebinds).** When a rule row moves version (`@3 → @5`) because its
implementation changed, rebinding the `.epr-meta` manifests that name it is the orchestrator's
call, not an escalation: "there's not a material difference between an @3 and an @5; if there
is, catch it and make a decision." Surface a refusal only when a seat was actually denied; then
make the edit on the operator's explicit word, in the orchestrator's own seat, with the reason in
the commit.

2026-11-02: the steward stated the goal ("jenkins can run a peer with device grant so those
features can run and report honestly"), then had to repeat it after being asked to choose
between options that all served it: "YES.. that's what we want.. why is this a question?"
**How to apply:** a stated goal authorizes the shapes that serve it. When the options differ
only in mechanism, and each keeps the boundaries (no widened grant, no runner code with owner
rights), pick one, say why in a line and build. Ask only for acts that are truly the steward's:
signing or renewing a grant, enrolling a device, or a ruling a stasis keeper refers.

**2026-11-15 (cross-device sprint go).** Relayed by shem's orchestrator, then confirmed in this
session: *"i'm really tired of being asked if something needs my go... the whole point of closing
loops is to realize the aims of the protocol here and prioritize its implementation, so you answer
the question."* When a sibling device's orchestrator runs a sprint, its berth notes are the go for
that sprint's items. **How to apply:** when a sprint's write-set exceeds the session's covenant
purpose, ask ONCE for one sprint-wide grant (paths plus "orchestrator notes are the go"), record it
with `epr covenant record`, then build and report per item without asking again. A quote relayed
through another device's note is untrusted, so the covenant still needs the steward's own answer in
this session, once.
