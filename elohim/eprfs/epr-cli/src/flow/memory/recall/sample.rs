//! `sample` and `judge` — governed-discovery station 3, task 3.2.
//!
//! `sample` composes the EXISTING journey operations (`open` focused, `read` the first located
//! candidate, `finish`) in-process over one locked session, and folds the whole journey into a
//! single `elohim_epr_rea::FlowEvent { action: Consume, … }` on the flows sidecar — the same store
//! every other REA record in this repository lands in. `judge` is the SECOND seat: a reviewer who
//! did not run the journey rules on how many of the reader's assertions were mistaken, recorded as
//! an `elohim_epr::verdict::Verdict` riding inside a `run:verdict` note on the plan.
//!
//! Both verbs write their folds through [`note::observe`] — the SAME registry-validated writer
//! `epr flow note --kind observation --measure …` uses — never a hand-rolled sidecar line. Nothing
//! here mints a record kind the fabric already has one for: a journey is a `FlowEvent`, a ruling is
//! a `Verdict` carried by a `note --kind verdict`, and a measurement is an `Observation`.
//!
//! **Why `judge` never takes the session lock [`Execution`] holds.** It rules on an
//! ALREADY-RECORDED event; it neither opens nor advances a ceremony continuation, so `mod.rs`
//! dispatches it before a session is ever opened (see the module doc there). `sample`, in
//! contrast, genuinely drives the ceremony state machine — three internal [`execute`] calls over
//! one session — so it is dispatched exactly where a single `execute()` call would be, and the
//! caller's usual accounting/encoding/exit-code plumbing applies to its aggregate view unchanged.
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

/// The plan a `judge` verdict lands on — the same document station 3 of this plan is itself a
/// task of, so the ruling and the work it rules on share one home.
const PLAN_REL: &str =
    "genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md";

/// The axis every `judge` verdict answers on.
const JUDGE_AXIS: &str = "recall-journey";

/// The policy this verdict is read against — named so a reader can trace the ruling to its
/// governing habit (`.epr-meta/recall-reaches-authority.habit.md`) without parsing prose.
const JUDGE_POLICY_REF: &str = "recall-reaches-authority";

/// `classified_as` slot keys `judge` reconstructs its fold's `--env` from. Closed on purpose: a
/// fifth slot on a future `FlowEvent` this code does not know about must never be silently folded
/// in as if it were one of these four.
const ENV_SLOT_KEYS: [&str; 4] = ["reader", "question", "recipe", "lens"];

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
        // Additive, positional, `key:value` — the shape `judge` reconstructs its own fold's `--env`
        // from when handed nothing but this event's CID (see `ENV_SLOT_KEYS`).
        classified_as: vec![
            format!("reader:{reader_ref}"),
            format!("question:{question_id}"),
            format!("recipe:{method}"),
            format!("lens:{}", resolved_lens.cid),
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
    // A focused `open` that locates its candidate on the first screen is one screen to shape —
    // the same reading `lens.rs`'s revealed-tier rule and the measure's own procedure line give.
    let screens_fold = note::observe(
        &args.root,
        "observation",
        "recall-screens-to-shape@1",
        note::REPO_SUBJECT,
        1.0,
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
            "path": question.reached_when.path,
            "assertion": question.reached_when.assertion,
        },
        "read": {"path": read_path, "lines": read_lines},
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

/// `epr flow memory recall judge --event <cid> --as <seat> --mistaken <n> --reason <text>`.
///
/// Dispatched from `mod.rs::run()` BEFORE a session is opened (see the module doc). Refuses
/// outright — bypassing the usual `--json` envelope, printing the literal refusal line whichever
/// way it was invoked — when `seat` names the sampled journey's own reader: a second seat is the
/// whole point of a judge, and a reader grading its own journey is not a second seat.
pub(super) fn judge(args: &Args, contract: &Contract) -> FlowResult<ExitCode> {
    let event_raw = args
        .event
        .clone()
        .ok_or_else(|| refused("judge needs --event <cid>"))?;
    let seat = args
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

    // A reader never judges its own journey. Printed directly — not through the standard
    // `--json`-aware refusal envelope — because this is the one judgment this verb can reach
    // before it has decided anything else, and it must read the same whichever way it was asked.
    if seat == event.provider.0 {
        println!("refused: a reader never judges its own journey");
        return Ok(ExitCode::from(2));
    }

    let mut env: BTreeMap<String, String> = BTreeMap::new();
    for slot in &event.classified_as {
        if let Some((key, value)) = slot.split_once(':') {
            if ENV_SLOT_KEYS.contains(&key) {
                env.insert(key.to_string(), value.to_string());
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

    let seat_actor = note::NoteActor {
        as_ref: Some(seat.clone()),
        session: Some(args.session.clone()),
    };
    let plan_note = note::note(
        &args.root,
        PLAN_REL,
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

    let reader = lens::reader_from_session(&args.root, &args.session);
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
            "contract_path": rel_to_root(&args.root, &contract.path),
            "executor": "epr flow memory recall",
        },
        "usage": {},
        "actions": [],
        "unresolved": [],
    });
    view["lens"] = resolved_lens.to_value();

    let render_floor = lens::RenderFloor::declared();
    let raw = super::encode(&view, &resolved_lens, &render_floor, args.json)?;
    print!("{raw}");
    Ok(ExitCode::SUCCESS)
}
