---
id: task-brit-feedback-install-report-20261112
gap: plans__2026-11-11-brit-feedback-operation#3
actor: agent:orchestrator@gpt-6-astra
status: DONE_WITH_CONCERNS
commits: [dd9001e7f0cc32934c93f62a779fe077fc31341a, 4575ab2eea9d2967de1bb6225971e42d3e2ce0e3, be9095de4373a69e5c9c365112746befa9797f28]
---

# Installed Brit feedback contract

Steward: `human:matthew`. Brief: `task-brit-feedback-install-brief.md`.
This is technical delivery evidence, not appointed acceptance.

The parent source `dd9001e7f0cc32934c93f62a779fe077fc31341a` passed all 27
pre-push gates and the normal hooked Brit push returned exit 0. A subsequent
Brit fetch confirmed that exact commit on `origin/dev`; its isolated checkout
was clean. Brit source `4575ab2eea9d2967de1bb6225971e42d3e2ce0e3` is published
on upstream main with its required aggregate check, package publisher/consumer
and four-executable CPU-qualified release green.

Native compare-and-swap landed these exact artifacts on 2026-11-12:

- EPR: `bafyreieu2hiydfq2n5pkhanqvsybhgs3q5s557swxbslsskqmo6bg3a3fu`, source `dd9001e7f0cc32934c93f62a779fe077fc31341a`.
- Brit: `bafyreiayutlx35qywcwbcexx7zc32rsc37xbi2qbmx6gmmon342coyxhbu`, source `4575ab2eea9d2967de1bb6225971e42d3e2ce0e3`, version 0.1.13.

EPR first passed recorder-only qualification against the old Brit. Brit then
passed full-pair qualification against the new EPR, including recorder-failure
exit preservation. A same-head EPR maintenance landing refreshed its full-pair
witness against the new Brit. Each act supplied the previously observed head
as `--expect`. Native launcher installation selected the landed immutable
launcher; all four Brit executables and the declared rakia alias remain in the
closure. No manual symlink or session-pin rewrite was used.

The installed-current diagnostic selected both artifacts through the native
launcher with no candidate arguments or overrides. Its `private-feedback-loop`
CheckWitness passed with `pairQualified: true`,
`recorderFailurePreservesExit: true`, and the pinned local capacity policy.
This diagnostic uses an isolated fixture. The separate ordinary shared-store
replay is documented in `task-brit-feedback-operation-report.md`.

Existing root-session pin files are byte-identical and still resolve to the old
EPR/Brit releases. A genuinely owned new process session selected and pinned
the new pair. Installation preserves existing session choices and explicit
overrides; it does not silently upgrade a running session.

Two installation seams discovered during delivery were repaired. SHA256SUMS
verification had incorrectly compared transport SHA-256 with native BLAKE3
identity. The fix hashes staged bytes with SHA-256 while retaining native CIDs
and canonical sums; an independent known vector failed before and seven
retained-closure tests passed after. The unchanged downloaded release was then
accepted by native ingestion. The advisory pre-push caller also omitted the
required `--expect`; follow-up commit `be9095de4373a69e5c9c365112746befa9797f28`
captures committed `status.head` once, refuses invalid status, and preserves
stale-head refusal without retry. Its 15 tests pass against the actual tree;
the new regressions fail against the old caller. Independent review found no
actionable issue. Native next-session follow already performed CAS and feedback
qualification correctly.

Proof is retained under `genesis/a2o/reports/recovery/brit-feedback-20261111/`:
`parent-publish-coordinated.log` and `.exit`, `published-parent-refs.txt`,
`final-brit-ingest.json`, `final-epr-land.json`, `final-brit-land.json`,
`final-epr-pair-maintenance.json`, both `final-*-launcher.json`,
`final-installed-probe.json`, `installed-replay-verification.json`, and
`hook-land-cas-tree-test.log`.

Concerns remain explicit: the broader daily-driver habit is red; unsupported
Codex shell invocation coverage is unmeasured. Old receipt-envelope decoding
still prevents gate reuse and has its own retained native proposal. Upstream
main's unrelated advisory and arm32 fixture failures are preserved in the
producer report; the required aggregate, publisher and CPU release passed.
Shared-root changes belonging to other work remain preserved for the coordinated
Claude handoff. No private feedback store was reset, reclaimed or published.
A checkout-local habit ruling initially fell back to the commit-author human
because the root session's actor sidecar is not present in that checkout.
`habit-green-ruling-isolated-attribution.json` preserves the append-only explicit
agent clarification; that fallback is not a new statement or acceptance by Matthew.
