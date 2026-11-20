---
name: feedback_verifier_needs_counterpart_minted_vector
title: "A verifier ported from another implementation's source is proven only by a vector that implementation minted"
description: "2026-11-06: the a2o epoch-2 attestation verifier passed its own self-consistent tests yet admitted three classes brit refuses (non-strict ed25519 weak keys, unsigned extra fields, held/refused swapped); only a brit-minted note plus an Opus review caught them."
metadata:
  type: feedback
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-06T20:54:31.068Z
---

Re-genesis 1, T4 follow-on: a Sonnet agent ported brit's `brit.validate@2` verification into
`genesis/a2o/scripts/lib/household-attestation.ts` by reading brit and elohim-epr source. Its tests
signed with the same framing it verified, so 39/39 proved self-consistency only. The Opus review
with a note minted by the real brit build (c3b3d8af0) found: Node's `crypto.verify` for ed25519 is
not `verify_strict` (an identity-key forgery verifies); brit decodes with `deny_unknown_fields`
while the port ignored extras; brit HOLDS an unknown scheme and REFUSES a malformed envelope, the
port did the reverse (so a writer would overwrite a future-scheme note instead of deferring).

**Why:** a verifier is an integrity surface; symmetric tests cannot see where the counterpart is
stricter. The rule in the plan (integrity surfaces get a reviewer from a different family, with the
counterpart's bytes) is what caught it, not more tests by the author.

**How to apply:** when porting any signature/envelope check (brit notes, EPR signatures, custody
announcements, inventory envelopes) across languages, the brief must demand a fixture minted by the
counterpart binary and a tamper probe on it; in Node, wrap ed25519 verification with small-order
key/R rejection; mirror the counterpart's HELD-vs-REFUSED split field by field. Related:
[[feedback_integrity_calls_stay_with_claude_models]], [[project_brit_worktree_gaps_2026_11]].
