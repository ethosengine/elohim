//! Governed-discovery station 4, task 4.2: the embedder is a pinned, bounded fold procedure.
//!
//! Two `Embedder`s answer one question — "what vectors do these texts have?" — under a budget the
//! caller resolves from the pinned contract (`EmbedBudget::query` for one question at query time,
//! `EmbedBudget::fold` for a fold batch off the query path). `PinnedProcedure` runs the declared
//! `embedder/embed.py` through the bounded subprocess envelope and refuses, as
//! `unavailable: <reason>`, whenever the method it would run is not the one the model manifest
//! pins: the procedure's own bytes, the model's bytes, the interpreter, the modules. `Fixture` is
//! the test-interchange embedder, declared like the `fixture` provider: never live fitness.
mod common;

use std::path::{Path, PathBuf};

use elohim_epr_cli::flow::memory::recall::embedder::{
    Embedder, Fixture, ModelManifest, PinnedProcedure, FIXTURE_FITNESS, MODEL_MANIFEST_REL,
    PROCEDURE_REL,
};
use elohim_epr_cli::flow::memory::recall::{self, Contract};
use elohim_epr_cli::flow::FlowError;
use eprfs_core::BlobCid;

fn contract() -> Contract {
    Contract::load(&common::repo_root().join(recall::CONTRACT_REL)).expect("live contract loads")
}

fn texts(items: &[&str]) -> Vec<String> {
    items.iter().map(|t| t.to_string()).collect()
}

fn norm(vector: &[f32]) -> f32 {
    vector.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() / (norm(a) * norm(b))
}

/// The live manifest with its `resolve` list pointed at `dir` only — so a test names exactly the
/// model directory the procedure is asked to verify, whatever this machine's cache holds.
fn manifest_resolving_to(dir: &Path) -> ModelManifest {
    let mut manifest = ModelManifest::load(&common::repo_root().join(MODEL_MANIFEST_REL))
        .expect("the live model manifest loads");
    manifest.resolve = vec![dir.to_string_lossy().into_owned()];
    manifest
}

fn procedure_path() -> PathBuf {
    common::repo_root().join(PROCEDURE_REL)
}

fn unavailable_reason(error: FlowError) -> String {
    match error {
        FlowError::Unavailable(reason) => reason,
        other => panic!("expected `unavailable: <reason>`, got `{other}`"),
    }
}

/// The fixture embedder is a pure function of its text: the same text twice is the same vector,
/// every vector is 384 unit-length floats, different texts differ, and the reply says it is
/// test interchange rather than a claim of fitness.
#[test]
fn the_fixture_embedder_is_deterministic_and_normalised() {
    let budget = recall::embedder::fold_budget(&contract()).expect("fold budget declared");
    let batch = texts(&[
        "which command rebuilds the stale index",
        "which command rebuilds the stale index",
        "a different question about stewardship",
        "",
    ]);
    let first = Fixture.embed(&batch, budget).expect("fixture embeds");
    let again = Fixture.embed(&batch, budget).expect("fixture embeds");
    assert_eq!(first.dims, 384);
    assert_eq!(first.vectors.len(), batch.len());
    assert_eq!(first.vectors, again.vectors, "deterministic across calls");
    assert_eq!(
        first.vectors[0], first.vectors[1],
        "deterministic within a batch"
    );
    assert_ne!(first.vectors[0], first.vectors[2]);
    for vector in &first.vectors {
        assert_eq!(vector.len(), 384);
        assert!(
            (norm(vector) - 1.0).abs() < 1e-5,
            "unit length: {}",
            norm(vector)
        );
    }
    assert_eq!(first.fitness, FIXTURE_FITNESS);
    assert!(FIXTURE_FITNESS.contains("no live provider fitness"));
}

/// Both budgets are read from the pinned contract, never hard-coded: the query budget is the
/// provider envelope for one question, the fold budget is the declared fold procedure envelope.
#[test]
fn both_embedding_budgets_are_declared_by_the_contract() {
    let contract = contract();
    let limits = &contract.value["limits"];
    let query = recall::embedder::query_budget(&contract).expect("query budget declared");
    assert_eq!(Some(query.bytes as u64), limits["provider_bytes"].as_u64());
    assert_eq!(Some(query.seconds), limits["provider_seconds"].as_f64());
    assert_eq!(query.texts, 1, "the query budget embeds one question");
    let fold = recall::embedder::fold_budget(&contract).expect("fold budget declared");
    assert_eq!(
        Some(fold.bytes as u64),
        limits["fold_procedure_bytes"].as_u64()
    );
    assert_eq!(
        Some(fold.seconds),
        limits["fold_procedure_seconds"].as_f64()
    );
    assert_eq!(Some(fold.texts as u64), limits["fold_batch_texts"].as_u64());

    let mut undeclared = contract.value.clone();
    undeclared["limits"]
        .as_object_mut()
        .expect("limits")
        .remove("fold_batch_texts");
    let undeclared = Contract::from_value(undeclared).expect("still a valid contract");
    let refused =
        recall::embedder::fold_budget(&undeclared).expect_err("an undeclared budget is refused");
    assert!(
        refused
            .to_string()
            .contains("invalid positive budget: fold_batch_texts"),
        "{refused}"
    );

    // The same reader `Contract::validate` uses: a count budget must be an integer, a seconds
    // budget may be fractional.
    let mut fractional = contract.value.clone();
    fractional["limits"]["fold_batch_texts"] = serde_json::json!(1.5);
    let fractional = Contract::from_value(fractional).expect("still a valid contract");
    let refused =
        recall::embedder::fold_budget(&fractional).expect_err("a fractional count is refused");
    assert!(
        refused
            .to_string()
            .contains("byte/count budgets must be integers: fold_batch_texts"),
        "{refused}"
    );
    let mut seconds = contract.value.clone();
    seconds["limits"]["fold_procedure_seconds"] = serde_json::json!(0.5);
    let seconds = Contract::from_value(seconds).expect("still a valid contract");
    assert_eq!(
        recall::embedder::fold_budget(&seconds).unwrap().seconds,
        0.5
    );
}

/// A batch larger than the budget allows is the caller's error, refused before anything runs.
#[test]
fn a_batch_over_the_budget_is_refused_before_anything_runs() {
    let query = recall::embedder::query_budget(&contract()).expect("query budget");
    let error = FlowError::from(
        Fixture
            .embed(&texts(&["one", "two"]), query)
            .expect_err("two texts exceed a one-question budget"),
    );
    assert!(matches!(error, FlowError::InvalidArguments(_)), "{error}");
}

/// The declared procedure is part of the method: the manifest pins `embed.py`'s bytes, and a
/// `PinnedProcedure` whose pin names other bytes refuses rather than running them.
#[test]
fn procedure_bytes_off_the_pin_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = manifest_resolving_to(dir.path());
    manifest.procedure = Some(BlobCid::compute_raw(b"some other procedure").to_string());
    let embedder = PinnedProcedure {
        manifest,
        procedure: procedure_path(),
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let error = FlowError::from(embedder.embed(&texts(&["x"]), budget).expect_err("refused"));
    assert_eq!(
        unavailable_reason(error),
        "procedure bytes do not match the pin"
    );
}

/// A model directory whose bytes are not the pinned model is refused by the procedure itself
/// (exit 3), surfaced as the pin refusal — never a panic, never vectors from the wrong model.
#[test]
fn a_model_dir_with_wrong_bytes_reports_the_pin_refusal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("model.onnx"), b"not the pinned model").unwrap();
    std::fs::write(dir.path().join("tokenizer.json"), b"{}").unwrap();
    let embedder = PinnedProcedure {
        manifest: manifest_resolving_to(dir.path()),
        procedure: procedure_path(),
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let error = FlowError::from(embedder.embed(&texts(&["x"]), budget).expect_err("refused"));
    let rendered = error.to_string();
    assert!(
        rendered.starts_with("unavailable: model bytes do not match the pin ("),
        "{rendered}"
    );
    // The procedure's own reason rides along: which file, what it hashes to, what was pinned.
    let reason = unavailable_reason(error);
    assert!(reason.contains("model.onnx hashes to bafkrei"), "{reason}");
    assert!(reason.contains("the pin is bafkrei"), "{reason}");
}

/// A resolved directory with no `model.onnx` is not silently the same as wrong bytes: the reason
/// says the file could not be read.
#[test]
fn a_model_dir_missing_its_model_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let embedder = PinnedProcedure {
        manifest: manifest_resolving_to(dir.path()),
        procedure: procedure_path(),
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let reason = unavailable_reason(FlowError::from(
        embedder.embed(&texts(&["x"]), budget).unwrap_err(),
    ));
    assert!(
        reason.starts_with("model bytes do not match the pin (model.onnx unreadable"),
        "{reason}"
    );
}

/// An unexpected failure reports its informative last line, not `Traceback (most recent call
/// last):`.
#[test]
fn an_unexpected_procedure_failure_reports_its_last_stderr_line() {
    let dir = tempfile::tempdir().unwrap();
    let stub = dir.path().join("stub.py");
    let body = b"import sys\nsys.stdin.read()\nraise RuntimeError('the session could not load')\n";
    std::fs::write(&stub, body).unwrap();
    let mut manifest = manifest_resolving_to(dir.path());
    manifest.procedure = Some(BlobCid::compute_raw(body).to_string());
    let embedder = PinnedProcedure {
        manifest,
        procedure: stub,
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let reason = unavailable_reason(FlowError::from(
        embedder.embed(&texts(&["x"]), budget).unwrap_err(),
    ));
    assert!(
        reason.ends_with("RuntimeError: the session could not load"),
        "{reason}"
    );
    assert!(!reason.contains("Traceback"), "{reason}");
}

/// No directory in the `resolve` list exists: the route is unavailable and says where it looked.
#[test]
fn no_resolvable_model_directory_reports_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let embedder = PinnedProcedure {
        manifest: manifest_resolving_to(&dir.path().join("absent")),
        procedure: procedure_path(),
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let reason = unavailable_reason(FlowError::from(
        embedder.embed(&texts(&["x"]), budget).unwrap_err(),
    ));
    assert!(reason.contains("no model directory resolves"), "{reason}");
}

/// A missing interpreter (the shape `EPR_EMBED_PYTHON=/nonexistent` produces) is checked at call
/// time and reported as unavailable, not inferred and not a panic.
#[test]
fn a_missing_interpreter_reports_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let embedder = PinnedProcedure {
        manifest: manifest_resolving_to(dir.path()),
        procedure: procedure_path(),
        interpreter: "/nonexistent".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let reason = unavailable_reason(FlowError::from(
        embedder.embed(&texts(&["x"]), budget).unwrap_err(),
    ));
    assert!(reason.contains("/nonexistent"), "{reason}");
}

/// A module the procedure cannot import is reported by the procedure (exit 2 with
/// `unavailable: <module>` on stderr) and surfaces as the same `unavailable: <reason>`.
#[test]
fn a_missing_python_module_reports_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let stub = dir.path().join("stub.py");
    let body = b"import sys\nsys.stdin.read()\nsys.stderr.write(\"unavailable: python module 'onnxruntime' is not importable\\n\")\nsys.exit(2)\n";
    std::fs::write(&stub, body).unwrap();
    let mut manifest = manifest_resolving_to(dir.path());
    manifest.procedure = Some(BlobCid::compute_raw(body).to_string());
    let embedder = PinnedProcedure {
        manifest,
        procedure: stub,
        interpreter: "python3".to_string(),
    };
    let budget = recall::embedder::query_budget(&contract()).unwrap();
    let error = FlowError::from(embedder.embed(&texts(&["x"]), budget).unwrap_err());
    assert_eq!(
        error.to_string(),
        "unavailable: python module 'onnxruntime' is not importable"
    );
}

/// The first of the procedure's three runtime modules (in `embed.py`'s own import order) that
/// `interpreter` cannot import, or `Err` when the interpreter itself does not run. Probed at call
/// time, never inferred from what a package list declares.
fn missing_runtime_module(interpreter: &str) -> Result<Option<&'static str>, String> {
    for module in ["numpy", "onnxruntime", "tokenizers"] {
        let probe = std::process::Command::new(interpreter)
            .args(["-c", &format!("import {module}")])
            .output()
            .map_err(|error| format!("{interpreter}: {error}"))?;
        if !probe.status.success() {
            return Ok(Some(module));
        }
    }
    Ok(None)
}

/// The live model, wherever the manifest's `resolve` list finds it: a two-text batch returns two
/// 384-vectors, each unit length, cosine(self) ≈ 1 and cosine(a, b) < 1. The precondition for the
/// live leg is BOTH halves of the runtime — a model directory that resolves AND an interpreter
/// that imports `numpy`, `onnxruntime` and `tokenizers` (probed first, never inferred). Otherwise
/// this test does not skip and never panics: it asserts the specific honest `unavailable: <reason>`
/// for what is absent — the model directory, the interpreter, or the first module missing.
///
/// Follow-up (station 4 final review, ruling I6): the native route should own its model directory
/// through a provisioning recipe; the palace's cache (`~/.cache/chroma/...`) that the manifest's
/// `resolve` list falls back to today is only today's fallback, not the route's home.
#[test]
fn the_live_model_embeds_a_two_text_batch_or_says_why_not() {
    let embedder = PinnedProcedure::declared(&common::repo_root()).expect("manifest loads");
    let budget = recall::embedder::fold_budget(&contract()).unwrap();
    let batch = texts(&[
        "which command rebuilds the stale index",
        "a household stewards its shared garden",
    ]);
    let answer = embedder.embed(&batch, budget).map_err(FlowError::from);
    let Some(dir) = embedder.manifest.resolve_model_dir() else {
        let reason = unavailable_reason(answer.expect_err("no model resolves"));
        assert!(reason.contains("no model directory resolves"), "{reason}");
        println!("live model absent on this machine: unavailable: {reason}");
        return;
    };
    match missing_runtime_module(&embedder.interpreter) {
        Err(why) => {
            let reason = unavailable_reason(answer.expect_err("the interpreter does not run"));
            assert!(reason.contains(&embedder.interpreter), "{reason} ({why})");
            println!("live runtime absent on this machine: unavailable: {reason}");
        }
        Ok(Some(module)) => {
            let reason = unavailable_reason(answer.expect_err("a runtime module is missing"));
            assert_eq!(
                reason,
                format!("python module '{module}' is not importable"),
                "the model resolves at {} but the runtime lacks {module}",
                dir.display()
            );
            println!("live runtime absent on this machine: unavailable: {reason}");
        }
        Ok(None) => {
            let embedding = answer.unwrap_or_else(|e| {
                panic!(
                    "the model resolves at {} and the runtime imports, but it did not embed: {e}",
                    dir.display()
                )
            });
            assert_eq!(embedding.dims, 384);
            assert_eq!(embedding.vectors.len(), 2);
            let (a, b) = (&embedding.vectors[0], &embedding.vectors[1]);
            assert_eq!(a.len(), 384);
            assert_eq!(b.len(), 384);
            assert!((norm(a) - 1.0).abs() < 1e-3, "normalised: {}", norm(a));
            assert!((cosine(a, a) - 1.0).abs() < 1e-3);
            assert!(cosine(a, b) < 0.99, "distinct texts: {}", cosine(a, b));
            assert!(embedding.fitness.contains("recall-bank-reach@1"));
            // Neither text is anywhere near the 256-token window: a procedure that counts says 0.
            assert!(
                embedding.truncated.is_none_or(|n| n == 0),
                "{:?}",
                embedding.truncated
            );
            // One text past the window (~600 tokens): a procedure that counts says exactly 1; one
            // that does not count says nothing (unknown), never a guessed 0.
            let long = "stewardship of the commons ".repeat(150);
            let counted = embedder
                .embed(&texts(&[&long, "a short text"]), budget)
                .expect("a long text embeds from its head");
            assert!(
                counted.truncated.is_none_or(|n| n == 1),
                "{:?}",
                counted.truncated
            );
            println!(
                "live procedure truncation count: {}",
                counted.truncated.map_or(
                    "unknown (the pinned procedure does not count)".to_string(),
                    |n| n.to_string()
                )
            );
            // One question fits the query-time provider envelope (8192 bytes, 15 s): the reply
            // is ~4 KB of 6-significant-digit floats.
            let question = embedder
                .embed(
                    &batch[..1],
                    recall::embedder::query_budget(&contract()).unwrap(),
                )
                .expect("one question embeds within the query budget");
            assert_eq!(question.vectors.len(), 1);
            assert!(
                cosine(&question.vectors[0], a) > 0.999,
                "same text, same vector"
            );
            println!(
                "live model at {}: cosine(a, b) = {:.4}",
                dir.display(),
                cosine(a, b)
            );
        }
    }
}

/// Station 4 final review, ruling I5: truncation is COUNTED. A procedure that reports `truncated`
/// (texts whose token count exceeded the model's window) carries it through `Embedding`; one that
/// does not says nothing, which reads as unknown (`None`) — never a guessed zero. The fixture
/// embedder never truncates.
#[test]
fn a_reply_carries_the_procedures_truncation_count_or_none() {
    let budget = recall::embedder::fold_budget(&contract()).unwrap();
    assert_eq!(
        Fixture
            .embed(&texts(&["a", "b"]), budget)
            .unwrap()
            .truncated,
        Some(0)
    );
    let dir = tempfile::tempdir().unwrap();
    for (reply, expected) in [
        ("{\"dims\": 384, \"vectors\": v, \"truncated\": 1}", Some(1)),
        ("{\"dims\": 384, \"vectors\": v}", None),
    ] {
        let stub = dir.path().join("stub.py");
        let body = format!(
            "import json, sys\nn = len(json.loads(sys.stdin.read())[\"texts\"])\n\
             v = [[1.0] + [0.0] * 383 for _ in range(n)]\nprint(json.dumps({reply}))\n"
        );
        std::fs::write(&stub, &body).unwrap();
        let mut manifest = manifest_resolving_to(dir.path());
        manifest.procedure = Some(BlobCid::compute_raw(body.as_bytes()).to_string());
        let embedder = PinnedProcedure {
            manifest,
            procedure: stub,
            interpreter: "python3".to_string(),
        };
        let embedding = embedder
            .embed(&texts(&["short", "long"]), budget)
            .expect("the stub answers");
        assert_eq!(embedding.truncated, expected, "{reply}");
    }
}
