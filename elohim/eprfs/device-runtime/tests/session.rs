use elohim_device_runtime::session::{resolve_with, Registry};
use serde_json::Value;
use std::{fs, path::Path};

fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../genesis/agentic/fixtures/session-cases.json"
    ))
    .unwrap()
}

fn fixture(root: &Path, case: &Value) {
    for row in case["processes"].as_array().unwrap() {
        let row = row.as_array().unwrap();
        let mut fields = vec!["0".to_owned(); 20];
        fields[0] = "S".into();
        for (index, col) in [(1, 1), (3, 3), (4, 4), (19, 5)] {
            fields[index] = row[col].to_string();
        }
        let dir = root.join(row[0].to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("stat"),
            format!(
                "{} ({}) {}",
                row[0],
                row[2].as_str().unwrap(),
                fields.join(" ")
            ),
        )
        .unwrap();
    }
}

#[test]
fn session_shared_fixture_cases() {
    for case in cases() {
        let tmp = tempfile::tempdir().unwrap();
        fixture(tmp.path(), &case);
        let registry: Registry = serde_json::from_value(case["registry"].clone()).unwrap();
        let env = |key: &str| case["env"][key].as_str().map(str::to_owned);
        let identity = resolve_with(
            &env,
            tmp.path(),
            case["startPid"].as_u64().unwrap() as u32,
            &registry,
        );
        assert_eq!(
            serde_json::to_value(identity).unwrap(),
            case["expected"],
            "{}",
            case["name"]
        );
    }
}

#[test]
fn session_schema_contract() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/views/session-identity.schema.json"
    ))
    .unwrap();
    let sources: Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/enums/session-source.schema.json"
    ))
    .unwrap();
    for case in cases() {
        let tmp = tempfile::tempdir().unwrap();
        fixture(tmp.path(), &case);
        let registry = serde_json::from_value(case["registry"].clone()).unwrap();
        let value = serde_json::to_value(resolve_with(
            &|key| case["env"][key].as_str().map(str::to_owned),
            tmp.path(),
            400,
            &registry,
        ))
        .unwrap();
        let object = value.as_object().unwrap();
        for key in schema["required"].as_array().unwrap() {
            assert!(object.contains_key(key.as_str().unwrap()));
        }
        assert_eq!(
            object.len(),
            schema["properties"].as_object().unwrap().len()
        );
        assert!(sources["enum"]
            .as_array()
            .unwrap()
            .contains(&value["source"]));
        assert!(value["runtime"].is_string());
        assert!(
            value["ownerStartTime"].is_null()
                || value["ownerStartTime"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .is_ok()
        );
        assert!(value["session"].is_null() || value["session"].is_string());
        assert!(value["ownerPid"].is_null() || value["ownerPid"].is_u64());
    }
}
