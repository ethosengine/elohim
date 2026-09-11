---
id: collective-memory-reader-entry
status: proposed
cites: []
---

# Observe the shared-memory ceremony

You are an isolated fresh reader for an authorized local experiment. Read this entry
only to begin; discover further context through the ceremony and its referenced evidence.
Do not reconstruct the repository or read implementation/test files outside those
sources. This deliberately isolated boundary is permitted by the root bounded-recall
contract. No build, commit, push, external provider or external message is required.

The concern: what do memory-economics observations warrant, and what still needs
judgment? The trial contains exact copies of current repository source, authored
contributions from agent A, and a labeled local-only deliberation passage. It is an
isolated local experiment, not a network or whole-product acceptance test.

Workspace: `/tmp/collective-memory-observation-20260909`.
Frozen native CLI: `/tmp/collective-memory-native-epr`.
Ceremony: `/projects/elohim/.claude/scripts/memory-kit/recall-ceremony.py`.
Native CLI SHA256: `a354c33a8fa329b2e0b47da5d8fb1ad98736b1cc11fc5137b5c6b202c39afe73`.
Use `--root` with the trial workspace and `--json` on both interfaces.

Register your own local identity using `epr actor claim --as agent:investigator@gpt-6-astra
--session reader-b`. This is attribution, not authentication. Then enter:

```sh
python3 /projects/elohim/.claude/scripts/memory-kit/recall-ceremony.py open --root /tmp/collective-memory-observation-20260909 --epr-bin /tmp/collective-memory-native-epr --session memory-observation --actor-session reader-b --scope genesis/concern --intent 'Decide what the measured memory changes warrant without erasing uncertainty.' --json
```

Follow the collective choice to understand governance and available input contracts.
Use `memory-project --memory-input genesis/concern/view.json` with the same common
flags to open agent A's explicit selection. Follow source choices and inspect only
passages needed to assess the claims. Record what is warranted and what is contested;
do not treat selection or a prior agent's claim as acceptance.

Exercise feedback on an exact reviewed projection (for selection or misleading
context), and contribute a qualified finding under your own identity if warranted.
Keep old source versions and uncertainty available. Only create new trial artifacts
under `genesis/concern/`; native/ceremony operational records remain in their existing
locations. Do not modify the copied code/design or private deliberation passage.
Select your resulting context through a new explicit request so another reader can
resume it. Stop BEFORE `finish` or closing measurement: root will appoint a different
fresh reader with only the continuation entry to complete the context-reset test.

Report the session locator, exact resulting request/projection references, sources
actually inspected, your decision and uncertainty, feedback reference and any friction.
Count visible output/source bytes if available; do not invent total token savings.
Your final report to root is observational evidence, not native acceptance.
