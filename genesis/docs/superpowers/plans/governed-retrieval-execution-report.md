---
id: governed-retrieval-execution-report
gap: plans__2026-09-09-governed-retrieval-execution#1
actor: agent:orchestrator@gpt-6
status: DONE_WITH_CONCERNS
cites: []
---

# Governed retrieval execution

Implemented version2 of the existing recall-contract algorithm content artifact and local
executor. Scope/discovery/filter/group/select/read/judgment composition is explicit; unsupported
composition refuses. CLI calls require one local session and an explicit evidence question.
Contract, CLI and runtime bytes pin the session. Continued calls retain cumulative costs;
unknown interrupted work remains visible. Session locking refuses concurrent execution.

Discovery streams bounded directory entries and metadata; filename filtering precedes metadata
reads, category membership is exact, and aggregates describe only returned candidates. Complete
frontmatter is required. Source excerpts carry their range and excerpt-only digest. Semantic and
native subprocess output/time are bounded before buffering, including stderr. Oversized serialized
output withholds evidence rather than silently truncating. External shell/MCP reads remain outside
this executor. The initial preset now reads usage and causal context, exposing the growing plan
as a follow-up route rather than silently widening the packet.

The co-located .epr-meta connects implementation edits to the algorithm contract and checks.
Root guidance, ceremony and four memory-agent packages teach the actual CLI. Canonical packages
were projected, not hand-edited. This is local EPRFS-governed content, not a new core EprKind,
DHT publication, peer authority, or universal harness enforcement. Session counters are local
operational accounting, not a planning queue. They are not tamper-proof or total-token meters.

Verification: 29 adversarial recall tests PASS (12 existing,17 new); 1919 package checks PASS.
Independent review found incomplete/truncated YAML delimiter bugs, corrected and retested.
Independent live CLI fitness: four named calls preserved 9676 source bytes and10599 scanned
bytes; zero unmetered attempts. Native output distinguished seven unaccepted assertions from
one scoped acceptance. Root additionally exercised bounded live semantic output (3332 provider
bytes,2.16s), with ranking/version/freshness explicitly unverified. No semantic ranking-quality
claim. No performance comparison equates these narrow reads with a complete signing investigation.

Native commitment: bafyreico3oz4hjjae52q2egcff2gw5y5skwygidessxzjmoupcuqetepau.
Independent technical approval: bafyreie2hbil4qyuyhbmfzq7pv7v63ofajo5kf7xsjdruwuymehfm7s3zi.
Implementation began under direct user authorization before native claim; no retroactive start
or actor history is asserted. Known limits: budgets can be changed by the artifact owner in a
new session; method-pinned local accounting does not prevent deliberate tool bypass; elapsed
scan bounds check between local filesystem operations, not cancellation of a blocked filesystem.
