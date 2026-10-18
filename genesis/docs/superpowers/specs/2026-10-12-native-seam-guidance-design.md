---
id: native-seam-guidance
status: Implemented
class: design
context-tier: warm
steward: governance-plane-single-evaluator
graduation-trigger: Native classifier, schema, adapter, privacy, review and projection checks pass; live activation is claimed separately for each harness.
cites: []
---

# Native seam guidance across agent harnesses

Authoring guidance identifies responsible seams and existing interfaces without making a lexical
match into an architectural ruling. The existing `governance-plane-single-evaluator` habit owns
this work. Existing permit/refuse/refer decisions and hard gates keep their authority.

## Authority and composition

The root composition standards in `.epr-meta/manifest.md` remain the prose contract. The
working-version standard (`2026-10-04-working-version-sdk-standard-design.md`) and the
concern-routing atlas (`2026-06-21-elohim-seam-map-concern-routing.md`) distinguish producer
ownership from application composition. In particular, planned working-version lifecycle APIs
are not delivered interfaces. Evidence carries `implemented`, `contract` or `planned` status.

Owner-local declarations, bound explicitly by the existing validator mechanism, refine these
standards. They live beside the owning `.epr-meta`, as `*.seam.json`. Each declares responsibility,
owner paths, allowed consumer paths, authoritative contracts, reusable interfaces, applicability,
lexical detection hints, composition hints and exclusions. The JSON schema is
`elohim/sdk/schemas/v1/seam-guidance.schema.json`; native strict deserialization and validation
reject malformed declarations. There is no recursively discovered second ownership registry.
Nearest rule binding wins through the existing cascade. Repository boundaries are respected:
no implicit parent inheritance into Brit or Rakia, and no automatic scanning of their contents.
A standalone repository must bind its own local declarations. The monorepo composition contract
may explicitly describe its relationship with independent tools without claiming their governance.

Four initial families cover reusable working-version persistence/sync, working values crossing
publication authority, peer-native serving implemented in doorway, and Git/build interpretation
inside generic governance/content primitives. All are advisory heuristics. A composition hint
is evidence, never an exemption that conceals a mixed change. Owner implementation and legitimate
reuse receive distinct candidate labels. Exclusivity, authorization and semantic correctness
cannot be proved by these hints.

## Governable algorithm and application

The classifier itself is `algorithm-content`, following the existing bounded-recall convention:
`.epr-meta/elohim/algorithms/seam-guidance.json`. Its governance names the existing habit,
executor, checks, retirement condition and authority boundary. The root validator binding
explicitly references the algorithm; this is governed authored content, not a new protocol kind.
Its ordered composition names normalization, resolution, detection, source verification,
packet bounding, active-agent review and review validation. The native executor refuses an
unsupported composition rather than claiming to execute arbitrary declared methods.

The artifact separately declares its application: advisory mode, active-agent review, candidate
count, packet budget, two-second deadline, no inference, no persistence and explicit-only
contributions. The report pins the raw algorithm CID, application CID and compiled implementation
identity. Native execution honors narrower candidate/packet budgets and rejects unsupported or
unsafe application settings. Changes to method, implementation or application invalidate earlier
review associations. The artifact is subject to the same `.epr-meta` cascade and deliberate
correction process as other authored policy; classification never changes it automatically.

## Native request and evidence contract

`epr govern --path ...` retains its existing behavior. `epr govern seam` reads a JSON object on
stdin. It accepts either `{harness, event}` or `{proposal}`, with optional `review`. The normalized
proposal carries `repository`, `session`, and `changes`; each change carries `operation`, `path`,
`prior`, `proposed`, and optional `from`. Operations are create, edit, delete and rename. Prior
bytes must match disk. Renames evaluate both paths; multi-file patches remain one proposal.
Unsupported tools, unknown operations, ambiguous context, malformed patches, non-UTF-8 files,
escaping paths, independent repositories and incomplete inputs produce explicit coverage gaps.
Reconstruction never executes a patch. This is an advisory snapshot, not filesystem concurrency
control or authorization to mutate.

The CLI composition root implements the classifier once, behind `ValidatorProvider`. Generic
EPRFS governance carries optional opaque validator arguments while preserving legacy string
references. No concrete seam identity or Git interpretation enters the generic crate. Compatibility
hosts read the existing shared validator registry and defer classification to native execution;
the hook reports native absence rather than recreating its logic.

Evidence identifies declaration bytes, source bytes and exact UTF-8 byte spans with line numbers.
Reusable sources are re-read and their declared anchors must exist before they are returned.
Changed sources invalidate review even if an anchor remains. Source anchors are not claims that
the complete architecture has been reviewed. The classifier deliberately reports uncertainty.

## Active-agent review and private memory

The active agent reviews at most three injected candidates. It receives source excerpts and the
proposal evidence, plus instructions to inspect the proposal and use existing bounded recall
for supplementary discovery. Similarity does not establish ownership or correctness. Candidate
sources must be re-read before citation. Large candidate packets are omitted whole with a named
coverage limit; injected context never exceeds 8 KiB.

Review responses contain `proposalId`, `decision` (`keep`, `route`, `split`, `abstain`), `rationale`,
`limitations` and `citations`. Citations are exact evidence objects from the current report.
Non-abstaining responses require evidence. Native validation checks structure, association with
the exact proposal and current source identities, and citation membership. It does not certify
semantic judgment. Disagreement with the heuristic, including a cited `keep`, is allowed.
Only a still-valid supplied review suppresses repeated guidance; changes to proposal/session,
declarations or cited sources invalidate it. There is no durable suppression cache or hidden
review ledger. The caller retains private review state and may resubmit it through the explicit
interface; automatic harness review persistence is outside this MVP.

Proposals and reviews are private local authoring data (B); derived reports are ephemeral (C),
reconstructed from the proposal and repository sources. Content-derived identities reuse
`eprfs_core::BlobCid`; no new address algorithm is introduced. No DHT entry, coordinator, signal,
HTTP route, sync message or recurring head is added. One-year head-plane increase is zero.
The local holder can inspect drafts; this path cannot join or publish activity across people.
The classifier and hook perform no inference call, collective-memory contribution, training,
policy change or publication. Corrections cross only through the existing deliberate governed
contribution process. Recall receipts remain private.

## Harness adapters and capability reporting

One native normalizer owns event reconstruction and one report owns review instructions. Claude
supports Write and Edit, including exact replace-all. Codex supports `apply_patch` in
`tool_input.command`: add, update, delete, move and multi-file patches with exact unambiguous
context. Fuzzy/ambiguous patches are explicitly uncovered. Arbitrary shell writes and other tools
remain uncovered; neither adapter claims a complete filesystem interception boundary.

`epr govern seam capabilities <claude|codex|antigravity>` reports support separately from registration
and trust. Antigravity declares a deferred invocation adapter and no supported events. It does
not impersonate Claude. The native hook package projects byte-identical thin wrappers for Claude
and Codex. Native package registrations are merged by their own projected path; unrelated hooks
and settings survive. Legacy imported hook registrations remain read-only.

The synchronous wrapper has a two-second registration deadline and a 1.8-second internal budget;
it never waits for model inference. Missing evaluators, failures and timeout paths emit advisory
coverage gaps, without returning allow/deny or modifying the tool input. Existing governance
hooks run independently. Source: [official Codex hook contract](https://learn.chatgpt.com/docs/hooks)
(read 2026-10-12). Project-local configuration and hook commands require normal runtime trust
review. Generated files or `doctor` file-presence checks do not prove activation.

## Acceptance and remaining limits

The labeled fixture corpus exercises misplaced implementation, reuse, owner implementation,
mixed composition, comments, examples and unrelated edits in all four families. Identical Claude
and Codex proposals must produce identical native reports. Native tests also exercise invalid
references, nested rule overrides, independent repositories, malformed patches, moves, multi-file
changes, UTF-8 offsets, size limits, stale priors, review disagreement/abstention and invalid
citations. Schema and projection tests protect strict declaration shapes and unrelated hooks.
Privacy tests check no local output store or proposed file mutation; hook tests substitute a local
process to exercise failure and deadlines without remote inference.

Active-agent exercises must inspect real producer bytes and record a cited route for a duplicate
and a keep for legitimate composition through both adapters. These are bounded usefulness
exercises, not a claim of an independent model evaluation or live Claude/Codex hook activation.
Verification results and measured latency belong in the existing habit atom, followed by habit
projection. Unverified activation, broader language parsing, fuzzy patch equivalence and automatic
review resubmission must remain named limits.

## Preserved regression boundaries

The native acceptance tests are the runnable probes for these harvested story boundaries:

```gherkin
@concern:governance-plane-single-evaluator @regression
Scenario: A reviewed proposal changes its governed classifier application
  Given an agent has reviewed a proposal using the declared classifier method
  When the classifier application changes its packet or candidate budget
  Then the old review is stale even when the proposed code is unchanged
  And the next report names the changed application identity

@concern:governance-plane-single-evaluator @regression
Scenario: An abstaining review cannot conceal missing classification
  Given the classifier method is invalid and the report names a coverage gap
  When the active agent abstains with an exact proposal reference
  Then the review structure may validate
  And the coverage gap remains visible to the next authoring step
```

These are design-level story descriptions, not claims of wired Cucumber steps. Native probes:
`governed_method_and_application_are_pinned_and_executed` and
`abstention_cannot_hide_failure_and_invalid_method_revisions_invalidate_review` in
`elohim/eprfs/epr-cli/tests/seam_guidance.rs`. Operational bounds are three candidates, 8 KiB
and two seconds; the package wrapper reserves 200 ms for startup and rendering.

## Verified implementation, 2026-10-12

The owning native gate passed 1,202 tests with zero failures/ignored, including existing governance
parity and live manifest checks. Final focused checks cover all eight seam tests after the last
bounded-output refinements. Declaration/method schema and registration tests passed (3), wrapper
privacy/degradation tests passed (5), seed schema validation passed (3,556) and package verification
passed 2,198 assertions. The full native gate exceeded its ten-minute soft budget; it did not
reach its twenty-minute hard ceiling.

Four active-agent synthetic reviews (duplicate route and composition keep, each through Claude
and Codex) validated against fresh evidence. The exact proposals had identical native reports
across adapters. Local receipts and latency samples are at
`genesis/a2o/reports/recovery/native-seam-guidance-2026-10-12/`. They use synthetic drafts and
explicitly record that the implementing agent supplied the judgments. No production draft was
persisted by classification.

Hook registration is projected. On 2026-10-12 the locked native build was refreshed using the
verified build cache and atomically installed at the existing PATH target,
`/projects/.cargo-target-pool/family/elohim/elohim__eprfs/dev/debug/epr`.
Both registered hook commands passed duplicate and composition smoke checks without
`EPR_SEAM_BIN`, with identical native reports and 127–181 ms elapsed times. `epr doctor`
reports the governance floor ready. The local `installed-integration.json` receipt pins the binary
SHA-256 and method identities.

Codex 0.162.1 has hooks enabled and the project trusted. Its native `hooks/list` API recognizes
the exact project hook without warnings or errors but reports its definition as `untrusted`.
Complete the normal `/hooks` review in the Codex CLI to activate it; the local
`codex-hooks-runtime.json` receipt records that distinction. Claude's registered command is
verified, while live lifecycle invocation remains unobserved. Neither generated files nor direct
adapter payload exercises constitute a trusted runtime activation claim.
