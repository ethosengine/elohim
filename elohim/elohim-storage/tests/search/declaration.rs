//! The content search declarations (ruling R-S2): the `content-lexical-index` measure and the
//! `rrf-v2` content search recipe, compiled in and checked at boot. Their CIDs are pinned here —
//! every answer prints them, so a moved CID is a changed method, never a silent re-baseline.

use elohim_epr_rea::{atom_cid, RankingMethod};
use elohim_storage::search::measure::{addressed, Declared, MEASURE_JSON, RECIPE_JSON};
use serde_json::Value;

/// `IndexMeasure::cid()` of `content-lexical-index.json` — reach `{commons, ceiling}`, the widest
/// ring the peer serves; per-reader gating is at the route (ruling R-S8).
const MEASURE_CID: &str = "bafyreifa7afnbfcrvx4u3dilv77xdbnkaigl5lsakyo46xst7tqbdat6gi";
/// `atom_cid` of the recipe's addressed keys (`recipe`, `k`, `order_only`, `producers`).
const RECIPE_CID: &str = "bafyreiedggzebsgekiw26kdkcdot6rxqpwsvbozpozmdwmig2mhhba4x3q";
/// The recall measures' `_chunk_rule` CID: one chunker, two folds.
const RECALL_CHUNK_RULE: &str = "bafyreiebgg4stqugjqnytbqwlper4rleruh5ap4mt7f3rgifoxkd5odfly";

#[test]
fn measure_declaration_loads_validates_and_has_stable_cid() {
    let declared = Declared::load().expect("the compiled-in declarations load and validate");
    declared
        .measure
        .validate()
        .expect("IndexMeasure::validate accepts it");
    assert_eq!(declared.measure.ranking, RankingMethod::Bm25);
    assert!(
        declared.measure.embedding.is_none(),
        "lexical only: no model pin"
    );
    assert_eq!(declared.measure.chunk_rule.to_string(), RECALL_CHUNK_RULE);
    assert_eq!(declared.fold_lag_limit(), 200);
    // The declared reach is the serving ceiling, not `self`: the fold covers every row, and the
    // route gates each candidate per reader (R-S8). Read literally, a `self` ceiling would refuse
    // the commons rows the index is built to serve.
    assert!(declared.measure.admits(elohim_epr::reach::Reach::Commons));
    assert!(declared.measure.admits(elohim_epr::reach::Reach::Private));
    assert_eq!(
        declared.measure_cid, MEASURE_CID,
        "the measure's method CID moved"
    );
    assert_eq!(
        Declared::load().unwrap().measure_cid,
        declared.measure_cid,
        "two loads, one CID"
    );

    // No floats anywhere in either governed declaration.
    fn no_floats(value: &Value, at: &str) {
        match value {
            Value::Number(n) => assert!(!n.is_f64(), "a float at {at}: {n}"),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .for_each(|(i, v)| no_floats(v, &format!("{at}[{i}]"))),
            Value::Object(map) => map
                .iter()
                .for_each(|(k, v)| no_floats(v, &format!("{at}.{k}"))),
            _ => {}
        }
    }
    no_floats(&serde_json::from_str(MEASURE_JSON).unwrap(), "measure");
    no_floats(&serde_json::from_str(RECIPE_JSON).unwrap(), "recipe");

    // A chunk rule that does not address to chunkRule is refused: the method is the declaration.
    let mut tampered: Value = serde_json::from_str(MEASURE_JSON).unwrap();
    tampered["_chunk_rule"]["max_chunks_per_file"] = Value::from(41);
    let refused = Declared::from_json(&tampered.to_string(), RECIPE_JSON)
        .expect_err("a re-shaped chunk rule under the old CID is refused");
    assert!(refused.contains("chunkRule"), "{refused}");

    // A surface beyond the Content kind is refused: this fold reads content rows only.
    let mut wider: Value = serde_json::from_str(MEASURE_JSON).unwrap();
    wider["surfaces"]["kinds"] = serde_json::json!(["Content", "Manifest"]);
    assert!(Declared::from_json(&wider.to_string(), RECIPE_JSON).is_err());
}

#[test]
fn recipe_rrf_v2_single_producer_parses() {
    let declared = Declared::load().unwrap();
    assert_eq!(declared.recipe.name, "rrf-v2");
    assert_eq!(declared.recipe.k, 60);
    assert_eq!(declared.recipe.producers, vec!["lexical".to_string()]);
    assert_eq!(declared.recipe.cid, RECIPE_CID, "the recipe's CID moved");

    // The CID addresses recipe, k, order_only and producers only: the lens table and the measure
    // path are addressed elsewhere, so retuning a lens level leaves the recipe CID where it is.
    let raw: Value = serde_json::from_str(RECIPE_JSON).unwrap();
    let addressed_keys = addressed(&raw);
    let mut keys: Vec<&String> = addressed_keys.keys().collect();
    keys.sort();
    assert_eq!(keys, ["k", "order_only", "producers", "recipe"]);
    assert_eq!(
        atom_cid(&Value::Object(addressed(&raw)))
            .unwrap()
            .to_string(),
        declared.recipe.cid
    );
    // Key order is not part of the address (canonical dag-cbor sorts map keys): the same keys
    // inserted in reverse address to the same CID, whatever order serde_json preserves.
    let mut reversed = serde_json::Map::new();
    for (key, value) in addressed(&raw).into_iter().rev() {
        reversed.insert(key, value);
    }
    assert_eq!(
        atom_cid(&Value::Object(reversed)).unwrap().to_string(),
        declared.recipe.cid
    );
    let mut retuned = raw.clone();
    retuned["lens_table"]["levels"]["standard"]["choice_count"] = Value::from(25);
    let retuned = Declared::from_json(MEASURE_JSON, &retuned.to_string()).unwrap();
    assert_eq!(retuned.recipe.cid, declared.recipe.cid);
    assert_ne!(
        retuned.lens_cid, declared.lens_cid,
        "the lens table has its own CID"
    );

    // The learning app's lens table: minimal 5, standard 20 (the default), whole 100.
    assert_eq!(declared.lens_default, "standard");
    let counts: Vec<(&str, u32)> = declared
        .lens_levels
        .iter()
        .map(|(name, level)| (name.as_str(), level.choice_count))
        .collect();
    assert_eq!(counts, [("minimal", 5), ("standard", 20), ("whole", 100)]);

    // rrf-v2 over any producer but `lexical` is refused here.
    let mut other = raw;
    other["producers"] = serde_json::json!(["lexical", "semantic"]);
    assert!(Declared::from_json(MEASURE_JSON, &other.to_string()).is_err());
}
