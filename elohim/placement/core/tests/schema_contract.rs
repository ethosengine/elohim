//! The Rust enums against their SDK schemas (interfaces spec §G, G11/G12), and the `ManifestNeeds`
//! mirror against `$defs.ManifestNeeds` in `manifest/app-manifest.schema.json`.
//!
//! The schemas are read with `include_str!` (no hand-written golden list in node), so a schema value
//! added without its Rust variant, or the reverse, fails this crate's build of its tests.

use placement_core::*;

macro_rules! schema {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../sdk/schemas/v1/",
            $path
        ))
    };
}

fn labels<T: serde::Serialize>(values: &[T]) -> Vec<String> {
    values
        .iter()
        .map(|v| {
            serde_json::to_value(v)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn enum_values(name: &str, text: &str) -> Vec<String> {
    let json: serde_json::Value =
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{name}: {e}"));
    json["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{name} has no enum array"))
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn every_enum_schema_equals_its_rust_enum_in_order() {
    let cases: Vec<(&str, &str, Vec<String>)> = vec![
        (
            "placement-kind",
            schema!("enums/placement-kind.schema.json"),
            labels(KindId::ALL),
        ),
        (
            "grant-scope",
            schema!("enums/grant-scope.schema.json"),
            labels(GrantScope::ALL),
        ),
        (
            "loss-domain",
            schema!("enums/loss-domain.schema.json"),
            labels(&[
                LossDomain::Device,
                LossDomain::Household,
                LossDomain::Region,
            ]),
        ),
        (
            "need-class",
            schema!("enums/need-class.schema.json"),
            labels(NeedClass::BUILTIN_ORDER),
        ),
        (
            "depth",
            schema!("enums/depth.schema.json"),
            labels(&[Depth::Query, Depth::Sparse, Depth::OnRead, Depth::Kept]),
        ),
        (
            "isolation",
            schema!("enums/isolation.schema.json"),
            labels(&[
                Isolation::None,
                Isolation::Intrinsic,
                Isolation::Delegated,
                Isolation::Bounded,
                Isolation::Enforced,
            ]),
        ),
        (
            "placement-factor",
            schema!("enums/placement-factor.schema.json"),
            labels(Factor::ALL),
        ),
    ];
    for (name, text, rust) in cases {
        assert_eq!(
            enum_values(name, text),
            rust,
            "enums/{name}.schema.json disagrees with the Rust enum"
        );
    }
}

fn needs_defs() -> serde_json::Value {
    let json: serde_json::Value =
        serde_json::from_str(schema!("manifest/app-manifest.schema.json")).unwrap();
    json["$defs"]["ManifestNeeds"]["$defs"].clone()
}

#[test]
fn every_field_the_mirror_reads_has_its_shape_in_the_schema() {
    let defs = needs_defs();
    for def in ["StartSet", "WorksOffline", "Loss", "Compute"] {
        assert!(
            defs.get(def).is_some(),
            "$defs.ManifestNeeds.$defs.{def} is missing"
        );
    }
    // worksOffline is the depth vocabulary without `query` (G7).
    let works = &defs["WorksOffline"]["allOf"];
    assert_eq!(works[0]["$ref"], "../enums/depth.schema.json");
    assert_eq!(works[1]["not"]["const"], "query");
    for (value, expected) in [
        ("sparse", WorksOffline::Sparse),
        ("on-read", WorksOffline::OnRead),
        ("kept", WorksOffline::Kept),
    ] {
        let got: WorksOffline = serde_json::from_value(serde_json::json!(value)).unwrap();
        assert_eq!(got, expected);
    }
    assert!(serde_json::from_value::<WorksOffline>(serde_json::json!("query")).is_err());
    // loss maps a content type to a loss domain.
    assert_eq!(
        defs["Loss"]["additionalProperties"]["$ref"],
        "../enums/loss-domain.schema.json"
    );
    // compute: { scope: grant-scope, bounds: { ratePerHour } }.
    let item = &defs["Compute"]["items"];
    assert_eq!(
        item["properties"]["scope"]["$ref"],
        "../enums/grant-scope.schema.json"
    );
    assert!(item["properties"]["bounds"]["properties"]
        .get("ratePerHour")
        .is_some());
    // closure is the view paths, an array.
    assert_eq!(
        defs["StartSet"]["$ref"],
        "../objects/tree-view.schema.json#/$defs/ViewPaths"
    );
}

#[test]
fn the_mirror_reads_a_needs_block_in_the_schemas_shape() {
    let n: ManifestNeeds = serde_json::from_value(serde_json::json!({
        "closure": [""],
        "worksOffline": "kept",
        "loss": { "concept": "household" },
        "compute": [{ "scope": "measure-stage", "bounds": { "ratePerHour": 4 } }]
    }))
    .unwrap();
    assert_eq!(n.closure, Some(vec![String::new()]));
    assert_eq!(n.loss.unwrap()["concept"], LossDomain::Household);
    assert_eq!(n.compute.unwrap()[0].bounds.rate_per_hour, Some(4));
}
