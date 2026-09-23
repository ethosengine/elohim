//! Governed-discovery station 4, task 4.1: the embedding model and the semantic index are
//! declared as governed artifacts before anything runs them.
//!
//! Two Manifest EPRs land beside the recall contract. `embedding-models/all-minilm-l6-v2.json`
//! pins the model's bytes and tokenizer by CID; `recall-semantic-index.json` is an
//! `elohim_epr_rea::IndexMeasure` (`recall-semantic-index@1`) whose `ModelPin` names those same
//! bytes. The contract (v15) declares the `semantic` provider over that measure and demotes the
//! palace to a declared visitor. No runtime behaviour moves here — these tests read the live
//! declarations and hold them to the protocol types' own refusals.
mod common;

use std::path::Path;

use elohim_epr::kind::EprKind;
use elohim_epr::measure::Period;
use elohim_epr::reach::Reach;
use elohim_epr_cli::flow::memory::recall::{self, Contract};
use elohim_epr_rea::{
    atom_cid, IndexError, IndexMeasure, LimitSource, PinnedRef, RankingMethod, ReachBound,
    Retention, Sense, VectorMetric, PRIVATE_CHAIN_KINDS,
};
use serde_json::Value;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";
const MODEL_REL: &str = ".epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json";

/// The v14 contract's method CID — the recipe every receipt pinned before this station. The
/// contract's bytes changed, so its address must have moved off this one.
const V14_METHOD_CID: &str = "bafkreieryn2ru3uz72jnif5dy3ichzpxhh22k4dtilhiwymuxgl5vyzcoa";

/// The v15 contract's method CID (task 4.1). Task 4.2 declared the fold procedure's budgets, so the
/// contract's bytes — and its address — moved off this one.
const V15_METHOD_CID: &str = "bafkreiauhzxlr2ex6vd6lwu66la2f6dzkp6pj2ogfdjfxaemuanbfiii5q";

/// `IndexMeasure::cid()` of the live `recall-semantic-index@1` declaration. Task 4.1 fix round 1
/// proved the string spelling of its CIDs left this address where the byte-array spelling had it
/// (`bafyreic5…ycma` both ways); the pin then moved once, deliberately, for the self-consistent
/// chunk rule and the canonical (absent) fold-lag sense/source.
const PINNED_MEASURE_CID: &str = "bafyreie4eowvvs7w4q54pwxfu5j4u7a6a7atmqajpoqtgfbnfyxx27m6oq";

fn read_json(root: &Path, rel: &str) -> Value {
    let raw = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel} reads: {e}"));
    serde_json::from_slice(&raw).unwrap_or_else(|e| panic!("{rel} parses: {e}"))
}

fn live_measure(root: &Path) -> IndexMeasure {
    serde_json::from_value(read_json(root, MEASURE_REL))
        .expect("the declaration deserializes into elohim_epr_rea::IndexMeasure")
}

/// The declared measure is a well-formed `IndexMeasure`: it deserializes through the protocol
/// type's own serde, re-validates against the same refusals the constructors apply, and carries
/// exactly the declaration the plan names — a cosine ranking under a pinned model, a
/// `SelfScope` ceiling, the contract's source roots as its surfaces, and a declared fold-lag
/// ceiling of 25 files.
#[test]
fn the_semantic_index_declaration_is_a_valid_index_measure() {
    let root = common::repo_root();
    let measure = live_measure(&root);
    measure
        .validate()
        .expect("a vector ranking with its model pin validates");

    assert_eq!(
        measure.measure,
        PinnedRef {
            id: "recall-semantic-index".into(),
            version: 1
        }
    );
    assert_eq!(
        measure.ranking,
        RankingMethod::Vector {
            metric: VectorMetric::Cosine
        }
    );
    assert_eq!(measure.reach, ReachBound::ceiling(Reach::SelfScope));
    assert!(measure.admits(Reach::SelfScope) && !measure.admits(Reach::Intimate));

    assert_eq!(measure.fold_lag.limit, 25.0);
    assert_eq!(measure.fold_lag.unit, "files");
    assert_eq!(measure.fold_lag.sense(), Sense::Ceiling);
    assert_eq!(measure.fold_lag.source(), LimitSource::Declared);
    // One meaning, one encoding: a ceiling and a declared source are spelled by absence, so the
    // bound passes `Bound::validate()`'s refusal of the redundant explicit spellings.
    measure
        .fold_lag
        .validate()
        .expect("the fold-lag bound is canonical");

    // Demotion at the first fold that observes a removal, never `Keep`, never a delete.
    assert_eq!(
        measure.retention,
        Retention::DemoteAfter {
            count: 0,
            per: Period::Second
        }
    );

    // The surfaces are the contract's source roots minus anything its discovery excludes, and no
    // kind the private chain holds.
    let contract = common::live_contract();
    let excluded: Vec<&str> = contract["discovery"]["exclude_directories"]
        .as_array()
        .expect("exclude_directories")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let expected: Vec<String> = contract["source_roots"]
        .as_array()
        .expect("source_roots")
        .iter()
        .filter_map(Value::as_str)
        .filter(|root| {
            !Path::new(root)
                .components()
                .any(|c| excluded.contains(&c.as_os_str().to_string_lossy().as_ref()))
        })
        .map(str::to_string)
        .collect();
    assert_eq!(measure.surfaces.paths(), expected.as_slice());
    assert!(!measure.surfaces.kinds().is_empty());
    assert!(measure
        .surfaces
        .kinds()
        .iter()
        .all(|k| !PRIVATE_CHAIN_KINDS.contains(k)));
    assert!(measure.surfaces.kinds().contains(&EprKind::Content));

    // A roundtrip does not re-address the declaration: the method CID a candidate will print is
    // a property of the declared value, not of how the file happens to be formatted.
    let again: IndexMeasure =
        serde_json::from_value(serde_json::to_value(&measure).unwrap()).unwrap();
    assert_eq!(again.cid().unwrap(), measure.cid().unwrap());
}

/// The method CID every semantic candidate will print, pinned. It is `IndexMeasure::cid()` over
/// canonical dag-cbor, so how the JSON spells a CID (string, not bytes) never moves it — only a
/// changed declaration does, and that is a deliberate re-pin here.
#[test]
fn the_measure_method_cid_is_pinned() {
    let measure = live_measure(&common::repo_root());
    assert_eq!(measure.cid().unwrap().to_string(), PINNED_MEASURE_CID);
}

/// The chunk rule is content-addressed: the declaration carries the rule object it names, and
/// `chunkRule` is that object's canonical dag-cbor CID — minted the way `IndexMeasure::cid()`
/// mints (`elohim_epr_rea::atom_cid`), never by a second implementation.
#[test]
fn the_chunk_rule_cid_addresses_the_declared_rule() {
    let root = common::repo_root();
    let declared = read_json(&root, MEASURE_REL);
    let rule = &declared["_chunk_rule"];
    assert!(rule.is_object(), "the measure declares its chunk rule");
    assert_eq!(rule["max_chunk_bytes"], 2000);
    assert_eq!(rule["max_chunks_per_file"], 12);
    // Self-consistent: a non-sectioned file's window is the chunk cap itself, a measure of its
    // own (the contract's lexical passage window is a different one), and the addressed object
    // carries values only — prose inside it would move the method CID on a rewording.
    assert_eq!(rule["other"]["window_bytes"], rule["max_chunk_bytes"]);
    fn has_no_prose(value: &Value) -> bool {
        match value {
            Value::Object(map) => map
                .iter()
                .all(|(key, v)| key != "declared_by" && has_no_prose(v)),
            Value::Array(items) => items.iter().all(has_no_prose),
            Value::String(text) => !text.contains(' '),
            _ => true,
        }
    }
    assert!(has_no_prose(rule), "the chunk rule holds values, not prose");
    let measure = live_measure(&root);
    assert_eq!(measure.chunk_rule, atom_cid(rule).unwrap());
    // One encoding per meaning: the declaration spells its CIDs as strings, with no mirror.
    assert_eq!(
        declared["chunkRule"].as_str(),
        Some(measure.chunk_rule.to_string().as_str())
    );
    assert!(declared.get("_cids").is_none());
}

/// The `ModelPin` resolves to the model manifest: one set of bytes, one license, one width. The
/// manifest is a Manifest EPR carrying both byte pins as raw CIDs and the fitness measure that
/// judges it.
#[test]
fn the_model_pin_resolves_to_the_model_manifest() {
    let root = common::repo_root();
    let manifest = read_json(&root, MODEL_REL);
    assert_eq!(manifest["artifact_type"], "model-manifest");
    assert_eq!(manifest["license"], "Apache-2.0");
    assert_eq!(manifest["dims"], 384);
    assert_eq!(manifest["pooling"], "mean");
    assert_eq!(manifest["normalize"], true);
    assert_eq!(manifest["max_tokens"], 256);
    assert_eq!(manifest["source"], "sentence-transformers/all-MiniLM-L6-v2");
    assert_eq!(manifest["fitness"], "recall-bank-reach@1");
    let resolve = manifest["resolve"].as_array().expect("resolve list");
    assert!(resolve.iter().any(|r| r == "$EPR_EMBED_MODEL_DIR"));

    let model_bytes: cid::Cid = manifest["model_bytes"]
        .as_str()
        .expect("model_bytes")
        .parse()
        .expect("model_bytes is a CID");
    let tokenizer_bytes: cid::Cid = manifest["tokenizer_bytes"]
        .as_str()
        .expect("tokenizer_bytes")
        .parse()
        .expect("tokenizer_bytes is a CID");
    // Raw bytes are addressed with the raw codec (0x55), as `eprfs_core::BlobCid::compute_raw`
    // mints them — the codec tag tells the truth about what was hashed.
    assert_eq!(model_bytes.codec(), 0x55);
    assert_eq!(tokenizer_bytes.codec(), 0x55);
    assert_ne!(model_bytes, tokenizer_bytes);

    let measure = live_measure(&root);
    let pin = measure
        .embedding
        .expect("the semantic measure pins a model");
    assert_eq!(pin.model_bytes, model_bytes);
    assert_eq!(Some(pin.license.as_str()), manifest["license"].as_str());
    assert_eq!(Some(u64::from(pin.dims)), manifest["dims"].as_u64());
    let declared = read_json(&root, MEASURE_REL);
    assert_eq!(declared["_model_manifest"], MODEL_REL);
    assert_eq!(
        declared["embedding"]["modelBytes"].as_str(),
        manifest["model_bytes"].as_str(),
        "the measure and the manifest spell the same model bytes identically"
    );
}

/// A copy of the declaration with its pin removed is refused by the protocol type itself: a
/// vector ranking without a model is malformed, never a silent default embedding.
#[test]
fn a_vector_ranking_without_its_model_pin_is_refused() {
    let root = common::repo_root();
    let mut declared = read_json(&root, MEASURE_REL);
    declared["embedding"] = Value::Null;
    let unpinned: IndexMeasure = serde_json::from_value(declared).expect("still deserializes");
    assert!(unpinned.embedding.is_none());
    assert_eq!(unpinned.validate(), Err(IndexError::ModelPinMissing));
}

/// Contract v15 names the native semantic provider, keeps the palace declared as a visitor, and
/// its method CID moved — with the question bank re-pinned to the new recipe.
#[test]
fn contract_v15_declares_the_native_semantic_provider_and_repins_the_bank() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(recall::CONTRACT_REL)).expect("live contract loads");
    let value = common::live_contract();
    // v15 introduced these declarations; later versions keep them (each byte change bumps).
    assert!(value["version"].as_u64() >= Some(15));

    let semantic_provider = value["discovery"]["semantic_provider"]
        .as_str()
        .expect("semantic_provider");
    assert_ne!(semantic_provider, "mempalace");
    assert_eq!(semantic_provider, "semantic");

    let semantic = &value["ceremony"]["providers"]["semantic"];
    assert_eq!(semantic["kind"], "semantic");
    assert_eq!(semantic["measure"], MEASURE_REL);
    assert_eq!(semantic["optional"], true);
    assert!(root.join(MEASURE_REL).is_file());

    let palace = &value["ceremony"]["providers"]["mempalace"];
    assert_eq!(palace["kind"], "mempalace");
    assert_eq!(palace["role"], "visitor");

    let method = contract.method_cid();
    assert_ne!(method, V14_METHOD_CID, "the contract's bytes moved");
    let bank = read_json(&root, value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v15 recipe");
}

/// Task 4.2: the model manifest pins the embedding procedure's own bytes (the procedure is part
/// of the method), and resolves the model directory explicit-first: an operator's
/// `$EPR_EMBED_MODEL_DIR` before the default cache path.
#[test]
fn the_model_manifest_pins_the_embedding_procedure() {
    use elohim_epr_cli::flow::memory::recall::embedder::PROCEDURE_REL;
    let root = common::repo_root();
    let manifest = read_json(&root, MODEL_REL);
    let procedure = std::fs::read(root.join(PROCEDURE_REL)).expect("embed.py reads");
    let pinned: cid::Cid = manifest["procedure"]
        .as_str()
        .expect("the manifest names its procedure")
        .parse()
        .expect("procedure is a CID");
    assert_eq!(
        pinned.codec(),
        0x55,
        "raw bytes are addressed with the raw codec"
    );
    assert_eq!(
        manifest["procedure"].as_str(),
        Some(
            eprfs_core::BlobCid::compute_raw(&procedure)
                .to_string()
                .as_str()
        ),
        "embed.py on disk is the pinned procedure"
    );
    assert_eq!(
        manifest["resolve"],
        serde_json::json!([
            "$EPR_EMBED_MODEL_DIR",
            "~/.cache/chroma/onnx_models/all-MiniLM-L6-v2/onnx"
        ])
    );
}

/// Contract v16 declares the fold procedure's own budget beside the query-time provider budget
/// (a 32-text batch of 384-float vectors cannot fit the 8192-byte query envelope), and the bank
/// is re-pinned to the new recipe.
#[test]
fn contract_v16_declares_the_fold_budget_and_repins_the_bank() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(recall::CONTRACT_REL)).expect("live contract loads");
    let value = common::live_contract();
    assert_eq!(value["version"], 16);
    let limits = &value["limits"];
    assert_eq!(
        limits["provider_bytes"], 8192,
        "the query budget is unchanged"
    );
    assert_eq!(limits["provider_seconds"], 15);
    assert_eq!(limits["fold_procedure_bytes"], 262144);
    assert_eq!(limits["fold_procedure_seconds"], 120);
    assert_eq!(limits["fold_batch_texts"], 32);

    let method = contract.method_cid();
    assert_ne!(method, V15_METHOD_CID, "the contract's bytes moved");
    let bank = read_json(&root, value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v16 recipe");
}
