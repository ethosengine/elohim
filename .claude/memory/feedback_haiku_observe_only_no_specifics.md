---
name: Haiku observers report API-grounded facts only — never specifics
description: Haiku confidently hallucinates specifics from log content; structurally constrain its output schema to API-grounded facts and closed taxonomies, route specifics to Sonnet
type: feedback
originSessionId: 3cb7ed09-dde3-4e85-b6da-b0407506caa2
---
Haiku-tier observation agents (e.g. `ci-observer`) must NOT make specific factual claims. Their output schema is constrained to API-grounded facts (build_id, status, first_failing_stage, counts) and closed-taxonomy classifications (error_class enum, pattern_id from catalog). They report `artifacts_pulled` as URL/MCP-ref pointers, never extracted content.

**Why:** Smoke-tested ci-observer on elohim-genesis build #966 with `confidence: high`. It confidently reported a cucumber-expression error in `genesis/a2o/features/deployment/hub-topology.feature` with a specific line number — except that file was a local-only WIP not on origin/dev, the sprint-report had 0 scenarios, and the error didn't exist. Haiku synthesized plausible-sounding specifics from the log's general shape. The hallucination passed every internal consistency check because it was internally consistent fiction.

The fix isn't "tell Haiku to be careful" — Haiku will be confidently careful and still hallucinate. The fix is structural: remove fields where specifics could go (no `evidence` strings, no `files_mentioned` arrays, no quoted excerpts). Specifics become exclusively the job of Sonnet (`ci-investigator`), which Opus dispatches when it needs them, handing over artifact URLs from the observer's `artifacts_pulled` plus a specific question.

**How to apply:** When designing any Haiku-tier reporter:
- Schema fields are categories, counts, IDs, URLs, status enums — never free-form prose drawn from inputs.
- If the data needed to answer requires reading log content, that's a different agent's job (Sonnet).
- "What you DO report" / "What you DO NOT report" sections in the agent prompt should explicitly forbid quoted excerpts and inferred file paths.
- Confidence is in **the data observed**, not in any analysis — `low` means "I didn't see what I needed," not "I'm uncertain about my interpretation."
- Wire the two-step flow at the orchestrator: observer first, investigator only when specifics are needed to act.

**Visual-triage exception (2026-05-06):** Haiku CAN do bounded visual triage — `image_state` closed enum (blank | loading | error_overlay | partial_render | feature_visible | unreadable), `feature_identifiable` boolean, and a 160-char observational one-liner. Same discipline shape: closed taxonomy + bounded length forbids the hallucination surface. Tier-2 (ci-investigator) handles UI element enumeration / completeness; tier-3 (Opus) handles UX/design-spec stewardship. See `genesis/docs/superpowers/specs/2026-05-06-haiku-visual-triage-design.md` and the `visual_triage` field in `.claude/schemas/haiku-output.schema.json`. The exception holds because the new fields stay categorical/bounded, not free-form prose — the failure mode that triggered this memory was free-form synthesis from logs, not constrained classification.
