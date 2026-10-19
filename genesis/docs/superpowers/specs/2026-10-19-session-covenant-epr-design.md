---
title: epr covenant — the authority half of the session covenant
id: session-covenant-epr-design
status: Draft
class: architecture
date: 2026-10-19
context-tier: disclosed
steward: human:matthew
author: "Claude (Opus 5.5), at the operator's direction, session 182f26d8"
tags: [governance, epr-meta, covenant, variance, steward-of-record, brit, mishpat]
graduation-trigger: >-
  `epr covenant` and the brit `covenant` verbs land with their tests green, the blind-reader hook
  reads a held variance on the household tree, and the rule-variance-under-covenant habit carries
  that evidence; then this spec decomposes into the habit atom and the brit spec.
cites:
  - "epr-meta-policy-registry-measure | `.epr-meta` Policy Registry + Measure Tier | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md"
  - "vision-design-ai-covenant-2026-06-14 | VISION DESIGN PASS | sha256:b43dfd073e45e4b1 | path: genesis/docs/superpowers/specs/2026-06-14-elohim-substrate-passes/VISION-DESIGN-ai-covenant-2026-06-14.md"
  - "human-participant-actor-plane-design | Human Participant in the Actor Plane | sha256:5f51c8e1c60887b8 | path: genesis/docs/superpowers/specs/2026-09-23-human-participant-actor-plane-design.md"
---

# `epr covenant` — the authority half of the session covenant

Plan: `/projects/.claude-config/plans/yes-plan-to-build-misty-flurry.md`. The mechanism half lives in
brit: `elohim/brit/docs/specs/2026-10-19-session-covenant-design.md`, on branch
`feat/session-covenant`. brit records, signs and lists clauses through `brit-build-ref covenant`.

## What it is

Once per agent session, the steward of record answers one bundle of asks. Each answer becomes a
clause that brit signs and indexes. `epr` decides what the bundle contains and whether each clause
is **honoured**. brit decides neither, by its own boundary. Hooks only ask `epr`; they never derive
these answers themselves.

A **variance** is a clause that lowers one `.epr-meta` rule's obligation to the floor that rule
itself declares, for one session and a set of paths:

```yaml
parameters:
  variance:
    floor: <the lowered obligation, verbatim>
    requires: steward-of-record
    offered-when: <when the agent should offer it>
```

Variances are born narrow. Only `inject` and `dispatch` rules can have one, and only when they
declare `parameters.variance`. A variance can only attenuate: it selects the declared floor and a
subset of the rule's paths. It never widens either.

## Commands

Every command takes `--repo R --session S`. With `--json`, the answer is one JSON object on stdout.

A **non-zero exit means the covenant could not be read**: brit is missing, a ref is bad, or parsing
failed. Callers then apply the full rule, so an unreadable covenant never lowers anything.

`brit-build-ref` is resolved from `BRIT_BUILD_REF_BIN`, then from `PATH`.

### `epr covenant offer`

```json
{ "session": "S",
  "standing": "human:matthew" | null,
  "pending":  [ { "kind": "affirmsStewardOfRecord", "question": "…" },
                { "kind": "declaresPurpose",        "question": "…" } ],
  "variable": [ { "rule": "a2o-story-blind-reader-review", "manifest": "genesis/a2o/.epr-meta",
                  "scope": { "write": "*.feature" }, "floor": "…", "offeredWhen": "…",
                  "requires": "steward-of-record" } ],
  "held":     [ <honoured clauses, as in show> ] }
```

- `pending` lists `affirmsStewardOfRecord` until the session holds an honoured one, and
  `declaresPurpose` until the session holds one.
- `variable` lists every rule in the repository's `.epr-meta` manifests that declares
  `parameters.variance`, except rules already held for this session.

### `epr covenant record`

```
epr covenant record --kind <affirmsStewardOfRecord|declaresPurpose|variesRule|revokesClause>
                    --party human:<handle> --quote "<the party's own words>"
                    [--purpose TEXT --write-set GLOB…]   # declaresPurpose
                    [--rule ID --paths GLOB…]            # variesRule
                    [--revokes CID]                      # revokesClause
                    [--valid-until RFC3339]
```

A refusal exits non-zero with a one-line reason and writes nothing. `record` refuses when:

- `--party` is not a `human:` handle with standing on this device. Standing is the same read
  `epr actor current` makes (`Standing`, `actor.rs`): the session's human claim, or the device's
  roster-bound standing.
- The kind is `affirmsStewardOfRecord` and the party is not an active, non-fixture Steward. This is
  the predicate `flow memory import --steward-of-record` uses.
- The kind is anything else and the session holds no honoured `affirmsStewardOfRecord` by the same
  party.
- The kind is `variesRule` and any of these hold:
  - the rule is not found;
  - its class is neither `inject` nor `dispatch`;
  - it declares no `parameters.variance`;
  - a path falls outside the rule's manifest directory.
- The kind is `revokesClause` and the target is not in this session or belongs to another party.

For `variesRule`, `epr` builds the payload it hands to brit. The caller never supplies it:

- `rule`, `manifestPath`, `paths` from the request;
- `manifestCid`: the raw-codec CID of the manifest's current bytes (`elohim_epr` CID, codec `0x55`);
- `floor`: copied from the rule.

On success, `record` calls `brit-build-ref --repo R covenant put … --output-json` and prints
`{"status":"recorded","cid":…,"clause":…}`.

### `epr covenant check --rule ID --path P`

```json
{ "status": "held" | "not-held", "variable": true|false, "clause": "<cid>"|null,
  "floor": "…"|null, "party": "human:…"|null, "reason": "…" }
```

A clause is **honoured** for `(rule, path)` only if all of these hold:

1. Its kind is `variesRule` and its `rule` matches.
2. `P` matches one of its `paths` globs, relative to the repository.
3. Now falls within `[validFrom, validUntil]`. A null `validUntil` means the rest of the session.
4. No later `revokesClause` by the same party names it.
5. The session holds an honoured `affirmsStewardOfRecord` by the same party, and that party still
   has standing.
6. `manifestCid` still equals the CID of the manifest's current bytes, and the rule still declares
   the same floor.

An edited rule voids its variances, because the attenuation was granted over different bytes; the
reason then says `rule edited since the clause`. `reason` always names the test that decided.

A rule that declares no variance at all gets `variable: false` and `status: not-held`.

### `epr covenant show`

```json
{ "session": "S", "steward": "human:…"|null,
  "purpose": { "purpose": "…", "writeSet": ["…"] } | null,
  "variances": [ { "clause": "<cid>", "rule": "…", "paths": ["…"], "floor": "…",
                   "party": "…", "validUntil": null } ],
  "clauses":   [ { "cid": "…", "kind": "…", "party": "…", "quote": "…", "validFrom": "…",
                   "honoured": true|false, "reason": "…" } ] }
```

`variances` contains only honoured clauses.

## What rung 1 does not claim

brit signs each clause with the repository's brit agent key, at `<git-common-dir>/brit/agent-key`.
That key is not yet bound to the device roster. At rung 1, then:

- the signature proves which repository store recorded the clause;
- **attribution of the human** rests on `epr` standing and on the session's affirmation by that
  same human.

Later rungs:

- **Rung 1b:** bind the brit key to the roster.
- **Rung 2:** the human's own key co-signs.
- **Rung 3:** the clause is notarized as a Mishpat `Commitment` with action `varies-rule`. This is
  coordinator-only, so the DNA hash does not move.
