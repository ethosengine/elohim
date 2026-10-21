---
id: placement-lens-brief
title: Apply the three design-lens findings to the placement-contract spec under rulings R1–R6
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#23
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:2eb4a96fe32bc3d8 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "placement-settle-report | Settling the placement-contract spec | sha256:a03732e0339e3d4b | path: genesis/docs/superpowers/plans/task-placement-settle-report.md"
---

Item P0b (gap `#23`). The absolutes half of P0b is already done (a pass took the spec's absolutes from 81
lines to 12, each rule now a commitment with its gate and that gate's state); do not redo it, and keep its
register: a rule states the commitment, then the gate that holds it and whether the gate exists yet.

Input: the three design-lens reports in
`/tmp/claude-0/-projects-elohim/7e52adff-23e3-4a4c-86f3-735370908262/scratchpad/settle-lenses.md`
(local-first; p2p design gate and concern canon; gradient and commons design). 67 findings, each with a
replacement. If that file is gone, the same findings are the `lens:*` results in the workflow journal
`/projects/.claude-config/projects/-projects-elohim/7e52adff-23e3-4a4c-86f3-735370908262/subagents/workflows/wf_24c92eb5-b73/journal.jsonl`.

The orchestrator's rulings R1–R6 are recorded on the plan and listed in `task-placement-interfaces-brief.md`.
Where two lenses conflict, these reconciliations bind:

1. **Posture and berth offer.** Notarized on the existing elohim-DNA `Intent` only for public or commons
   reach (R2). Inside a household they are household-local signed records, written on change with hysteresis,
   read by the household's own floor. The facts and private intents behind a posture never leave the device;
   others see "not offered".
2. **Battery.** Carrying for another household or space is off by default on a device on battery, on a metered
   link or not always on, and its person may lift that default with their own posture. Holding within the
   device's own household is never refused for battery; it is deferred to a charging or unmetered window.
   Today no placement code reads `battery`, and the spec says so.
3. **Leases.** Long-lived. A lapse re-places before it releases, and a recipient being unreachable never by
   itself ends what is carried for it (R1).
4. **The ceiling.** There is no space-elohim. Each member's elohim drafts for its own person from what that
   person may read. Every parameter set put in force is ratified by the participants it touches, every time
   (R5). A carrier that can sign as the person is chosen by the person, never placed by the floor.
5. **Spaces.** Move the holons §9.1 bullet out of §2 ("already ruled") into §3 as the working hypothesis it is,
   noting that it is under product review.
6. **Depth as placement and "eligible is not enlisted"** (added to §7.2, §12.7 and §16 during settle) stand,
   and the lens edits must keep them.

Do:

- Apply every blocker and major finding under these rulings. Apply minor findings unless one conflicts with
  a ruling. Name each refusal in the report.
- Rename or add the §16 stories the lenses call for (including "an elohim's draft tuning, unratified, changes
  no placement on any peer"), and the §15 missing nodes they name.
- Re-read the spec before every edit and edit by unique-string replacement only: another seat may edit the
  same file. Keep section numbers stable. Run `epr flow cites verify` on the spec and quote its line.

Report: `task-placement-lens-report.md` with `gap:`, `actor:`, `status:` and a table of every finding, marked
applied, applied-differently or refused, with one line of why. No staging, commits or pushes.
