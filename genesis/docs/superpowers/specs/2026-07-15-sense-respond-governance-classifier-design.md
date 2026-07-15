---
title: "The Sense-and-Respond Governance Classifier — the Deterministic Floor's Afferent Nerve to the Elohim"
id: sense-respond-governance-classifier
tier: spec
status: Draft
created: 2026-07-15
maintainers: Matthew Dowell + Opus 4.8
class: process-meta
process_subdomain: governance-substrate
topic: [governance-classifier, deterministic-floor, elohim-ceiling, sense-and-respond, vsm, algedonic, selective-prediction, abstention, frame-ontology, rea-feedback-signal, escalation-ladder, no-overwhelm, progressive-revelation, agent-agnostic]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
sovereignty-frame: descriptive  # this spec DEFINES the classifier that resolves apex-concept frames; it quotes "self-sovereign" etc. as the concepts being governed, never asserts them — suppresses sovereignty-guard-signal
refines:
  - genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
cites:
  - epr-meta-compose-gate | the compose-gate substrate this classifier plugs into — validator-EPR seam, verdict/combine cascade, Precedent-shaped policy registry, acyclic-by-construction | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - elohim-ceiling-design | the ceiling half — audit-the-guardian-not-trust-it (Principle 8), JudgmentCall, assemble→contest→decide→dissolve, No-Backdoor fresh-quorum invariant, Stances II.1–II.3 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md
  - values-forward | the classifier peer — declared-up-front-and-refusable-in-advance; the four-part stance; the human sortition floor sovereign over the machine, no self-sovereign apex | path: genesis/docs/content/elohim-protocol/values-forward.md
  - self-healing-control-plane-design | the structural no-overwhelm invariant, detect→recover→verify→elevate, no-runtime-write-to-.claude/data (external poller), the WANT→GapTracker→REA-Commitment loop | path: genesis/docs/superpowers/plans/2026-06-13-self-healing-control-plane-design.md
  - substrate-trust-contract-runbook | probe-names-itself discipline; on-demand-not-continuous surfacing (the cadence half of no-overwhelm) | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - stewardship-over-sovereignty | the canon the first apex-concept guard enforces — recovery ladder, bounded_by spine, soft-warn ceremony over hard lock | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - .claude/skills/epr-content-addressing/SKILL.md
  - .claude/skills/semantic-links/SKILL.md
---

# The Sense-and-Respond Governance Classifier

> **What this is.** A single shared primitive — the frame/intent classifier — that every values guard in the protocol
> consumes. It generalizes what the two live guards (`epr:validator-p2p-design-gate`,
> `epr:validator-sovereignty-ontology-guard`) hand-roll. It is **not a better content filter**. It is the protocol's
> distributed *sense-organ*: a bank of cheap deterministic gates at the point of byte emission that resolve the routine
> case locally and — the load-bearing move — **know the shape of their own ignorance and route it to the tier that can
> resolve it.** Its terminal endpoint is not a database verdict. It is the **elohim**.

## 1. Thesis — the elohim are the endpoint

The elohim of the Elohim Protocol are bounded, inspectable, correctable AI wisdom agents — first-class participants in
governing shared resource pools, never tool, never sovereign (`values-forward`, Stance II.3, *Phased*). This spec states
where they get their senses and how their judgment is applied:

**The classifier is the elohim's afferent nervous system. The deterministic floor is the nerve ending; the elohim are
the endpoint.** A guard fires at the point a byte is written; what it can classify cheaply, it resolves locally; what it
*cannot* classify — the un-decidable frame, the abstention — escalates, and the **terminal resolving *endpoint* of that
afferent path is the elohim**: where the distributed floor's sensing is integrated into wisdom-judgment. They are the
endpoint of *sensing*, never the seat of *sovereignty* — the human sortition floor (Stance II.2) remains **sovereign** and
can override any elohim resolution; the elohim are first-class participants, never sovereign (Stance II.3). "Endpoint of
the classifier" names where the nerve terminates, not who holds the final vote. This
is what makes governance *"a daily capability, not a periodic vote"* mechanically true: the elohim govern **continuously**
because the classifier fires at every edit and routes the hard frames to them — a Habermas-machine pre-embedded at the
edge of the network, the marginal cost of surfaced wisdom trending toward zero.

And because **every hop is bounded, inspectable, correctable, and REA-signalled in both directions**, the elohim cannot
enclose the commons they steward. That is the anti-enclosure thesis made structural: the sense-organ that lets AI
participate in governance is the same mechanism that keeps that participation auditable and revocable. *Audit the
guardian; do not trust the guardian* (`elohim-ceiling-design`, Principle 8).

### 1.1 The prior-art spine (why this shape, not a classifier)

The design lives with a weakness rather than pretending to solve it. Content-moderation research shows **no shallow
lexical signal is trustworthy in either direction**: it over-fires on mention/quotation/counter-speech/reclaimed-language/
satire and under-fires on dog-whistle/obfuscation — the *same* shallow weakness manifesting as opposite-direction errors.
So the classifier is a **selective predictor with a first-class `abstain` output** (Chow's 1970 error–reject tradeoff;
SelectiveNet), where abstention *is* the escalation trigger, not a failure. The core principle you named — *"anything
that touches a guard that cannot be classified is a prime subject for escalation"* — is Chow's reject option wearing a
governance costume, and Stafford Beer's **algedonic bypass** (a pain/pleasure alert that skips the hierarchy to demand
attention) is the escalation *path*. Ashby's Law of Requisite Variety is the reason it is a *shared* primitive: each
guard supplies the local variety for its own concept-space; the shared classifier supplies the common frame-resolution
machinery.

### 1.2 One instance of a general primitive — the eprfs witnessed-interaction event

The machinery here — a **local witness** emitting an **REA event** about an interaction with a **content-addressed
object**, whose **peer-sync turns local witness into validated (notarized) witness**, aggregating on the object's CID,
**denominated in the right substrate** — is not specific to governance. It is the general shape of an **eprfs
witnessed-interaction primitive**; this spec is its *governance-domain* instantiation:

| instance | interaction event | witness | denominated in | aggregates on |
|---|---|---|---|---|
| **this spec (governance)** | an edit touched an apex-concept frame | the guard + classifier | affirm/dismiss (`FeedbackSignal`) | the rule CID |
| `.mp3` | minutes listened | the light-runtime | attention (minutes) / energy | the track CID |
| `.mkv` | minutes watched | the light-runtime | attention (minutes) | the video CID |
| `.epub` | pages read | the light-runtime | attention (pages) | the book CID |

A **light-runtime on the machine tracks the actual events** — including *offline* consumption — and **syncs to peers,
which validate it into a witnessed record**: the same local-witness → peer-validated → notarized ladder the classifier's
`classifier_agent` (self-report → coordinator-attestation, §5/§11.4) walks. Value is **denominated in the attention/energy
substrate the interaction actually spends** — an honest reconciliation the shefa economics can aggregate (REA: Resource =
the object; Event = the interaction; Agent = the person; the substrate = the unit).

**This spec deliberately does *not* try to be that general primitive.** The *governance* classifier is contextual to
authored bytes (social posts, documentation, code), where the interesting event is *what frame a byte asserts*. The
consumption primitive (mins-listened / pages-read / joules-spent, offline-reconciled, peer-witnessed, attention/energy-
denominated) is its **sibling instance** and earns its own spec. Both share the skeleton — *local witness → peer-validated
→ REA-aggregated on the object CID* — differing only in *substrate* and in whether the witnessed quantity is a
*classification* (governance) or a *count* (attention/energy). Read this spec as **proving the skeleton on the hardest
substrate — contested meaning** — so the easier, countable substrates inherit it. Parent seed:
`eprfs-witnessed-interaction-primitive` (follow-up spec).

## 2. The honest floor claim (the hinge — ratified as the v1 default)

At v1 there is **one operator, no sortition floor, no coordinator-derived identity, no DHT anti-gaming**. Under those
conditions you cannot simultaneously have a genuine un-lobbyable *blocking* floor **and** honest escalate-don't-block
context handling — with one human and a spoofable counter, one of the two is theatre.

**We choose the sensing floor.** v1 is **advisory-sensing only: no terminal mechanical `deny` at edit-time, and no
automated *arming* — a new trigger, a new frame, a promotion, or any *tightening* of the floor requires human
ratification.** The one automation permitted runs *only in the fail-safe direction*: the system may **loosen itself**
without a human (FP-rate demotion, TTL expiry — §12.4), never **tighten itself** without one. That asymmetry is the whole
safety posture — a runaway can only ever relax the floor, never clamp it. The
"deterministic, un-lobbyable refusal" of the mechanical floor (Stance II.1) is **declared-but-reserved** until the tiers
it escalates *to* — the human sortition floor and the phased elohim ceiling — actually exist. We say this plainly rather
than ship a `deny` that would deterministically censor the highest-value speech on the highest-stakes topics with **no
appeal path**, before the appeal path is built. This is the single decision the whole primitive swings on (§15, D1), and
it is ratified here as the default. `values-forward`'s "refusable in advance" cuts both ways: we refuse, in advance, to
pretend a floor is un-lobbyable when its appeal tiers do not yet exist.

## 3. Architecture — two layers, two clocks

The primitive runs on **two independent clocks that must never collapse into one**:

| | **Edit-time (synchronous)** | **Post-landing (asynchronous — "once in a while")** |
|---|---|---|
| Seam | PreToolUse `epr-meta-resolver.py` **+ the harness-neutral git/CI choke point (§6)** | PostToolUse `classifier-signal.py` → external poller `classifier-recall-harvest.py` |
| Handles | **Family-1** (word present, assertion absent): agent-in-loop self-classifies against the surfaced frame rubric | **Family-2** (assertion present, word absent) + accumulated abstentions + evidenced dismissals |
| Cost | Cheap, deterministic — **no LLM in the hook** (a fixed constraint) | Expensive semantic recall, batched, off the write path |
| Cadence | Every write; Layer A over-fires by design (recall-first) | Threshold- or audit-sample-triggered; rare by design |
| Collapse failure | An LLM call per edit (too slow/costly) | Family-2 landings never surface (no trigger word → the agent is never prompted — the recall gap) |

- **Layer A** = the high-recall / low-precision trigger: a bank of **weakly-supervised labeling functions**
  (Snorkel / data-programming shape), one per failure mode, each with a modeled reliability, each of which may only ever
  *over-fire*, never suppress. Suppression is Layer B's job, exclusively.
- **Layer B** = the frame/intent resolver, split by *family*: **Family-1 resolved synchronously by the agent-in-loop**
  (the one entity with the authorial context the classifier lacks); **Family-2 resolved asynchronously by post-landing
  semantic recall.** Abstention is the baton passed between the layers.

## 4. The shared-primitive dependency-injection contract

The classifier is **one** registered validator, consumed by **every** values guard. Guards do not *call* it — they
**declare a binding** naming their apex-concept, and the engine injects the shared machinery. Adding a *guard* is a policy
row plus one Layer-A labeling function — that part touches no engine code. Wiring the *classifier itself* into the engine,
however, is **not** free (contra an earlier draft): the routing this spec specifies — unregistered→`ask`, the fire-vs-clear
pre-pass, the `defer` verdict, the algedonic side-channel — are edits to `_eval_rule`/`evaluate`/`combine`, and `evaluate`
is documented **PURE** (`epr_meta.py:534`: *no writes, no side-effects*; it is re-invoked on the same payload by
`measure_census`/`subtree_coverage`). So the rich `FrameClassification` (§5) **must thread through the `Verdict` return
channel** (extend `Verdict`), *not* ride on a mutated `write["_frame_classifications"]` — mutating the input dict would
break `evaluate`'s purity and be order-dependent across its re-invocations. The seam is the existing `dict -> bool`
validator interface (`REFERENCE_VALIDATORS`), a one-line registration; `_eval_rule`'s dispatch is already generic, and
`evaluate` already *accumulates* every fired verdict — but `combine` is a **max-severity selector** (it keeps the top
verdict, discards the rest), so it is the accumulation/selection primitive, **not** the co-resolution step. The actual
defeater-vs-trigger co-resolution is the §10.1 pre-pass. This engine wiring is tracked as a gap in §16.

## 5. The unified verdict shape — `FrameClassification`

One classification act — the co-resolution unit that joins the existing cascade:

```
FrameClassification {
  target_cid        # CID of the write/EPR being judged        (v1: path digest)
  frame_ref         # CID/ref of the frame atom this is about
  verdict           # legitimate | drift | abstain             (three-way; abstain is first-class)
  confidence        # calibrated in [0,1]                       (coarse proxy at v1 — §9.4, §15 D2)
  evidence {
    spans                    # spans in the ORIGINAL content (Layer A scans a normalized shadow)
    matched_recall_signal    # which labeling function(s) fired
    rubric_answer            # the agent's Family-1 self-classification, when present
    reason_ref               # v1: path: cite to the story-of-why; graduated: CID (§13)
  }
  classifier_agent  # v1: self-reported {claude|codex|gemini|human}; graduated: coordinator-derived agent_info()
  bounded_by        # optional Commitment CID, if issued under delegated steward authority
}
```

Three commitments this shape carries: (1) **`abstain` is a verdict, not an error** — it is the escalation trigger;
(2) **`confidence` must be calibrated to be sound** (Chow/SelectiveNet) — at v1 it is a coarse proxy and the abstain-band
is set deliberately wide (§9.4); (3) **`classifier_agent` is coordinator-derived and unspoofable only at graduation** — at
v1 it is self-reported, so **all v1 signals are advisory-weight only** (§11.4). The engine-facing triple
(`cls`, `reason`, `rule_id`) is derivable from this shape, so the existing `combine()`/`_emit_*` paths keep working.

## 6. The seam + the harness-neutral choke point (agent-agnosticism)

The union's most-repeated flaw — every crux critique hit it — is that a PreToolUse loop is **Claude-Code-specific**:
Codex/Gemini may not honor `ask.reason`, and a human in `vim` running `git push` fires *no hook at all*. "Agent-agnostic"
is a property of the content-addressed artifacts, falsely generalized to the enforcement seam.

**Resolution: the authoritative gate is harness-neutral.** A git `pre-commit` / server-side `pre-receive` / CI check runs
the *same* `frame_classifier.py` and surfaces the rubric in its message. The PreToolUse hook is retained as an *optional
early-warning convenience* for harnesses that support it. **Frame declarations live in the bytes** (an in-content marker,
§9.2), never in a Claude-specific ledger — so any human, any agent, any tool declares a frame the same way, and the async
pass reads markers *from content*. Agnosticism becomes a property of *where the seam sits*, not a hope about shared
behavior. Four touch-points — three additive, one (the §4 engine routing) an explicit engine-modification gap (§16):

1. **Register the shared classifier** — one line in `REFERENCE_VALIDATORS`. Respect the load-bearing quirk: an
   unregistered `validator:` ref downgrades to `class: inject` regardless of declared class — **not silently**; it emits an
   advisory reason (`epr_meta.py:325`). Register before binding. The v1 policy on a validator that *cannot run* is split
   by cause (§10.1): a *specific* validator unregistered on a blocking rule → `ask` (a `deny` that cannot run is an open
   question, not an advisory); the *shared classifier module* failing to load → `inject` circuit-breaker, so one broken
   import cannot `ask`-storm every guard at once.
2. **The pre-commit / pre-receive / CI gate** is the authoritative *sensing* seam — but at v1 it is **advisory-annotating
   only**: it posts a PR comment / prints the rubric as a warning and **`exit 0` always**. A git/CI check is binary
   (`exit≠0` *is* a terminal deny — the one §2/D1 forbid — with no `inject`/`ask` projection onto an exit code), so a
   *blocking* CI status is a graduated capability that shares the terminal-`deny` RESERVED row (§14). The PreToolUse hook
   stays the richer edit-time convenience; the git seam carries strictly less verdict-expressivity (§16).
3. **Generalize the PostToolUse signal hook** — lift `sovereignty-guard-signal.py` (today one companion for one rule)
   into a shared `classifier-signal.py`, keyed on *which* validator fired, sharing the **same detector functions** the
   PRE side uses ("so the ledger and the gate can never disagree" — the invariant the sovereignty hook already holds).
4. **Add the external poller** `classifier-recall-harvest.py` — a `runtime-harvest.py` clone that reads the ledger + a
   cursor, runs the Family-2 semantic pass on new deltas, dispatches `guard-recall-triage` on *new* fingerprints only.
   **Never in-process** — the no-runtime-write-to-`.claude/data` L4 rule holds; the poller is external.

## 7. The Frame Ontology (typed, content-addressed)

A guard does not *contain* its theory of "when a trigger word is really an assertion." It **cites** a shared **Frame
Ontology**: one content-addressed atom per failure mode, plus a thin collection atom `frame-ontology@1` that `cites:`
every frame. Adding a frame is one row + one cite; no guard changes. The ontology is **data, not a model prompt** — the
identity-layer guarantee of agent-agnosticism (any model reads the same text; any model re-derives the same CID).

**Frame atom schema:** `id`/`version`; `family` (`defeater` = Family-1 | `detector` = Family-2); `polarity`
(`legitimizing` | `incriminating`); `linguistic_definition`; `rubric` (the question the agent-in-loop answers);
`recall_signal` (Layer-A labeling-function spec); `cost_class` (`high-fp-cost` | `high-fn-cost` | `symmetric` — carrying
its own thresholds); `binding` (Precedent ladder tier); `cites`; `established_by` / `superseded_by`.

- **Family-1 — defeaters** (`legitimizing`, resolved by agent-in-loop, `high-fp-cost`): `frame-negation`, `frame-satire`,
  `frame-use-mention` (quotation/reported speech), `frame-definition`, `frame-hedged-bounded`, `frame-historical-external`,
  `frame-bridge-legibility`, `frame-pedagogical`, `frame-fiction`. A confirmed defeater maps trigger → **legitimate**.
  These are `high-fp-cost` because over-flagging satire/counter-speech/reclaimed language is a *first-order harm*, not a
  safe fallback.
- **Family-2 — detectors** (`incriminating`, resolved async, `high-fn-cost`): `frame-euphemism`, `frame-obfuscation`
  (leetspeak/homoglyph), `frame-translation-dogwhistle`. **Load-bearing constraint (§9.3):** a detector may only ever
  emit `abstain`, never `drift` — a keyword-silent semantic catch *is* "not knowing," and the author had no
  frame-declaration opportunity.

The typing is the design's spine: defeaters are a false-positive-suppression problem the agent self-serves at edit-time;
detectors are a false-negative-recovery problem no edit-time trigger can surface. One threshold cannot serve both.

## 8. Layer A — the deterministic recall trigger

`_lib/frame_classifier.py` exposes `triggers(write) -> list[TriggerHit]`. Each labeling function is one narrow, auditable
heuristic tagged with the apex-concept/frame it senses — generalizing exactly what the two live validators hand-roll
(`_p2p_design_gate` substring-scan; `_sovereignty_ontology_guard` net-new count against `_SOV_APEX_PHRASES`). **Invariant
(schema-checked): an LF may only over-fire; suppression is Layer B's job exclusively.**

**Obfuscation/homoglyph hardening.** Every scan runs against a **normalized shadow copy** built once per `write` and
cached on the payload dict: (1) NFKC; (2) ICU confusable-fold (Cyrillic `а` U+0430 → Latin `a`); (3) leetspeak/de-obfusc
fold (`4→a`, `@→a`, `$→s`, collapse repeats, strip zero-width/interior punctuation); (4) bounded Levenshtein backstop
against apex keyphrases. LFs scan the normalized copy, report spans in the **original**. This is an arms race, not a
solution — the un-normalizable residue is precisely the recall gap Family-2 async recall (§9.3) exists to catch. Layer A
buys the cheap 80%; §9.3 catches the concept-without-keyword tail.

**Scan-size cap (fail-degraded, never fail-open).** `epr_meta.py` caps *manifests* at 64KB but not *scanned write
content*; a megabyte of filler — or an embedded base64/minified blob — around the drifting sentence would blow the
~3000ms hook budget and fail the hook *open*, landing drift on the very edit-time layer §12.1 relies on. So Layer A
**caps its scan at the first N KB, degrades to a cheap substring-only pass beyond it, and logs a `measure`
`unscanned-tail` verdict** — recall-preserving degradation, never a silent fail-open. (§12.1(b) bulk-write detection
defers *async dispatch*, not this *synchronous* scan.)

**Layer A alone cannot deny**: a bare Layer-A hit degrades to at most `ask`/`inject` (mirroring the engine's
unregistered-validator asymmetry) — and at v1, per §2, nothing denies terminally.

## 9. Layer B — agent-in-loop (Family-1) + async semantic recall (Family-2)

### 9.1 Family-1: the agent-in-loop frame rubric

When Layer A triggers a defeater-class concept, the resolver surfaces the frame's `rubric` as the message. This is not an
accusation — it is a request to **declare the frame** so the record is honest: *if a defeater holds (negation /
use-mention / satire / definition / hedged / historical-external / bridge-legibility / pedagogical / fiction), add the
in-content marker; if this is an assertion in the protocol's own voice and intended, this is a real drift and it
escalates.* The agent's declaration **is** its `FrameClassification{verdict: legitimate}`. This is sound because the
use-mention distinction is recoverable *when you can see intent*, and the author is the one entity that can.

**v1 honesty — the floor-claim extended to the defeater layer.** That soundness holds *under author honesty*. At v1
`classifier_agent` is self-reported and the author is frequently an autonomous agent: an adversarial or jailbroken author
simply declares the defeater, and the Family-2 backstop is shadow-log / abstain-only / advisory (§9.3–§9.4). So **v1
Family-1 resolves *cooperative* authors only** — against a mislabeling author it has zero adversarial resistance and
Family-2 is inert. Adversarial-frame resistance is a *graduated* property, gated on coordinator identity + an armed,
evidenced Family-2 (§14). This is exactly the §2 floor-claim, said plainly for the defeater layer: the sensing floor
senses *cooperative* authors, not adversaries, until the graduated tiers exist.

### 9.2 The frame marker lives in the bytes (agent-agnostic)

A frame is declared by an in-content marker (e.g. `use-mention-frame:` / `satire-frame:`) — content-addressed and
editor-agnostic: the async pass reads markers *from content*, not from a Claude-specific ledger; the ledger entry is a
convenience projection, the byte is the source of truth. Two corrections the live prototype forces:

- **Block-scoped, not whole-file.** The live `sovereignty-guard-signal.py` suppresses on `_SOV_FRAME_MARKER in
  post.lower()` — the literal substring *anywhere* in the file, including inside a code block or a quoted example. That is
  too coarse: a marker must suppress only the spans **within its enclosing block** (its line range), never the whole
  document. And until Family-2 validation is armed (graduated), an unvalidated marker **downgrades the guard to `inject`
  (still surfaces) rather than fully suppressing** — the marker is *necessary, never sufficient* even at v1, closing the
  "one substring silences the guard for the whole file" hole.
- **A carrier for every content type scanned.** A literal `use-mention-frame:` token cannot live in a `sophia-quiz-json`
  field, a compiled/binary asset, an image, or WYSIWYG/API-emitted JSON — so legit framed content there would be
  un-defeatable and re-escalate on every touch, forever. The frame declaration must be *addressable per content type*: a
  sidecar `.epr-meta` `frame:` binding (path- or CID-scoped), a structured `_frame` field for JSON, a
  language-appropriate comment for code, inline markers for prose. The declaration is a first-class, type-aware locator —
  not an assumed inline-markdown string.

### 9.3 Family-2: async post-landing semantic recall

Runs on the PostToolUse seam because the write has landed (no hot-path cost) and the hook already reconstructs net-new
content. Generalizes `sovereignty-guard-signal.py`: net-new scoring — diff against on-disk for `Write`, reconstruct
pre-edit from `old_string→new_string` for `Edit`; fire only if `net_new > 0` (cleaning edits never log).

- **v1-buildable:** an operator-curated apex-concept embedding index (`.claude/data/apex-anchors.npz`) — exemplar
  sentences per concept (with and without keywords, including known euphemisms/dog-whistles), embedded once with a small
  **local CPU sentence-encoder** (MiniLM-class). *Provisioning is owned, not assumed:* `all-MiniLM-L6-v2` loads offline
  here only because MemPalace baked it into `/opt/mempalace/hf-cache` with `TRANSFORMERS_OFFLINE=1`; a fresh container / CI
  runner / household node with only `~/.cache/huggingface` would silently *network-fetch* it (egress is open),
  contradicting "not a network LLM." So the classifier must either reuse MemPalace's baked cache + offline env explicitly,
  or **vendor + pin the model into its own footprint**, and the household-node case (neither present) must be named. The
  encoder is genuinely feasible; only the provisioning was unowned. Post-landing, net-new chunks are embedded and
  cosine-compared to anchor centroids; a chunk crossing threshold is a Family-2 hit. The genuine
  "assertion present, keyword absent" case a labeling function cannot catch rides the sentinel pattern: a **new**
  fingerprint dispatches a background `guard-recall-triage` agent (model-pluggable — Opus/Sonnet/Gemini/local); a
  recurring known fingerprint does not re-fire.
- **Critical invariant (§15 D3):** a Family-2 hit **may never emit `drift` / `negative` / `debit`**. It emits `abstain`
  only, routed for adjudication, and does **not** retro-block the landed write. The content most likely to be
  keyword-absent-but-loaded is *also* the content most likely to be euphemistic satire or indirect critique — and the
  author never saw a prompt. "The machine found a concept without a keyword" is *not knowing*, never *knowing a violation
  occurred*.
- **First-person / testimony exemption (never author-visible as an accusation).** The same async pass that catches a
  dog-whistle also sweeps survivor testimony in clinical language, academic analysis quoting a slur to dissect it, and
  indirect political critique. Closing edit-time *block*-chill (§10.1) must not open async *audit*-chill on exactly the
  most sensitive legitimate speech: being enqueued for adjudication is itself chilling for a first-person account. So a
  Family-2 abstain is **never surfaced to the author as an accusation**, and first-person / testimony content carries a
  `high-fp-cost` exemption — its surfacing sealed or suppressed below a corroboration threshold, reviewed (if at all) as
  sensitive, never paged.

### 9.4 Calibration cold-start

Selective prediction is only sound if `confidence` is calibrated. At v1 there is no labeled corpus. **Ship abstain-heavy
(wide band, safe), Family-2 in shadow-log mode**, and treat every escalation outcome as an active-learning calibration
sample (§12). The empirical resolve-rate (fraction of low-confidence abstentions resolving `legitimate` vs `drift`) is
reported in the drift tally and re-tuned each ceremony — a living parameter, never a frozen cutoff, never auto-adjusted.
A real early-operator-load cost, accepted as the price of calibration data.

## 10. Uncertainty → escalation ladder — the elohim as terminal endpoint

### 10.1 Abstain routing — async-log by default, sync-`ask` reserved

The deepest split in the source designs was *abstain = synchronous block* vs *abstain = async log*. **Resolution by
polarity/cost-class:**

- **`high-fp-cost` defeater abstention → `inject` (non-blocking) + async-log to the review ledger.** The write lands, the
  story is captured, nothing interrupts. Escalation ≠ interrupt — the case *is* escalated (to the poller surface) while
  friction stays off exactly the legitimate speech (satire/counter-speech/quotation) `values-forward` protects. This
  closes the chilling-by-friction censor bug all five critiques converged on.
- **Synchronous `ask` is reserved** for: a confident defeater-*absent* reading on an `ask`-class guard; existential-class
  triggers *after* frame resolution (§10.3); fire-vs-clear dissent among labeling functions on one write (itself
  uncertainty → `ask`, the §4 co-resolution pre-pass before `combine`); and a *specific* validator unregistered on a
  blocking rule (→ `ask` — a `deny` that cannot run is an open question, not an advisory; the downgrade to `inject` is
  *not silent* — it emits a reason — but on a blocking rule an advisory is the wrong verdict). **Distinguish** this from
  the **shared classifier module failing to load** (a syntax error mid-migration, a missing dep): that makes *every*
  classifier-backed guard unavailable at once, so it degrades to an `inject` **circuit-breaker**, never a repo-wide
  `ask`-storm.

### 10.2 The four-hop ladder — the elohim at the top, the human floor above them

| Hop | From → To | What decides the hop | Floor/ceiling stance |
|---|---|---|---|
| **H0** | byte emission → **mechanical floor** | a Layer-A recall trigger matches — deterministic, always runs | Stance II.1 — refused by the substrate. *At v1: sensing, not blocking.* |
| **H1** | floor → **agent-in-loop** | defeater fired; rubric surfaced; agent self-classifies (Family-1), declares a marker or revises | proto-ceiling: witness + recommend (`JudgmentCall ∈ {approve, deny, escalate, defer}`) |
| **H2** | agent → **human sortition steward** | the agent itself abstains, OR a landed signal contests H1, OR stakes cross a gravity threshold | Stance II.2 — the floor can always override the ceiling; terminus of routine escalation |
| **H3** | steward → **the elohim** | phased, admissibility-gated: a scoped, revocable, witnessed `delegates-compute` Commitment; the elohim witness/recommend until they earn authority act-by-witnessed-act | Stance II.3 (Phased) — *audit the guardian, not trust the guardian* |

The elohim sit at H3 — **the endpoint**: the terminal resolving *endpoint* for the frame the floor could not classify —
the wisdom-integration point of the afferent path, **not** its final authority. The human sortition floor (H2) sits
*above* H3 in sovereignty precisely because H3 is the sensing terminus, not the sovereign: the floor can always override
an elohim resolution, and the elohim are admitted only through a scoped, revocable, witnessed compute Commitment. `escalate` climbs; **`defer`** (the missing dual) re-parks a case at the current
tier with a TTL — "not now, not enough signal, re-present when more lands" — so the ladder is not a one-way ratchet
forcing resolution under pressure. Two invariants carried verbatim from `elohim-ceiling-design`: **gravity forces a
structurally-distinct quorum, never reuse of a lower one** (No-Backdoor); **the floor retains final override at every
tier**.

**v1 degradation, stated honestly.** H2 and H3 both collapse onto one operator; a population of one cannot be "a
differently-composed higher quorum than itself." The sortition invariant is therefore **declared-but-inert** at v1 (we
say plainly it is not enforced). The ladder tops out at "the operator, witnessed." Where an existential misfire must be
resolved before a second steward exists, the stand-in is a **logged, time-delayed operator override with written
justification** — a synthetic "distinct deliberative act" for the quorum that does not yet exist (§15 D1). The elohim as
endpoint is the *designed* terminus; at v1 the endpoint is the operator standing in the elohim's place, witnessed.

### 10.3 Algedonic bypass — rare, unfilterable, post-frame-resolution

Most escalation is orderly and batched. The algedonic channel (Beer) is the exception: a rare, threshold-triggered,
hierarchy-bypassing signal a local guard fires **directly to the steward tier**, structurally unfilterable by intervening
layers (it writes its own ledger line + forces its own banner *in addition to* whatever `combine()` returns; no
downstream rule or debounce can swallow it).

- **Pain fires on:** an existential HARD-BLOCK trigger **AND `framed:false` after frame resolution** — the bypass jumps
  the *counter*, not the *frame gate*, so a history lesson / quotation / counter-speech does not page. This resolves the
  sharpest censor bug in the source corpus: **existential guards do not short-circuit the rubric.** Only a confirmed
  assertion-in-own-voice on an existential concept bypasses; and even then, at v1, it is a *hard-escalate to the steward
  tier* (the algedonic page) plus at most a synchronous `ask` at edit-time — **never** a terminal mechanical deny (§2).
  There is no `block-pending-steward` enforcement class: `ENFORCEMENT_CLASSES` is exactly `deny/ask/inject/measure/dispatch`,
  and v1 forbids terminal `deny`, so "existential" resolves to `ask`-then-escalate, consistent with §15 D1's "hard-escalate
  rather than block."
- **Pleasure fires on:** a guard affirmed legitimate an exceptional number of times → a promotion *candidate* (feeds
  policy adaptation, not incident response). Pleasure never auto-promotes (§11.5).
- **Anti-desensitization:** the author-facing banner debounces per-fingerprint (one mis-tuned existential substring cannot
  banner every edit forever); the *steward-facing* delivery stays un-swallowed.

## 11. REA-backed bidirectional signals + aggregation on the gate

### 11.1 Substrate — `FeedbackSignal`, not a ValueFlows `EconomicEvent`

The graduated signal is a `FeedbackSignal` (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs`),
not a new ValueFlows entry type. It is the closest live analog — a graduated affirm/dismiss vocabulary already keyed on a
`target_cid`, with a `standing_impact` gradient, the proven no-self-vouch coordinator gate, and a natural
`PrecedentToFeedbackSignal` link. We keep the *conceptual* REA framing (Resource = the rule; Event = the signal; Agent =
the signer) — `FeedbackSignal` **is** REA-shaped — while avoiding a two-substrate build.

### 11.2 The rule, the signal, the aggregation point

- **The rule** is a `Mishpat::Precedent` (`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/`): title/reasoning,
  `binding` ladder, `scope`, `superseded_by` lineage. **Its CID is the aggregation anchor.** Because identity is
  content-addressed, two `.epr-meta` manifests binding the same `id@version` resolve to the same CID → the same
  aggregation node → signals from every directory accumulate on **one** rule, auto-deduplicated. No path/name-keyed
  side-ledger can offer this.
- **The signal** is `FeedbackSignal`-shaped, `target_cid` = the **Precedent CID**, linked via `PrecedentToFeedbackSignal`.
  The rolled-up score is a **derived graph projection** (à la `standing_projector.rs`), never a mutated counter — *storage
  is projection, not truth.* **Cost honesty:** adding `vouch_kind: frame-declared | gate-decision-affirmed` edits the
  `const` `VOUCH_KINDS`/`SIGNAL_KINDS` arrays in `content_store_integrity` — an **integrity zome** — so this **moves the
  DNA hash**: a P2P-partition / network-seed-lineage event on the DNA ladder, *not* a cheap "validation-whitelist
  widening." G8 owns the *timing*; the *cost class* (a deliberate DNA-lineage event) must be owned too. And `vouch` today
  carries *restorative* semantics (`accept-correction`/`restitution`, asserted by its own tests) — reconcile that before
  claiming `vouch` "is REA-shaped" for a gate-affirmation.
- **POSITIVE** = `signal_kind: vouch` (`vouch_kind: frame-declared | gate-decision-affirmed`), `standing_impact:
  advisory`. **NEGATIVE** = `signal_kind: correction` (requires `evidence_cid`) or `squelch` (capped `advisory`). Only
  *evidenced* drift carries `debit-soft`/`debit-firm`.

### 11.3 Frame-declaration is a provenance record, NOT a health vote

Resolution of the rule-poisoning capture vector (flood cheap plausible markers → manufacture a false over-fire ratio →
get a real guard retired): **the rule-drift / over-fire aggregate keys ONLY on evidenced `correction` and evidenced
`dismissal` — never on frame-declaration volume.** A `frame-declared` positive is a *provenance record* ("this occurrence
was consciously framed"), too cheap to carry suppression authority, and confirmable only when the async Layer-B
classifier *agrees* the declared frame matches the content — the marker is **necessary, never sufficient**. A single
marker act is classified as *either* a legitimate declaration *or* a dismissal **at review time**, from its
affirmed/rejected outcome — never logged as both at landing.

**Landings-tally hardening (agenda-setting capture).** Beyond declaration-volume, the raw *landings* tally is itself a
steering surface: an actor lands the pattern across many low-value throwaway docs, the tally crosses `_ESCALATE_AT`, and
the steward is repeatedly told *this* guard is noisy — agenda-setting toward retiring a healthy guard, which survives
§11.3, §12.3, and §11.4-as-"display." So the surfaced signal is **provenance-weighted and low-value-discounted**: a
landing from a low-standing / bulk / short-lived doc contributes ≈0. And the review prompt is **neutral** — "evaluate
corpus-drift *vs* rule-drift" — never pre-loaded toward "refine the rule."

### 11.4 Anti-gaming floors

Signals are **hook-observed, not agent-claimed** (`frame-declared` requires the marker present in landed content;
`dismissal`/`drift-landed` are derived from observed net-new + verdict state — closing the forge vector before coordinator
identity exists). Graduated floors: coordinator-derived `signer_pubkey`; **no-self-affirm** (signer ≠ rule author,
enforced T8-coordinator not HDI); evidence-required negatives; standing-weighted signals via `reach_earning::evaluate()`;
`FloorClass` protections never gameable by volume. **v1 reality, honest:** identity is self-reported, so **all v1 signals
are advisory-weight only**, and **no aggregate-triggered path may mutate a rule** — the dismissal counter is display-only;
supersession is steward-*originated*, never volume-triggered.

### 11.5 The binding ladder is the anti-gaming ceiling

A `persuasive` frame's signal volume never silently promotes it over a `constitutional` one; existential HARD-BLOCKs live
in `global.rs`, un-signalable. Crossing a binding-promotion threshold **pauses for explicit steward acknowledgment**
(the `acknowledges-reach-change` ceremony analog — friction is the feature). This prevents a brigading population from
voting a weak rule into constitutional force *and*, symmetrically, from voting a HARD-BLOCK down.

## 12. No-overwhelm surfacing + review→feedback (active learning)

### 12.1 Layered attenuation lattice (structural, not best-effort)

Ported from the self-healing control plane's headline invariant — *no single layer is load-bearing for the no-overwhelm
property*: (1) Layer A over-fires, the agent self-classifies at edit-time — most Family-1 never reaches the ledger;
(2) PostToolUse logs, never pages; (3) the semantic sweep batches + dedups by fingerprint (closure-by-deletion);
(4) a threshold gate — only accumulation past `_ESCALATE_AT` (per-rule, per-cost-class) crosses to review; (5) surfaced
once, into one venue (a SessionStart gate line), not per-event.

**Two channels, both rare-by-design:** the **threshold channel** (algedonic, from below) and the **System-3\* audit
channel** (from above — the sweep *samples quiet rules* and re-runs their detector against recent landings, because
quiet ⊇ {healthy, gamed-by-euphemism, broken}; a dog-whistle evading a rule that *should* fire is invisible to the
threshold channel by definition). **Two backpressure limits the source designs missed:** (a) **edit-time session dedup**
— once an author has answered/dismissed a frame's rubric in a session, that frame does not re-ask (mirror the async
fingerprint dedup onto the sync seam), so a 2,000-word hard-topic doc does not fire dozens of interrupts; (b) a **global
async escalation budget** across all frames (priority = `cost_class × standing`) plus **bulk-write detection** that
batch-defers dispatch, so a seed import / memory ceremony / doc migration does not spawn thousands of triage agents at
once (a concurrent-dispatch cap coalesced by rule — respecting the "top-tier fleets burn the limit" constraint).

### 12.2 The review→feedback loop (active learning, human-gated)

The `guard-recall-triage` verdict is one of four, each wiring to a *different* update surface:

| Verdict | Meaning | Updates | Mechanism |
|---|---|---|---|
| **`corpus-drift`** | rule healthy, corpus slipping | nothing in the rule | file a content-fix backlog item |
| **`euphemism-confirmed`** | Family-2 evasion real | **proposes** a Layer-A lexicon entry `status: proposed` | **human-ratified** before it arms the floor (§12.3) |
| **`false-positive`** | gate over-fired; use was legitimate | frame `why`/rubric exception; the abstention becomes a POSITIVE | **propose** rubric/`why` refinement (human-ratified, §12.3); raise the asymmetric FP cost for this class |
| **`rule-drift`** | the rule itself is wrong | **supersede the Precedent** (new version, `superseded_by`) | governance act, **never auto**, operator-provenance required |

Every escalation is a free active-learning sample — the exact hard-case population (near-boundary satire, half-recognized
dog-whistles) the model most needs. The human-review cost compounds into an asset: **the escalation *is* the training
sample.**

### 12.3 The floor-expansion gate (build-blocking safety)

May the async pass write back to the deterministic Layer-A trigger set? Auto-writeback is a floor-expansion-by-ungated-LLM
bug **plus** a prompt-injection vector (a poisoned audited doc steering the triage agent into arming the floor with
attacker-chosen phrases). **Resolution: `euphemism-confirmed` PROPOSES a lexicon entry in `status: proposed` that only
advises/abstains until a human ratifies it into an active trigger.** No agent verdict arms the mechanical floor
unratified. The async agent starts at **flag/escalate/propose-only** authority (the phased ceiling); deny/append
authority is *earned*, never day-one (§15 D6).

**The rubric and frame `why` text sit under the same gate.** They are not "just wording": the rubric is the exact prompt
every Family-1 self-classification answers, so a subtle shift ("are you quoting to analyze?" → "does this platform a
harmful view?") steers *all* future self-classifications toward drift **without touching the gated lexicon** — a
governor-injection surface distinct from the D6 frame-immutability question. So a `false-positive`-driven rubric/`why`
edit is a **proposed** change requiring the same human ratification as a trigger-set change.

### 12.4 Break the monotone ratchet (eviction / un-learning)

The lexicon must not only grow. Every proposed/ratified entry carries a **TTL + FP-rate auto-demote**: a phrase whose
synchronous firings resolve `false-positive` above rate R auto-demotes back to async-only. Frames can be superseded. This
is the *automatic loosening* half of the §2 asymmetry — the system may relax itself without a human (FP-demote, TTL) to
balance the human-ratified-only tightening — never clamp itself without one. That is the *only* automation on the floor,
and it runs solely in the fail-safe direction, countering the structural over-blocking bias.

### 12.5 Signal survival across rule versions + where it surfaces

Correcting a rule changes its `why`/rubric/threshold → new version → new CID. The aggregate keys on `id@version` (→ CID
at graduation) **and** the `superseded_by` lineage makes prior-version signals *visible-in-review* to the corrected
version. Convention to fix now: the drift-tally cache keys on `rule_cid` / `id@version`, **never on the bare `rule_id`
name**. Surfacing venue: a `guard:` gate line in the SessionStart MEMORY BUDGET headline, peer to `cleanup:`/`scope:`
(`held ✅` at stasis; `⚠ N drift / M recall-hits due` when queued). Review venue is a **`guard-stasis-loop`** (sibling of
`/deprecation-stasis`: measure → dispatch → re-measure → repeat). `rule-drift` on a `constitutional`/`binding-network`
Precedent routes up the floor→ceiling stack — a human-sortition concern, because the deepest rules cannot live in policy.

## 13. Progressive CID revelation — the story of why

Every `FrameClassification` carries `evidence.reason_ref` — the entry point into a walkable "story of why this guard
fired," HyperCard-deep, followable by any agent. No new UI is invented; the primitives exist (envelope tiering, coupling
CIDs, acyclic-by-construction, fuel-bounded chains).

- **Card face** = the ~500-byte frame-atom envelope (`id`, `family`, `polarity`, one-line `linguistic_definition`, and
  the "Connections" list = its `cites`) — small enough to gossip on the cheap dataplane; the DHT notarizes only
  load-bearing frames.
- **Reveal-on-tap = resolve one CID deeper.** The reader decides whether to follow each cite *from the `desc` field alone,
  without resolving* (progressive discovery). Depth ladder: rule fired → frame card face → frame `why`+`rubric` → cited
  prior-art (use-mention, satire-detection-limits, dog-whistle-under-flagging) → sibling frames (`frame-use-mention` ↔
  `frame-negation` ↔ `frame-satire`) → canon (`values-forward`, `elohim-ceiling-design` Principle 8, the
  `stewardship-over-sovereignty` mechanical-floor stance). Acyclic by construction; depth-bounded via visited-CID memo;
  chains spend fuel; same-CID collapses fan-out.

**Honesty on the v1 form.** At v1 `reason_ref` renders as the frame doc's **`path:` cite** (a plain `Read` follows it) —
progressive *path* revelation. The "Codex re-derives the same CID and lands on the same node" agnosticism guarantee is
**graduated-only** (RESERVED). The `frames/*.md` docs, being ordinary `genesis/docs` members, *are* sealable today with
`cite-gen.py --seal`; the frame *manifest bindings* graduate only when `.epr-meta` joins the `managed_surfaces.py`
registry (a real, named gap).

## 14. v1-buildable slice vs graduated form (honest)

| Concern | v1 (dev-tooling tier, buildable now) | Graduated (brit/eprfs/Mishpat) |
|---|---|---|
| **Enforcement seam** | PreToolUse convenience + **git pre-commit / CI gate, advisory-annotating only (`exit 0` always)** running `frame_classifier.py` | *blocking* CI status (shares terminal-`deny` reserved row) + pre-receive on notary path |
| **Engine wiring** | the §4 routing (unregistered→`ask`, co-resolution pre-pass, `defer`, algedonic side-channel, `FrameClassification` on the `Verdict` return) is an **engine-modification gap** — edits `_eval_rule`/`evaluate`/`combine`, NOT additive (`evaluate` is documented PURE) | same + graduated verdict fields |
| **Frame ontology** | `frames/*.md` under a governance dir, cite-sealed; `frame-ontology@1` + `frame-<name>@1` rows in `policies.yaml` | frame atoms = `Mishpat::Precedent` (CID = `entry_hash`); bindings become `cites` |
| **Layer A** | `_lib/frame_classifier.py` LF bank + normalization + **scan-size cap (degrade, never fail-open)**; sovereignty guard rebound as first consumer (parity-proven via sha-diff) | unchanged (data-driven) |
| **Layer B Family-1** | rubric surfaced as `ask`/`inject`; block-scoped in-content marker; **cooperative-authors-only — no adversarial resistance** | + adversarial-frame resistance (coordinator identity + armed evidenced Family-2) |
| **Layer B Family-2** | `apex-anchors.npz` + local MiniLM (cache/env owned or vendored-pinned) + cosine; sentinel on new fp; **abstain-only, shadow-log, testimony-exempt, never author-visible-as-accusation** | fine-tuned local head trained on accumulated escalation outcomes |
| **Verdict** | `FrameClassification`; coarse `confidence`, wide abstain band; `classifier_agent` self-reported | calibrated confidence; coordinator-derived `agent_info()` |
| **Signals** | JSONL in `.claude/data/classifier-signals.jsonl`, **advisory-weight only**, hook-observed | `FeedbackSignal`/`Attestation`, `target_cid` = Precedent CID, `PrecedentToFeedbackSignal` links |
| **Aggregation** | materialized `classifier-drift.json` keyed `id@version` | derived graph projection over the link fan-out |
| **Anti-gaming** | forge-blocked (hook-observed); **no coordinator identity, no standing weight, no aggregate-triggered mutation** | no-self-affirm, standing-weighted, evidence-required, distinct-signer floor |
| **Terminal `deny`** | **RESERVED, unwired** — nothing denies terminally; existential = `ask` + async-escalate (no `block-pending-steward` class exists) | wired only when sortition floor + appeal tier exist |
| **Floor writeback** | `euphemism-confirmed` = `status: proposed`, **human-ratified**; TTL + FP-demote | same, standing-gated ratification |
| **Story-of-why** | `reason_ref` = `path:` cite (progressive *path* revelation) | `reason_ref` = CID; envelope-tiered HyperCard walk |
| **Escalation endpoint** | tops out at **one operator, witnessed**; H2/H3 sortition invariant declared-but-inert; synthetic time-delayed-override-with-justification | H2 real cryptographic sortition; **H3 = the elohim** via `delegates-compute`-scoped, revocable, witnessed Commitment |

**Net v1 deliverable, honest:** recall-first **advisory** sensing + rubric surfacing (edit-time convenience + pre-commit
gate) + Family-2 shadow-log abstain-surfacing + drift-accumulation surfacing + a human-gated proposal/review loop. v1 does
**not** make rules self-correcting in the *loosening* direction, does **not** auto-mutate the floor, and does **not**
terminally deny. Everything aggregate-triggered or standing-weighted is graduated. "Projection swap, not a rewrite" is a
*design intent* about an *unbuilt* migration — asserted, not proven.

## 15. Open decisions for the human architect

- **D1 — Ratify the sensing-floor default (the hinge).** v1: no terminal `deny`, no auto floor-mutation, humans ratify all
  rule/lexicon/frame changes, existential concepts hard-escalate rather than block terminally. The alternative — ship a
  real `deny` now — buys an un-lobbyable blocking floor at the cost of deterministically censoring the highest-value
  speech on the highest-stakes topics with no appeal path, before the tiers it should appeal *to* exist. **Recommendation:
  ratify the sensing-floor default.**
- **D2 — Calibration cold-start + audit rate.** Ship abstain-heavy / shadow-only and narrow as outcomes accumulate
  (accepting early operator load); the System-3\* audit rate (`binding × staleness`) under-covers the *successfully-evaded*
  rule (low landings *because* the euphemism works). How much compute to spend auditing quiet rules?
- **D3 — Who may author a new apex-concept/frame, and how is *that* not a capture vector?** A hostile/careless frame
  ("criticism-of-governance is drift") weaponizes the classifier against the dissent `values-forward` protects. The
  recursion terminates at a human sortition body — *but the classifier ships before the ceiling exists.* What is the
  interim admissibility authority (single operator = capture risk; quorum = bootstrap-cost risk)?
- **D4 — Reclaimed-language / speaker-identity routing.** The same string is solidarity-speech or slur by speaker and a
  heterogeneous community's norms — context a byte-emission classifier structurally lacks. This can only route to the
  human sortition floor; *which* body scopes a given community's reclamation norms, and how that verdict binds back onto a
  frame atom without the floor over-reaching into governance it must not own, is unspecified. The sharpest floor↔ceiling
  boundary case.
- **D5 — PRE→POST correlation token acceptability.** Distinguishing `dismissal` (agent warned, proceeded) from
  benign-triggers-present needs a token written by the resolver, read by the signal hook. It is hook-written (not runtime
  Rust), so likely L4-safe — confirm before wiring the *loosening* direction of self-correction on it.
- **D6 — Ontology amendment vs graduated immutability.** The classifier governs the guards; what governs the ontology?
  Constitutional defeaters (negation, use-mention are not up for a vote) need graduated-immutability protection, while
  detector coverage (dog-whistle codebooks) is an open adversarial set that must stay amendable. The
  amendment-vs-immutability gradient *for the ontology itself* is the largest unbuilt piece.

## 16. Decomposition seed

This spec decomposes into gaps (in dependency order): **(G1)** `_lib/frame_classifier.py` — the Layer-A LF bank +
normalized-shadow + the `triggers()` interface, with the sovereignty guard rebound as first consumer (parity-proven);
**(G2)** the `frames/*.md` ontology + `frame-ontology@1`/`frame-<name>@1` policy rows, cite-sealed; **(G3)** the
**engine-modification gap** — extend `Verdict` to carry `FrameClassification` (NOT a `write` mutation — `evaluate` is PURE),
add the §4 co-resolution pre-pass + `defer` verdict + the unregistered→`ask` / module-load→`inject` split, and resolver
rubric-surfacing; **(G4)** the harness-neutral pre-commit / CI gate (advisory-annotating, `exit 0` always — carries less
verdict-expressivity than PreToolUse, so a *blocking* status is deferred with terminal-`deny`); **(G5)** `classifier-signal.py`
(generalized PostToolUse) + `classifier-drift.json` keyed `id@version` + git-seam net-new (added diff-hunk lines only) +
content-hash dedup ledger; **(G6)** `classifier-recall-harvest.py` external poller + `apex-anchors.npz` + local encoder
(provisioning owned) + `guard-recall-triage` agent (shadow-log, abstain-only, testimony-exempt); **(G7)** the `guard:`
SessionStart gate line + `guard-stasis-loop`; **(G8, graduated)** the `FeedbackSignal`-on-Precedent-CID substrate (a
DNA-lineage integrity-zome change, not a cheap widening) + coordinator-identity anti-gaming + CID story-of-why.
G1–G2 are the household-testable spine; G3 is the engine wiring; G4–G7 wire the loop; G8 is gated on the
epr-meta→brit/eprfs graduation.

*Resolved contradictions (for the refiner's audit): abstain routing = async-log by default, sync-`ask` reserved (§10.1);
auto-mutation authority = none at v1, human-ratified proposals only (§12.3); existential class = no rubric short-circuit,
no terminal deny, algedonic post-frame-resolution (§10.3); frame-declaration = provenance not health-vote (§11.3); signal
substrate = `FeedbackSignal` not ValueFlows `EconomicEvent` (§11.1); one canonical name per hook/ledger/verdict/poller/
triage-agent/stasis-loop (§6, §12); agent-agnosticism = harness-neutral git/CI seam + in-byte markers (§6, §9.2); Family-2
= abstain-only, never debit (§9.3); rule-version signal survival keyed on `id@version` + supersession lineage (§12.5);
monotone-ratchet broken by TTL + FP-demote — the only floor automation, fail-safe-loosening-only (§2, §12.4).*

*Red-team hardening (2026-07-15 verify pass): the classifier→engine wiring is an explicit modification gap, not additive —
`FrameClassification` threads the `Verdict` return, never a `write` mutation (`evaluate` is PURE) (§4, §16-G3); the git/CI
seam is advisory-annotating-only (`exit 0` always) — a blocking status shares the terminal-`deny` reserved row (§6, §14);
no `block-pending-steward` class exists — v1 existential resolves to `ask` + async-escalate (§10.3, §14); v1 Family-1
resolves cooperative authors only, adversarial resistance is graduated (§9.1); the frame marker is block-scoped,
per-content-type, unvalidated→`inject` (§9.2); Family-2 is testimony-exempt and never author-visible-as-accusation (§9.3);
the landings tally is provenance-weighted with a neutral review prompt (§11.3); rubric/`why` text sits under the
trigger-set ratification gate (§12.3); `vouch_kind` additions are a DNA-lineage integrity-zome change, not a cheap
widening (§11.2); the Layer-A scan is size-capped (degrade, never fail-open) (§8); the unregistered-validator downgrade is
advisory, not silent (§6, §10.1).*
