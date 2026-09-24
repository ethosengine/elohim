//! The declared fusion recipe and reciprocal-rank fusion, for order only.
use elohim_epr_index::fuse::{fuse, Recipe, RRF_V1, RRF_V2};
use elohim_epr_rea::atom_cid;
use serde_json::{json, Value};

fn candidate(path: &str) -> Value {
    json!({ "path": path })
}

#[test]
fn rrf_v2_accepts_single_producer() {
    let object = json!({"recipe": "rrf-v2", "k": 60, "producers": ["lexical"], "order_only": true});
    let recipe = Recipe::from_declared(&object).expect("a single producer is a recipe");
    assert_eq!(recipe.name, RRF_V2);
    assert_eq!(recipe.producers, vec!["lexical"]);
    // One producer fuses to its own order.
    let fused = fuse(
        &[(
            "lexical".into(),
            vec![candidate("b"), candidate("a"), candidate("c")],
        )],
        recipe.k,
    );
    let order: Vec<&str> = fused.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(order, vec!["b", "a", "c"]);
    assert_eq!(fused[0].candidate["ranks"], json!({"lexical": 1}));

    let empty = json!({"recipe": "rrf-v2", "k": 60, "producers": [], "order_only": true});
    let refused = Recipe::from_declared(&empty).unwrap_err();
    assert!(refused.contains("producer"), "{refused}");
    let summing =
        json!({"recipe": "rrf-v2", "k": 60, "producers": ["lexical"], "order_only": false});
    assert!(Recipe::from_declared(&summing)
        .unwrap_err()
        .contains("order"));
}

#[test]
fn rrf_v1_still_requires_local_first() {
    for producers in [
        json!(["semantic"]),
        json!(["local"]),
        json!(["semantic", "local"]),
    ] {
        let object =
            json!({"recipe": "rrf-v1", "k": 60, "producers": producers, "order_only": true});
        let refused = Recipe::from_declared(&object).unwrap_err();
        assert!(refused.contains("local"), "{refused}");
    }
    let object = json!({"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"], "order_only": true});
    let recipe = Recipe::from_declared(&object).expect("the agent contract's recipe parses");
    assert_eq!(recipe.name, RRF_V1);
    assert_eq!(recipe.producers, vec!["local", "semantic"]);

    let unknown = json!({"recipe": "sum-v1", "k": 60, "producers": ["local", "semantic"], "order_only": true});
    assert!(Recipe::from_declared(&unknown)
        .unwrap_err()
        .contains("sum-v1"));
    let zero =
        json!({"recipe": "rrf-v1", "k": 0, "producers": ["local", "semantic"], "order_only": true});
    assert!(Recipe::from_declared(&zero).unwrap_err().contains("k"));
}

#[test]
fn recipe_cid_is_atom_cid_of_object() {
    for object in [
        json!({"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"], "order_only": true}),
        json!({"recipe": "rrf-v2", "k": 60, "producers": ["lexical"], "order_only": true}),
    ] {
        let recipe = Recipe::from_declared(&object).unwrap();
        assert_eq!(recipe.cid, atom_cid(&object).unwrap().to_string());
    }
}

#[test]
fn the_fused_order_is_reciprocal_rank_over_the_producer_orders() {
    let local = vec![candidate("a.md"), candidate("b.md"), candidate("c.md")];
    let semantic = vec![candidate("c.md"), candidate("d.md"), candidate("a.md")];
    let fused = fuse(
        &[("local".into(), local), ("semantic".into(), semantic)],
        60,
    );
    let order: Vec<&str> = fused.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(order, vec!["a.md", "c.md", "b.md", "d.md"]);
    assert!((fused[0].score - (1.0 / 61.0 + 1.0 / 63.0)).abs() < 1e-12);
    assert_eq!(
        fused[3].candidate["ranks"],
        json!({"local": null, "semantic": 2})
    );
}
