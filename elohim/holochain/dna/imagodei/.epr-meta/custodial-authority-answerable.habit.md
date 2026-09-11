---
epr-habit-version: 1
id: custodial-authority-answerable
invariant: >
  A party acting on another party's account is answerable for it: the authority reaches the
  ACTING HUMAN over a traversable path even when the steward is an institution, the act names
  that human afterwards, and the subject can contest the grant while it is in force. Authority
  here is relational — over this subject's named surfaces — never a rank; and layered policy
  composes one-way, so every additional steward may only ADD restrictions.
status: red
active: false
checks:
  - "a2o @concern:custodial-stewardship-06-collective-stewards (genesis/a2o/features/stewardship/collective-steward-answerable.feature — @act:i, household lane: `just test mesh features/stewardship/collective-steward-answerable.feature`, or from genesis/a2o `npx cucumber-js --config '' features/stewardship/collective-steward-answerable.feature` (exit 1 today). 8 scenarios, born RED 2026-09-11 with NO @wip on purpose — undefined steps are the loud honest red, since a held chapter measures nothing and would leave this habit unwired. They assert, on the existing Collective / Membership{role: Steward} / StewardshipGrant / StewardshipAppeal / DevicePolicy entry types and no new ones: a collective-held grant resolves to named steward members; an act names the member afterwards; an objection filed while the grant is live is answered by a named adult before its deadline, and an unanswered one is reported unanswered, never refused; authority is bounded to the grant's named surfaces; a second steward may add a restriction and can never lift another's; a withdrawn membership cannot act from the moment it was withdrawn. The concern tag is stewardship README row 06, reused verbatim as the README requires.)"
first_move: >
  write the red: the first `.feature` in genesis/a2o/features/stewardship/ that exercises a
  collective steward — grant held by a Collective CID, traversed via Membership{role: Steward}
  to a named member, the act attributed to that member, and the subject's appeal answered. The
  scenario is the specification; the two-party carrier above the DNA is designed against it, not
  ahead of it. Prose about the philosophy does not advance this habit (covenant rule 2).
retire-when: >
  when an institutional act that cannot name the acting human is UNREPRESENTABLE rather than
  merely discouraged — `steward_id` resolves to a member path by construction and an
  unattributed act has nowhere to be recorded. At that point answerability is a property of the
  entry types, not a practice under watch.
refs:
  - "genesis/a2o/features/stewardship/README.md (concern taxonomy, pickup order)"
  - "genesis/a2o/features/stewardship/.epr-meta (the authoring rule: a grant scenario must exercise the appeal)"
  - "genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md row 26"
  - "elohim/holochain/dna/imagodei/STEWARDSHIP_PHILOSOPHY.md"
---
2026-08-27: DECLARED `unwired` — committed to, with no runnable check. This is the first habit
declared under the composed register, and it is the state the old register could not afford: at
12/12 with a headcount cap, a slot was never spent on a commitment that could not yet be
observed, so `unwired` sat at zero for the whole life of the file while the honest answer for
this lane was exactly that. The lane exists (`genesis/a2o/features/stewardship/`, README +
`.epr-meta` with the appeal-path rule), the substrate exists (imagodei `StewardshipGrant` /
`StewardshipAppeal` / `DevicePolicy`, plus `Collective` / `Membership` / `CollabAgreement` in the
same integrity zome), and nothing above the DNA carries either — `steward_id` is still a single
String while `authority_basis` already names institutional bases. No `.feature` file exists yet,
so there is nothing to run and nothing to claim.
RED WRITTEN 2026-09-11 (unwired -> red, the first_move completed): genesis/a2o/features/stewardship/collective-steward-answerable.feature — 8 scenarios / 85 steps, all undefined, `npx cucumber-js --config '' <file>` exits 1 (a real failing measurement, not a parse no-op); no new entry types proposed. Authored by an Opus seat against the DNA truth (stewardship.rs, qahal.rs, the existing coordinator externs) and the stewardship README/.epr-meta rules (the appeal is exercised while the grant is in force); two context-isolated blind readers (a2o-story profile) returned READY twice — the first pass's three MAJORs (appeal causal chain: who answers and how the deadline is set; composition bundled two claims; Background env-var self-containment) were folded, the appeal chapter split into a positive and a negative scenario, and the second pass's remaining MAJORs are DEFERRED BY NAME: `And the answer is never the collective on its own` may read as redundant with the positive assertion above it; `when james opens his own account` names no defined surface; the appeal scenario keeps its filed→decided sequence in one scenario. `@requires:local-conductor` was dropped from the tag line because the act1 cluster-state declares no such cap (it would have printed UNDECLARED CAP and gated nothing) — conductor availability is inside Act I's household baseline. Cost accepted: `just test mesh` now carries 8 undefined scenarios and exits non-zero until the two-party carrier above the DNA exists.
