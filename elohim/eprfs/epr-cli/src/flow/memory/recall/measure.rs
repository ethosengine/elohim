//! The paired deterministic footprint lens and the `measure` op that pins it.
use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The paired deterministic footprint lens
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Take one sample: NATIVELY unless the caller named an external lens script.
///
/// The lens is a measurement primitive the ceremony depends on, so it is part of the executor
/// rather than a script the executor has to go and find. `--footprint-lens` stays available for an
/// external lens — a different implementation someone wants to measure with — and is the only thing that
/// reaches the subprocess path.
fn sample_balance(
    root: &Path,
    contract: &Contract,
    lens: Option<&Path>,
    run_id: &str,
    phase: &str,
    scope: &[Value],
) -> FlowResult<(Value, Vec<String>)> {
    let declared = contract.value.pointer("/ceremony/measurements/limits");
    let limits = footprint::Limits::from_declared(declared);
    let Some(lens) = lens else {
        return Ok((
            footprint::snapshot(root, run_id, phase, scope, limits)?,
            Vec::new(),
        ));
    };
    external_sample(root, contract, lens, run_id, phase, scope, &limits)
}

/// Run an EXPLICIT external lens script, bounded like any other foreign provider.
#[allow(clippy::too_many_arguments)]
fn external_sample(
    root: &Path,
    contract: &Contract,
    lens: &Path,
    run_id: &str,
    phase: &str,
    scope: &[Value],
    limits: &footprint::Limits,
) -> FlowResult<(Value, Vec<String>)> {
    let mut args = vec![
        lens.to_string_lossy().to_string(),
        "--root".into(),
        root.to_string_lossy().to_string(),
        "--json".into(),
        "--no-save".into(),
        "--run-id".into(),
        run_id.to_string(),
        "--phase".into(),
        phase.to_string(),
    ];
    for entry in scope {
        args.push("--scope".into());
        args.push(format!(
            "{}:{}",
            entry["category"].as_str().unwrap_or("authored"),
            entry["path"].as_str().unwrap_or_default()
        ));
    }
    for (flag, value) in [
        ("--max-files", limits.max_files),
        ("--max-bytes", limits.max_bytes),
        ("--max-entries", limits.max_entries),
    ] {
        args.push(flag.into());
        args.push(value.to_string());
    }
    let unresolved = vec![format!(
        "sampled with the EXTERNAL lens `{}`; its method pin, budgets and omissions are its own, not this executor's",
        rel_to_root(root, lens)
    )];
    let outcome = bounded_process(
        "python3",
        &args,
        contract.limit_usize("native_raw_bytes"),
        contract.limit_secs("native_timeout_seconds"),
    )
    .map_err(|error| refused(format!("footprint lens unavailable: {error}")))?;
    if let Some(reason) = outcome.error {
        return Err(refused(format!("footprint lens {reason}")));
    }
    if outcome.status != Some(0) {
        return Err(refused(format!(
            "footprint lens failed: {}",
            String::from_utf8_lossy(&outcome.stderr)
                .chars()
                .take(400)
                .collect::<String>()
        )));
    }
    let sample: Value = serde_json::from_slice(&outcome.stdout)
        .map_err(|error| refused(format!("footprint lens output unreadable: {error}")))?;
    Ok((sample, unresolved))
}

/// Compare a pinned baseline against a pinned close. One implementation, in the lens.
fn compare_samples(baseline: &Value, close: &Value) -> Value {
    footprint::compare(baseline, close)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// measure — the paired deterministic lens, receipted and pinned
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Take (or re-read) one phase of the paired footprint measurement.
///
/// Both halves of the pair are pinned receipts, and an unpaired `close` REFUSES to compare rather
/// than reconstructing a baseline after the fact — a baseline sampled after the work is not a
/// baseline, and a delta computed from one is a number with no meaning.
pub(super) fn measure(
    args: &Args,
    contract: &Contract,
    state: &mut Value,
    phase: &str,
    method: &str,
) -> FlowResult<Value> {
    let policy = contract
        .value
        .pointer("/ceremony/measurements")
        .cloned()
        .unwrap_or(Value::Null);
    if state["measurements"].is_null() {
        state["measurements"] = json!({});
    }
    let mut usage = json!({});
    let mut unresolved: Vec<String> = Vec::new();

    if !args.measure_scope.is_empty() && !state["measurements"]["scope"].is_null() {
        let requested = parse_measure_scope(&args.measure_scope)?;
        if json!(requested) != state["measurements"]["scope"] {
            return Err(refused("measurement scope changed; begin a new run"));
        }
    }
    let session_limit = contract
        .value
        .pointer("/limits/session_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let baseline_reference = state["measurements"]["baseline"].clone();
    let baseline = if phase == "close" && !baseline_reference.is_null() {
        Some(load_receipt(
            &args.root,
            &baseline_reference,
            session_limit,
        )?)
    } else {
        None
    };

    let sample = if !state["measurements"][phase].is_null() {
        let reference = state["measurements"][phase].clone();
        add_usage(
            &mut usage,
            &json!({"measurement_receipt_bytes": reference["bytes"]}),
        );
        load_receipt(&args.root, &reference, session_limit)?
    } else {
        if phase == "close" && baseline_reference.is_null() {
            return Ok(json!({
                "comparable": false,
                "reasons": ["No contemporaneous baseline; cannot reconstruct one."],
            }));
        }
        if state["measurements"]["scope"].is_null() {
            let scope = if !args.measure_scope.is_empty() {
                json!(parse_measure_scope(&args.measure_scope)?)
            } else if state["scope"].as_str() != Some(".") {
                json!([{"path": state["scope"], "category": "authored"}])
            } else {
                policy["default_scope"].clone()
            };
            state["measurements"]["scope"] = scope;
        }
        if state["measurements"]["run_id"].is_null() {
            state["measurements"]["run_id"] = json!(args.session);
        }
        let run_id = state["measurements"]["run_id"]
            .as_str()
            .unwrap_or(&args.session)
            .to_string();
        let scope = state["measurements"]["scope"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        // No path search and no refusal: the lens is native. `--footprint-lens` names an EXTERNAL one.
        let (sample, lens_notes) = sample_balance(
            &args.root,
            contract,
            args.footprint_lens.as_deref(),
            &run_id,
            phase,
            &scope,
        )?;
        unresolved.extend(lens_notes);
        add_usage(
            &mut usage,
            &json!({
                "measurement_source_bytes": sample["overhead"]["source_bytes_read"],
                "measurement_entries": sample["overhead"]["entries_examined"],
            }),
        );
        let mut reference = save_receipt(&args.root, &args.session, phase, method, &sample)?;
        // Existing native resource identity; numerical truth remains in the pinned sample.
        let pin = super::super::execute(&args.root, "pin", reference["path"].as_str(), None);
        match pin {
            Ok(pin) => {
                add_usage(&mut usage, &prefixed(&pin["usage"], "measurement_native_"));
                reference["resource"] = pin["resource"].clone();
            }
            Err(error) => {
                reference["native_pin_unavailable"] = json!([error.to_string()]);
            }
        }
        if let Some(actor) = &args.actor_session {
            let reason = format!(
                "Explicit {phase} memory burden observation for {run_id}; quantities and sampling window are in the referenced evidence, not this unit note."
            );
            match note::note_for_session(
                &args.root,
                reference["path"].as_str().unwrap_or_default(),
                "observation",
                &reason,
                actor,
                None,
            ) {
                Ok(outcome) => reference["observation"] = json!(outcome.record_cid),
                Err(error) => reference["observation_unresolved"] = json!([error.to_string()]),
            }
        }
        state["measurements"][phase] = reference;
        state["measurements"]["costs"][phase] = sample["overhead"].clone();
        sample
    };

    let reference = state["measurements"][phase].clone();
    let omissions = sample["omissions"].as_array().cloned().unwrap_or_default();
    let mut result = json!({
        "usage": usage,
        "phase": phase,
        "evidence": reference,
        "observed": sample["observed"],
        "partitions": sample["totals"],
        "complete": sample["complete"],
        "omissions": omissions.iter().take(8).cloned().collect::<Vec<_>>(),
        "omission_count": omissions.len(),
        "sampling_finished": sample["sampling_finished"],
        "meaning": "Fixed sampled cohort, not a ceremony grade or proof of token savings.",
        "post_sampling_artifact_bytes": reference["bytes"],
        "post_sampling_artifacts_unmeasured": ["continuation write", "native observation append", "rendered output", "later reports"],
        "limitations": sample["limitations"],
    });
    if !unresolved.is_empty() {
        result["unresolved"] = json!(unresolved);
    }
    if phase == "close" {
        let baseline_bytes = baseline_reference["bytes"].as_i64().unwrap_or(0);
        let previous = result["usage"]["measurement_receipt_bytes"]
            .as_i64()
            .unwrap_or(0);
        result["usage"]["measurement_receipt_bytes"] = json!(previous + baseline_bytes);
        result["baseline"] = baseline_reference;
        let mut comparison = match &baseline {
            Some(baseline) => compare_samples(baseline, &sample),
            None => {
                json!({"comparable": false, "reasons": ["No contemporaneous baseline; cannot reconstruct one."]})
            }
        };
        for key in ["added_paths", "removed_paths", "same_content_relocations"] {
            if let Some(rows) = comparison.get(key).and_then(Value::as_array).cloned() {
                comparison[format!("{key}_count")] = json!(rows.len());
                comparison[key] = json!(rows.iter().take(4).cloned().collect::<Vec<_>>());
            }
        }
        comparison["detail"] = json!("Up to four path examples per category; full detail derives from the exact paired snapshot files.");
        result["comparison"] = comparison;
        result["later_work"] =
            json!("This pair is closed; later work needs a new run for a new comparison.");
    }
    Ok(result)
}

fn parse_measure_scope(items: &[String]) -> FlowResult<Vec<Value>> {
    items
        .iter()
        .map(|item| {
            let (category, path) = item
                .split_once(':')
                .ok_or_else(|| refused("--measure-scope requires category:relative/path"))?;
            Ok(json!({"category": category, "path": path}))
        })
        .collect()
}

fn prefixed(usage: &Value, prefix: &str) -> Value {
    let mut out = Map::new();
    if let Some(map) = usage.as_object() {
        for (key, value) in map {
            if numeric(value).is_some() {
                out.insert(format!("{prefix}{key}"), value.clone());
            }
        }
    }
    Value::Object(out)
}
