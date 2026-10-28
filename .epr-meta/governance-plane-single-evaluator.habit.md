---
epr-habit-version: 1
id: governance-plane-single-evaluator
invariant: >
  The repo's governance plane has exactly ONE evaluator. A given manifest
  yields one decision class, not one per implementation that happens to be
  reachable; every decision names the evaluator that produced it; and a
  second host can re-derive that decision and record a disagreement by name.
status: green
active: false
checks:
  - "Native seam guidance: just gate eprfs; cargo test -p elohim-epr-cli --test seam_guidance (use the manifest cargo target). Eight acceptance tests include a 28-case labeled corpus, Claude/Codex equivalence, source/algorithm/application freshness, review validation, boundaries and privacy. GREEN 2026-10-12."
  - "node --test elohim/sdk/domains/elohim-agent/scripts/seam-guidance.test.mjs; python3 elohim/sdk/domains/elohim-agent/tests/seam_hook_test.py; pnpm run elohim-agent:packages:verify — schema, registration preservation, adapter privacy/degradation and projection fidelity. GREEN 2026-10-12."
  - "python3 .claude/scripts/_lib/__tests__/validator_registry_parity_test.py — 11 assertions, GREEN 2026-08-12. Validator ACCOUNTING against the shared scope registry (no implemented validator undeclared; each host implements exactly its declared share), WIRING (both live decision surfaces call the native evaluator), and ATTRIBUTION (both hosts name themselves by content address; the ledger stamps an evaluator on every decision; a dispute record names both builds)."
  - "cargo test -p elohim-epr-cli --test governance_parity_vectors — the Rust half of the correspondence theorem over governance-parity-vectors.json, run against the COMPOSED evaluator (library + ElohimRepositoryValidators). GREEN at 12/12, 0 skips. The Python half (python3 .claude/scripts/_lib/__tests__/test_governance_parity.py) is GREEN at 12/12 / 52 assertions. Both hosts now derive the same verdict from the same corpus — correspondence, which is necessary and NOT sufficient for this habit."
  - "cargo test -p elohim-epr-cli --test live_manifest_integrity — the native integrity layer over the LIVE corpus (43 manifests), guarding against an over-strict check that would false-block every write under a subtree. GREEN."
guard: >
  Risk 1 — "parity" achieved by deleting the divergent validators rather
  than by unifying the evaluators. That would make the check green and the
  governance plane weaker; the three python-only rules are bound by live
  manifests and are class `inject`. RESOLVED 2026-08-12, and the resolution
  is worth stating because the obvious repair was the wrong one: porting
  each validator to both hosts yields TWO IMPLEMENTATIONS OF ONE PREDICATE,
  which IS the fork rather than its cure. What must not diverge is the MAP,
  so the scope registry declares who owns what and each host implements
  exactly its own share. The first version of the check asserted "same set"
  and would have driven precisely the wrong repair.
  Risk 1b (NEW, measured) — a decision-only collapse silently drops the
  three python-owned `inject` ADVISORIES, which fire on elohim-storage/src.
  One decision authority need not mean one advisory source; the scope
  registry already distinguishes them. Do not let "single evaluator" quietly
  become "less advice".
  Risk 2 — making the Python hook shell to `epr check` without a
  fail-closed fallback. When the binary is absent the hook must REFER, never
  permit; a permit-on-missing-binary turns an evaluator outage into silent
  blanket approval, which is strictly worse than the fork.
  Risk 3 — reading this habit as "make the test pass." The failable claim is
  re-derivability by a SECOND HOST, and agreement is the trivial half. The
  artifact is the disagreement record; a green parity test with no dispute
  path has not moved the habit.
  Risk 4 (WIP fence) — this takes the register to 12/12 and to 2/2 active
  alongside notary-authority. Covenant rule 3 says finishing beats starting.
  If notary-authority is the priority, flip this to active: false; it stays
  a legitimate red either way.
refs:
  - "guidestar: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md — §5's repair list; this habit is the adjudicated slice 1, chosen ahead of the standing/Principal primitive because admitting a new primitive into a plane that is currently forked ships it twice, differently"
  - ".claude/scripts/_lib/epr_meta.py REFERENCE_VALIDATORS (the LIVE registry) vs elohim/eprfs/epr-cli/src/repository_validators.rs (the twin)"
  - "elohim/sdk/schemas/v1/registries/governance-parity-vectors.json — the 9 vectors; their coverage gap is failure (2) of the check"
  - "NOT covered here — the standing/proxy-declarer primitive (§5 rank 2), deliberately deferred; and .claude/data/governance-findings.jsonl, 4.3MB written with zero readers, which needs a read path or a stop-write"
retire-when: >
  when the Python and Rust evaluators are ONE artifact both hosts call, rather than two
  implementations held in correspondence by parity vectors. "Exactly one evaluator" is
  then a fact about the build, not a property under watch.
---
RED WRITTEN 2026-08-12, born red, measured not asserted. Two independent
reds, and the second is a DEMONSTRATED divergence rather than a
structural one.
(A) REGISTRY FORK, in both severity directions at once: Python
`REFERENCE_VALIDATORS` holds 10, the Rust `ElohimRepositoryValidators`
match arm holds 8, overlap 7 — python-only are heal-fills-never-moves,
bounded-work, dna-hash-neutrality (unenforced under `epr check`);
rust-only is eprfs-meta-domain-neutrality (invisible to the live hook).
All FOUR divergent validators appear in ZERO golden vectors, which is how
governance-parity-vectors.json read green over a forked plane — the
correspondence theorem was asserted over the intersection and silent on
the difference. And `epr check` is invoked by NO gate (not
.husky/pre-push, not .husky/pre-commit, not the resolver hook, not the git
gate), so the parity-tested Rust twin never runs and a divergence has no
way to surface in normal use.
(B) MEASURED OPPOSITE VERDICTS, found 2026-08-12 while repairing (A). The
measure-tier `kind: level|rate|ratio` requirement landed 2026-08-11
(8da9b519c) in the Python evaluator only. The golden corpus was last
touched 2026-07-23 and still carried a kind-less measure rule, so from
8da9b519c the two runners derived OPPOSITE decisions from the same
manifest — Python `refer` (malformed, routes to operator), Rust `permit`
(accepted the rule and applied it). Nobody saw it because no gate runs
either runner. Repaired the fixture to current law AND added vector
measure-without-kind-is-malformed to pin the divergence itself, so the
fork is now a standing red instead of a silence: Python 10/10 green, Rust
fails that one vector with "decision expected refer got permit".
DELTA 2026-08-12 — the two evaluators now CORRESPOND; the habit stays red
because correspondence is not collapse. Four divergences were measured
(each a golden vector that failed before and passes after) and closed:
(1) Rust returned Ask for ANY unresolvable validator regardless of the
rule's declared class, hardening advisory `inject` rules into blocking
referrals — the existing vector used `class: deny`, where the two agree by
coincidence of severity, so the corpus proved the half that matched;
(2) Rust's provider could not tell "declared elsewhere" from "unknown", so
the three python-only validators routed to judgment there;
(3) the L6 measure.kind split above; and (4) `malformed-manifest-refers`,
SKIPPED in Rust with a reason recording the gap as out-of-scope rather
than as the fork it was. Directions were opposite — (1)(2) made Rust more
blocking, (3)(4) less — so wiring a gate before this would have
false-blocked every write under elohim-storage/src while still missing
malformed governance. Repaired by ONE shared scope map
(governance-validators.json, read by both hosts) plus a native integrity
layer mirroring validate_meta check-for-check; verified over the same 123
live manifests, both hosts flagging the identical 2 (stale worktree copies
predating L6) with matching rule indices and message text. The Rust runner
moved to epr-cli because against NoValidators it could not have detected a
provider divergence of any kind. `epr govern` now exists — the verb that
evaluates a PROSPECTIVE write — which is what "make the hook a client" was
always missing: `epr check` reads content from disk and cannot answer the
authoring question at all. Cross-checked at 129 prospective writes across
all 43 governed subtrees: 129/129 decisions identical, 127/129 identical
down to rule id, the two differing by declared scope, not drift.
GREEN 2026-08-12, same day, against the three-part flip condition stated
when it was filed — registry is one list; every decision carries an
evaluator_cid; a recorded disagreement names the evaluator CID it disputes.
(1) ONE LIST: governance-validators.json, read by both hosts, with the
accounting enforced (no undeclared implementation, no unhonoured
declaration, and no predicate implemented twice).
(2) EVERY DECISION NAMES ITS EVALUATOR: `epr govern` stamps
{id, version, cid} where cid is sha256 over the binary itself; the Python
evaluator addresses its own source the same way; witness() stamps one on
every ledger row. Verified live — a real row now carries
cid sha256:ff5d86ed…, the build that decided it.
(3) THE DISPUTE PATH PRODUCES THE ARTIFACT: both decision surfaces run
both hosts, `epr` is the authority, and a decision-level disagreement is
witnessed with refer.reason=evaluator-disagreement plus BOTH content
addresses ("disputed:" the losing build, "authority:" the deciding one).
Proven by controlled injection — a stub evaluator returning `refuse` where
Python returned `permit` — which is the honest way to test a dispute path
whose whole purpose was to have no live instances left. Ledger row
verified end to end.
Two design corrections are load-bearing and are why this is not merely a
passing test. Authority applies to the DECISION, not the advice: when the
hosts agree, Python's class/rule/reason are kept, because three validators
are scoped `python` by declaration and their `inject` advisories exist
nowhere else (guard Risk 1b). And the degraded notice fires only where a
decision was actually made — a clean allow stays SILENT per the polarity
law, and is debounced besides, so a missing binary informs once rather than
bannering every governed write.
FALLBACK (operator decision 2026-08-12): when `epr` cannot be resolved the
Python evaluator decides and says so. `epr` is not on PATH — it is compiled
into a branch-scoped cargo-pool slot — so "refer when the binary is
missing" would fire on essentially every governed write rather than in a
rare outage. Referring is reserved for the genuine absence the polarity law
is about: neither host able to answer.
CEILING CLOSED 2026-08-12 (same day): `epr` is now established per session
by .claude/hooks/epr-evaluator-guard.py, so the native evaluator decides by
default rather than when a pool slot happens to be warm. The guard is
`async: true` and can never fail a session — the same reasoning that keeps
it out of devfile `postStart`, where a failed cargo build aborts
whole-workspace startup. It has TWO modes and only one compiles: THIS repo
is the evaluator's home, so here the binary is built from the tree and
rebuilt when sources move (dogfooding — the repo that authors the evaluator
is governed by it at the commit it currently is, and a stale binary would
otherwise decide with old law while stamping a CID that says so only after
the fact). A CONSUMER workspace receives `epr` PREBAKED in its image; there
the guard refuses to build and names the packaging gap, because
"compile if missing" as a universal rule turns every consumer into a build
host and hides image regressions behind a slow local success. Measured:
0.15s when current, ~0.6s to reinstall on a warm slot, and all five paths
(present · missing-in-dev · missing-without-sources · stale · idempotent
re-run) verified.
STILL NOT CLAIMED: the prebaked consumer image itself. No workspace image
bakes `epr` today, so a consumer workspace lands on the announced Python
fallback and the guard says exactly that. The dev surface is closed; the
packaging surface is named, not built.
DELTA 2026-08-15 — the ledger row gained WHO-ACTED beside
which-evaluator-decided: `epr govern --session` stamps the session's
registered ActorClaim ({claimed, definitionCid, source: claim|unclaimed});
every identity-sidecar failure reads `unclaimed` — the identity plane
never breaks the decision authority, and a tampered claim reads as
unclaimed, never as the identity the tamperer wrote. Threaded end to end
(resolver stdin session_id / git-gate env → --session → witness(actor=…),
key present only when the plane was consulted). First witnessed exhibit:
the 15:15:50Z refuse row naming agent:scribe@claude-opus-4-6 by package
build (definitionCid sha256:e117a5a9…) during the actor-plane spec's own
authoring. Spec: 2026-08-15-actor-plane-inflight-identity-claims-design.md;
a2o @concern:agent-identity-claim-and-acceptance
(genesis/a2o/features/devflow/, @wip — authored, unstepped).

DELTA 2026-10-12 — Native seam guidance implemented under the existing validator provider: four owner-local classifier families, 28 labeled cases, eight native acceptance tests, Claude Write/Edit and Codex apply_patch adapters; Antigravity invocation explicitly deferred. The classifier is governed algorithm-content with separately pinned application and compiled-method identities; changed proposals, declarations, source bytes or usage policy invalidate review, while a valid abstention cannot hide failure. Active-agent synthetic exercises routed duplicate sync to AutomergeSync and kept legitimate SDK composition in both adapters; judgments were by the implementing agent, not an independent model evaluation. Proof: just gate eprfs GREEN (1,202 tests, zero failures/ignored; 10-minute soft-budget notice), final focused native/parity/live-manifest tests GREEN, schema gate 3,556/3,556, 3 Node tests, 5 Python tests, package verification 2,198 assertions. Wrapper latency and exact review receipts are local at genesis/a2o/reports/recovery/native-seam-guidance-2026-10-12/active-agent-exercises.json; normal maxima below 200 ms, forced timeout visible below two seconds. Registered/projection-present is not trusted activation; deployed PATH epr availability is separate, and EPR_SEAM_BIN can select the verified build. Coverage remains exact UTF-8 writes/patches, lexical candidates and caller-supplied review resubmission; arbitrary shell writes and semantic correctness are not certified. Source inspection outside recall was direct local reading, not reconciled collective memory. No raw drafts, classification state or recall receipts were published. Habit status remains green.

DELTA 2026-10-12 — Installed seam evaluator into the existing PATH epr target after a successful locked native build refresh. Both registered Claude/Codex commands passed duplicate and composition smoke checks without EPR_SEAM_BIN; reports match, four invocations took 127–181 ms, and epr doctor reports READY. Local installed-integration.json pins binary SHA-256 and method identities. Codex native hooks/list recognizes the hook without errors and reports enabled=true, trustStatus=untrusted: normal /hooks review is the remaining activation step. Claude live lifecycle invocation remains unobserved. Evidence: genesis/a2o/reports/recovery/native-seam-guidance-2026-10-12/.

DELTA 2026-10-16 — `python3 .claude/scripts/_lib/__tests__/validator_registry_parity_test.py` reports `11 validator-accounting assertions passed ✅`; the Python leg holds, cargo legs were not run and remain unmeasured this run. Habit status remains green.

DELTA 2026-10-28 (rescue absolutes guard; status unchanged): restored 0ee5dfde8's Python classifier, frame, advisory dispatch policy, landing measures and root binding; the post-write hook is a package-master HookPackage projected through current dev tooling. Classifier 53/53, hook 8/8, frame atoms 82/82, policy 81/81 and evaluator 42/42 checks pass (exit 0); package verification passes 2260 checks (exit 0). Canon lift and policy pins regenerated and fresh. The native registry declares this validator Python-only; no native parity or blocking enforcement claimed.
