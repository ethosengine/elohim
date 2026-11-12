---
id: task-brit-feedback-operation-report-20261112
gap: plans__2026-11-11-brit-feedback-operation#5
actor: agent:orchestrator@gpt-6-astra
status: DONE_WITH_CONCERNS
commits: [dd9001e7f0cc32934c93f62a779fe077fc31341a, 4575ab2eea9d2967de1bb6225971e42d3e2ce0e3]
---

# Ordinary installed feedback and experienced recovery

Steward: `human:matthew`. Brief: `task-brit-feedback-operation-brief.md`.
Technical review and fulfillment remain distinct from appointed acceptance.

The reviewed replay driver opened and closed an owned new process session,
`brit-feedback-installed-replay-20261112-a`, in the shared checkout. It used
ordinary PATH launchers with candidate overrides removed. Native resolution
selected the newly landed EPR and Brit as session pins. Raw stdout, stderr,
exit codes and timestamps are retained separately for each actual command.

`brit log -5` returned exit 0 and exactly five history entries. `brit diff
--stat` returned its expected refusal, exit 2. Reopening the existing private
store found exactly one automatic occurrence for each invocation:

- Success: `bafyreia223fovq7sszyn24g2jbpcji36eiwzbon2ub5lz5ednms3az7ekq`.
- Refusal: `bafyreibbrlmx6qhnrw3xykbcrk2j6qau5tdbsctw3heq4wned7mcjrmwwy`.

Both retain producer, operation, invocation identity, outcome and duration.
Both have null participant, message and detail; no raw argv, diagnostics or
environment was captured. Neither automatically created a commitment or recovery
claim. No automatic occurrence was attributed to Matthew.

The original log and diff records reopen with unchanged bytes/CIDs and policy.
Root-session pin SHA-256 values are unchanged, and the existing session still
selects its old releases. The replay's capacity reading retained policy CID
`bafyreictuustfij4ycilnpvo3gqlosmr2qplw5rajxlmmgqv76rwpajqly`, a 16 MiB ceiling,
58,835 retained/protected bytes, 59,109 total managed bytes, zero rejected
captures and zero reclaimable bytes. These are the replay-time measurements,
before subsequent deliberately authored recovery evidence. No reclamation or
store reset was performed.

Deliberate witness `bafyreia5qskgv6gswcsozvptzxm5ldsa5owx3pbh35gy3hiwkm6ijvzgpe`
attributes only the agent's actual successful log replay. The store refused an
initial attempt to reuse the automatic source identity with changed attribution;
that refusal is retained. The accepted witness uses a distinct authored item
with the same actual run, so distinct-run counts do not increase. Evidence link
`bafyreicutdxoxzyyvetwxi6br4lvcbqyb5cju4ba42vavuz7vxenaeb2zm` records experienced
recovery for the agent's prior installed refusal
`bafyreihui52r3wizupf3xqhincgtdk2uky6drqq5ew26ikqukxvlc774vy`.

The original Matthew occurrence receives available-release evidence only.
Native availability note
`bafyreigofpmj7mtjy66br6nnx4zocbviz35unk4pzeku6tuyphmes652cm` binds the published
source, installed pair and measured replay. A separate deliberate verification
`bafyreigdhpscsqcy2sgp2lcrjqabrth6ok4gse7lrixazcuvvagmsadhaq` closes the agent's
original installation-gap experience. No participant experience is inferred
from CI, fixture qualification or release availability alone.

The bounded-history repair was selected through retained feedback, native
grouping, Intent, registered claim, commitment link and independent review.
The delivered repair now has an attributable installed replay. Additional
native Intents retain the concrete next-sprint inputs: merge-commit support,
gitlink staging, push-transport diagnosis and receipt-envelope interoperability.
Those proposals remain unadmitted, with no fabricated frequency ranking. This
push's successful transfer is preserved separately from earlier transport
failure; it does not prove every historical cause repaired.

Proof: `genesis/a2o/reports/recovery/brit-feedback-20261111/`, especially
`brit-feedback-installed-replay-20261112-a/`,
`installed-replay-verification.json`, `attributed-installed-after-witness.json`,
`installed-log-recovery-link.json`, `installation-recovery-link.json`,
`available-release-*-link.json`, and `triage-after-installed-replay.json`.
The installed-current CheckWitness is `final-installed-probe.json`.
Unsupported Codex shell-hook coverage remains unmeasured; this result proves
the native producer/recorder and launcher path, not universal shell coverage.
