//! `sample` and `judge` — governed-discovery station 3, task 3.2.
//!
//! `sample` composes the EXISTING journey operations (`open` focused, `read` the first located
//! candidate, `finish`) in-process over one locked session, and folds the whole journey into a
//! single `elohim_epr_rea::FlowEvent { action: Consume, … }` on the flows sidecar — the same store
//! every other REA record in this repository lands in. `judge` is the SECOND seat: a reviewer who
//! did not run the journey rules on how many of the reader's assertions were mistaken, recorded as
//! an `elohim_epr::verdict::Verdict` riding inside a `run:verdict` note on the recall contract —
//! the governed method whose CID every receipt in this file already pins, never a dated plan path.
//!
//! Both verbs write their folds through [`note::observe`] — the SAME registry-validated writer
//! `epr flow note --kind observation --measure …` uses — never a hand-rolled sidecar line. Nothing
//! here mints a record kind the fabric already has one for: a journey is a `FlowEvent`, a ruling is
//! a `Verdict` carried by a `note --kind verdict`, and a measurement is an `Observation`.
//!
//! **Why `judge` never takes the session lock [`Execution`] holds.** It rules on an
//! ALREADY-RECORDED event; it neither opens nor advances a ceremony continuation, so `mod.rs`
//! dispatches it BEFORE the `--session` requirement itself (the seat claims nothing — see the
//! dispatch site's own comment). `sample`, in contrast, genuinely drives the ceremony state
//! machine — three internal [`execute`] calls over one session — so it is dispatched exactly
//! where a single `execute()` call would be, and the caller's usual accounting/encoding/exit-code
//! plumbing applies to its aggregate view unchanged.
use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr::verdict::{CheckOutcome, CheckWitness, Decision, Verdict, Witness};
use elohim_epr_rea::{
    atom_cid, parse_agent_ref, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    SidecarFlowStore,
};

use super::*;

/// The synthetic receiver every sampled journey's `FlowEvent` names — the recall executor's own
/// steward seat, not the reader who ran the journey.
const SAMPLE_RECEIVER: &str = "agent:steward@repo";

/// The axis every `judge` verdict answers on.
const JUDGE_AXIS: &str = "recall-journey";

/// The policy this verdict is read against — named so a reader can trace the ruling to its
/// governing habit (`.epr-meta/recall-reaches-authority.habit.md`) without parsing prose.
const JUDGE_POLICY_REF: &str = "recall-reaches-authority";

/// `env:k=v` slot keys `judge` reconstructs its fold's `--env` from — exact parity with
/// `note::observe`'s own `ENV_SLOT_PREFIX`/`k=v` writer (fix round 1, D1), so a fold minted by
/// `sample` and one reconstructed by `judge` from the event alone are indistinguishable.  Closed
/// on purpose: a fifth slot on a future `FlowEvent` this code does not know about must never be
/// silently folded in as if it were one of these four.
const ENV_SLOT_KEYS: [&str; 4] = ["reader", "question", "recipe", "lens"];

/// The journey operations `sample` always runs internally (`open`, `read`, `finish`) — the base
/// the "not reached" screens-to-shape fold adds one to (fix round 1, Q3's ruling).
const JOURNEY_OPERATIONS: f64 = 3.0;

/// `epr flow memory recall sample --question <id> --reader agent:<role>@<model> [--lens L]`.
///
/// Dispatched from `mod.rs::run()` in place of a single [`execute`] call, over the SAME locked
/// [`Execution`] — see the module doc's "why `judge` never takes the session lock" note for the
/// contrast. Three internal journey calls, each synthesizing its own [`Args`] from the caller's:
/// `open` (focused, `scope` = the bank question's own scope, `need` = its intent's question text),
/// `read` (the first located candidate's path and located line range), `finish`. The read excerpt
/// is checked against `reached_when.terms`; the journey is folded as one `FlowEvent` regardless of
/// whether it reached authority — only `fulfills` differs.
pub(super) fn sample(
    args: &Args,
    contract: &Contract,
    execution: &mut Execution,
    method: &str,
) -> FlowResult<(Value, lens::LensView)> {
    let question_id = args
        .question
        .clone()
        .ok_or_else(|| refused("sample needs --question <id>"))?;
    let reader_ref = args
        .reader
        .clone()
        .ok_or_else(|| refused("sample needs --reader agent:<role>@<model>"))?;
    // One shape, checked once: the fabric's own parser, so the CLI cannot accept an identity the
    // store would later refuse.
    parse_agent_ref(&reader_ref)?;

    let bank = contract.question_bank()?;
    let question = bank
        .iter()
        .find(|candidate| candidate.id == question_id)
        .cloned()
        .ok_or_else(|| {
            let known: Vec<&str> = bank.iter().map(|q| q.id.as_str()).collect();
            refused(format!(
                "unknown --question `{question_id}`; the bank declares: {}",
                known.join(", ")
            ))
        })?;

    // Register the reader's actor claim for this session — the same write `epr actor claim`
    // performs — so the lens below resolves the reader's own stated/revealed tier from
    // `--reader`, not from an unclaimed session.
    crate::actor::claim(&args.root, &reader_ref, &args.session)
        .map_err(|error| refused(error.to_string()))?;

    let need = question
        .intent
        .resource_spec
        .classified_as
        .first()
        .cloned()
        .unwrap_or_else(|| question.id.clone());

    // 1. open — focused on the question's own scope and question text.
    let mut open_args = args.clone();
    open_args.operation = "open".into();
    open_args.scope = Some(question.scope.clone());
    open_args.intent = None;
    open_args.purpose = None;
    open_args.need = need.clone();
    open_args.need_explicit = true;
    let (open_view, _) = execute(&open_args, contract, execution, method)?;

    let candidates: Vec<Value> = open_view["first_screen"]["candidates"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let located = candidates
        .first()
        .filter(|candidate| candidate["best_section"]["lines"].is_string())
        .ok_or_else(|| {
            refused(
                "sample found no located candidate for this question; open the session manually \
                 and choose a source",
            )
        })?;
    let read_path = located["path"].as_str().unwrap_or_default().to_string();
    let read_lines = located["best_section"]["lines"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // 2. read — the first located candidate's own range.
    let mut read_args = args.clone();
    read_args.operation = "read".into();
    read_args.path = Some(read_path.clone());
    read_args.lines = Some(read_lines.clone());
    read_args.need = need.clone();
    let (read_view, _) = execute(&read_args, contract, execution, method)?;

    let excerpt = read_view["evidence"]["sources"][0]["content"]
        .as_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let reached = !question.reached_when.terms.is_empty()
        && question
            .reached_when
            .terms
            .iter()
            .all(|term| excerpt.contains(&term.to_ascii_lowercase()));
    let metered_bytes = read_view["usage"]["source_bytes"].as_u64().unwrap_or(0);

    // 3. finish — a focused journey closes on its inspected passage, not a selected concern.
    let mut finish_args = args.clone();
    finish_args.operation = "finish".into();
    finish_args.outcome = Some(format!(
        "sample journey for question `{question_id}`; reached={reached}"
    ));
    finish_args.question = Some(format!(
        "sample --question {question_id} — journey closed, nothing left open"
    ));
    let (finish_view, resolved_lens) = execute(&finish_args, contract, execution, method)?;

    let mut usage = json!({});
    add_usage(&mut usage, &open_view["usage"]);
    add_usage(&mut usage, &read_view["usage"]);
    add_usage(&mut usage, &finish_view["usage"]);

    // Identity: the bank question's own Intent, addressed the same way every fabric atom is.
    let intent_cid = atom_cid(&question.intent)?;
    let recipe_cid: Cid = method
        .parse()
        .map_err(|_| refused("recall contract method cid failed to parse"))?;
    let (_author, occurred_at) = head_commit_provenance(&args.root).ok_or_else(|| {
        refused(
            "cannot date a sample: git has no HEAD commit to author it against — a journey is \
             dated by the tree it was run against, never by wall clock",
        )
    })?;

    let event = FlowEvent {
        action: ReaVerb::Consume,
        provider: AgentRef(reader_ref.clone()),
        receiver: AgentRef(SAMPLE_RECEIVER.to_string()),
        resource: intent_cid,
        quantity: Magnitude::Count {
            value: metered_bytes as f64,
            unit: "bytes".to_string(),
        },
        process: Some(recipe_cid),
        in_scope_of: recipe_cid,
        fulfills: if reached {
            vec![intent_cid]
        } else {
            Vec::new()
        },
        satisfies: Vec::new(),
        // Fix round 1, D1: `env:k=v`, exact parity with `note::observe`'s own env-slot writer —
        // the shape `judge` reconstructs its fold's `--env` from when handed nothing but this
        // event's cid.
        classified_as: vec![
            format!("env:reader={reader_ref}"),
            format!("env:question={question_id}"),
            format!("env:recipe={method}"),
            format!("env:lens={}", resolved_lens.cid),
        ],
        occurred_at,
    };

    let mut store = SidecarFlowStore::open(&args.root)?.transaction()?;
    let existing = store.records()?;
    let record = FlowRecord::Event(event.clone());
    let event_cid = record.cid()?;
    // Idempotent by construction, matching the fabric's own dedup discipline (`flow/mod.rs`'s
    // module doc): a repeated sample of the same journey on the same HEAD mints the same bytes,
    // so it is a no-op append rather than a second row.
    if !existing.iter().any(|(cid, _)| *cid == event_cid) {
        store.append(record)?;
    }
    drop(store);

    let mut env: BTreeMap<String, String> = BTreeMap::new();
    env.insert("reader".into(), reader_ref.clone());
    env.insert("question".into(), question_id.clone());
    env.insert("recipe".into(), method.to_string());
    env.insert("lens".into(), resolved_lens.cid.clone());

    let measures_path = measures::default_measures(&args.root);
    let fold_actor = note::NoteActor {
        as_ref: Some(reader_ref.clone()),
        session: Some(args.session.clone()),
    };
    let metered_fold = note::observe(
        &args.root,
        "observation",
        "recall-metered-bytes@1",
        note::REPO_SUBJECT,
        metered_bytes as f64,
        Some("bytes"),
        &env,
        None,
        &fold_actor,
        &measures_path,
    )?;
    // Fix round 1, Q3 (controller ruling): reached → the shape was on the first screen the first
    // located candidate carried, so it folds 1. Not reached → the shape was never on a screen this
    // journey showed, so it folds the journey's rendered-screen count (open, read, finish = 3)
    // plus one (4) — an honest "more than every screen it rendered" rather than the flat 1 every
    // journey folded before this fix, reached or not.
    let screens_value = if reached {
        1.0
    } else {
        JOURNEY_OPERATIONS + 1.0
    };
    let screens_fold = note::observe(
        &args.root,
        "observation",
        "recall-screens-to-shape@1",
        note::REPO_SUBJECT,
        screens_value,
        Some("count"),
        &env,
        None,
        &fold_actor,
        &measures_path,
    )?;
    let unmetered_fold = note::observe(
        &args.root,
        "observation",
        "recall-unmetered-bytes@1",
        note::REPO_SUBJECT,
        0.0,
        Some("bytes"),
        &env,
        None,
        &fold_actor,
        &measures_path,
    )?;

    // Fix round 1, C1: name the located-vs-declared divergence when the ranking's first located
    // candidate is not the question's own `reached_when.path` — `reached:false` on such a journey
    // is the ranking telling the truth, not a defect, and the view should say so in one line
    // rather than leave a reader to diff two buried fields themselves.
    let declared_path = question.reached_when.path.display().to_string();
    let location = if read_path == declared_path {
        format!("located {read_path} (matches declared)")
    } else {
        format!("located {read_path} · declared {declared_path}")
    };

    let mut view = json!({
        "operation": "sample",
        "orientation": {
            "intent": need,
            "scope": question.scope,
            "worthwhile_finish": "one journey through the recall ceremony, addressed as a FlowEvent",
            "guiding_context": [],
            "constraints": [
                "A sample folds bytes and reach; it is not itself an acceptance.",
                "A second seat's judge is what turns a sample into a graded journey.",
            ],
            "recipe_version": contract.value["version"],
            "provider": Value::Null,
        },
        // Fix round 1, Q2: the honesty floor reads `outcome.receipts` (receipt count) and
        // `first_screen.ranking`/`.omissions` (selection rule, omissions) directly off the view —
        // without these, the floor line under-reports a journey that ranked, omitted and stands
        // on a real receipt as "no candidates ranked … omissions: 0 · receipts: 0".
        "outcome": finish_view["outcome"].clone(),
        "first_screen": open_view["first_screen"].clone(),
        "event": {
            "cid": event_cid.to_string(),
            "action": "consume",
            "provider": event.provider.0,
            "receiver": event.receiver.0,
            "resource": event.resource.to_string(),
            "quantity": {"value": metered_bytes, "unit": "bytes"},
            "process": event.process.map(|cid| cid.to_string()),
            "inScopeOf": event.in_scope_of.to_string(),
            "fulfills": event.fulfills.iter().map(Cid::to_string).collect::<Vec<_>>(),
            "occurredAt": event.occurred_at,
        },
        "reached": reached,
        "question": {
            "id": question_id,
            "path": declared_path,
            "assertion": question.reached_when.assertion,
        },
        "read": {"path": read_path, "lines": read_lines},
        "location": location,
        "folds": [
            metered_fold.record_cid,
            screens_fold.record_cid,
            unmetered_fold.record_cid,
        ],
        "usage": usage,
        "actions": [],
        "unresolved": [],
    });
    view["lens"] = resolved_lens.to_value();

    Ok((view, resolved_lens))
}

/// Everything one `judge` invocation resolves to, before rendering.
enum JudgeOutcome {
    /// The seat named the sampled journey's own reader — refused, never rendered as a ruling.
    SelfJudge,
    Rendered(Value, Box<lens::LensView>),
}

/// `epr flow memory recall judge --event <cid> --as <seat> --mistaken <n> --reason <text>`.
///
/// Dispatched from `mod.rs::run()` BEFORE a session is opened (see the module doc) — `--session`
/// is not even required (fix round 1, S1). The outer shell owns rendering: [`judge_inner`] does
/// the work and returns either [`JudgeOutcome::SelfJudge`] or a rendered view; every ordinary
/// `Err` this leg can produce (a missing flag, an unknown `--event`) is caught HERE and routed
/// through the same `--json`-aware [`print_refusal`] envelope every other operation uses — fixing
/// a latent gap where those errors would otherwise propagate past this whole executor's rendering
/// contract and surface as a bare `main.rs` stderr line instead.
pub(super) fn judge(args: &Args, contract: &Contract) -> FlowResult<ExitCode> {
    match judge_inner(args, contract) {
        Ok(JudgeOutcome::SelfJudge) => judge_self_refusal(&args.session, args.json),
        Ok(JudgeOutcome::Rendered(view, resolved_lens)) => {
            let render_floor = lens::RenderFloor::declared();
            let raw = super::encode(&view, &resolved_lens, &render_floor, args.json)?;
            print!("{raw}");
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => print_refusal(&error.to_string(), &args.session, None, args.json),
    }
}

/// Fix round 1, D2: the self-judge refusal keeps the standard refusal ENVELOPE (so `--json`
/// still parses, and `next`/`session` are still present) while carrying the brief's exact
/// wording — `"refused: a reader never judges its own journey"` — literally inside
/// `unresolved[0]` in `--json`, so the substring holds under both renderings without the ordinary
/// `refusal_lines` human template doubling the `"refused: "` prefix on the human rendering.
fn judge_self_refusal(session: &str, json_output: bool) -> FlowResult<ExitCode> {
    const MESSAGE: &str = "a reader never judges its own journey";
    let remedy = remedy_for(MESSAGE, session);
    if !json_output {
        print!("{}", refusal_lines(MESSAGE, &remedy));
        return Ok(ExitCode::from(2));
    }
    let failure = json!({
        "unresolved": [format!("refused: {MESSAGE}")],
        "session": session,
        "next": remedy,
        "accounting": "Session unavailable; refusal outside session accounting.",
    });
    println!("{}", serde_json::to_string(&failure)?);
    Ok(ExitCode::from(2))
}

/// The whole of `judge`'s work: resolve arguments, load the sampled event, decide self-judge vs.
/// a genuine second seat, and — for a genuine seat — write the verdict note and its fold and
/// render the view. Every early exit here is an ordinary `FlowResult::Err`, caught by the caller.
fn judge_inner(args: &Args, contract: &Contract) -> FlowResult<JudgeOutcome> {
    let event_raw = args
        .event
        .clone()
        .ok_or_else(|| refused("judge needs --event <cid>"))?;
    let seat_raw = args
        .seat
        .clone()
        .ok_or_else(|| refused("judge needs --as <seat>"))?;
    let mistaken = args
        .mistaken
        .ok_or_else(|| refused("judge needs --mistaken <n>"))?;
    let reason = args
        .reason
        .clone()
        .ok_or_else(|| refused("judge needs --reason <text>"))?;

    // Fix round 1, Q1: parsed and trimmed exactly as `sample` validates `--reader` — a leading/
    // trailing space or a malformed shape must not slip an untrimmed string past the self-judge
    // comparison below.
    let seat = seat_raw.trim().to_string();
    parse_agent_ref(&seat)?;

    let event_cid: Cid = event_raw
        .trim()
        .parse()
        .map_err(|_| refused("--event is not a CID"))?;
    let store = SidecarFlowStore::open(&args.root)?;
    let event = store
        .records()?
        .into_iter()
        .find_map(|(cid, record)| (cid == event_cid).then_some(record))
        .and_then(|record| match record {
            FlowRecord::Event(event) => Some(event),
            _ => None,
        })
        .ok_or_else(|| refused("--event does not name a recorded FlowEvent"))?;

    if seat == event.provider.0 {
        return Ok(JudgeOutcome::SelfJudge);
    }

    // Fix round 1, D1: read the `env:k=v` slots back the same way `note::observe` writes them.
    let mut env: BTreeMap<String, String> = BTreeMap::new();
    for slot in &event.classified_as {
        if let Some(pair) = slot.strip_prefix("env:") {
            if let Some((key, value)) = pair.split_once('=') {
                if ENV_SLOT_KEYS.contains(&key) {
                    env.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    let decision_word = if mistaken == 0 { "permit" } else { "refuse" };
    let verdict_word = if mistaken == 0 {
        "approved"
    } else {
        "changes-requested"
    };
    let check = CheckWitness {
        check_id: "mistaken-assertions".into(),
        outcome: if mistaken == 0 {
            CheckOutcome::Passed
        } else {
            CheckOutcome::Failed
        },
        summary: reason.clone(),
        observed: Some(json!(mistaken)),
    };
    let verdict = Verdict {
        axis: JUDGE_AXIS.into(),
        subject: Some(event_cid.to_string()),
        decision: if mistaken == 0 {
            Decision::Permit
        } else {
            Decision::Refuse
        },
        witness: Witness {
            checks: vec![check],
        },
        policy_ref: Some(JUDGE_POLICY_REF.into()),
    };
    let verdict_json = serde_json::to_string(&verdict)?;

    // Fix round 1, Q5 (controller ruling): the verdict's subject/fold target is the governed
    // recall contract already loaded — the method whose cid every receipt in this file pins —
    // never a dated plan path that would need re-pointing every time the plan moves or archives.
    let contract_rel = rel_to_root(&args.root, &contract.path);

    let seat_actor = note::NoteActor {
        as_ref: Some(seat.clone()),
        session: Some(args.session.clone()),
    };
    let plan_note = note::note(
        &args.root,
        &contract_rel,
        "verdict",
        &verdict_json,
        None,
        Some(verdict_word),
        &seat_actor,
    )?;

    let measures_path = measures::default_measures(&args.root);
    let fold = note::observe(
        &args.root,
        "observation",
        "recall-mistaken-assertions@1",
        note::REPO_SUBJECT,
        mistaken as f64,
        Some("count"),
        &env,
        Some(&format!(
            "second-seat judge of {event_cid}: {mistaken} mistaken assertion(s)"
        )),
        &seat_actor,
        &measures_path,
    )?;

    // Fix round 1, Q4: the lens is the SEAT's own reading, resolved directly from `--as` — never
    // from the session claim (`judge` registers no actor claim of its own; the seat claims
    // nothing).
    let reader = lens::ReaderRef::Agent(AgentRef(seat.clone()));
    let resolved_lens = lens::resolve(&reader, contract, args.lens, &args.root);
    let mut view = json!({
        "operation": "judge",
        "orientation": {
            "intent": format!("judge {event_cid}"),
            "worthwhile_finish": "a second seat's ruling on one sampled journey",
            "guiding_context": [],
            "constraints": [
                "A verdict never discharges the sampled journey — it rules on it.",
            ],
            "recipe_version": contract.value["version"],
            "provider": Value::Null,
        },
        "verdict": {
            "axis": JUDGE_AXIS,
            "subject": event_cid.to_string(),
            "decision": decision_word,
            "witness": {
                "checks": [{
                    "checkId": "mistaken-assertions",
                    "outcome": if mistaken == 0 { "passed" } else { "failed" },
                    "summary": reason,
                    "observed": mistaken,
                }],
            },
            "policyRef": JUDGE_POLICY_REF,
            "note": plan_note.record_cid,
        },
        "fold": fold.record_cid,
        "execution_method": {
            "method": contract.method_cid(),
            "recall-contract.json": contract.sha256(),
            "contract_path": contract_rel,
            "executor": "epr flow memory recall",
        },
        "usage": {},
        "actions": [],
        "unresolved": [],
    });
    view["lens"] = resolved_lens.to_value();

    Ok(JudgeOutcome::Rendered(view, Box::new(resolved_lens)))
}
