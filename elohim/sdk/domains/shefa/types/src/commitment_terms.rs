//! The current audience may change without rewriting the undertaking's other terms.
//! This is a structural boundary, not proof of a collective's ruling authority.

use serde_json::{Map, Value};

// No larger than storage's existing authenticated Commitment record budget.
const MAX_COMMITMENT_METADATA_BYTES: usize = 256 * 1024;

fn terms(raw: &str) -> Result<Map<String, Value>, String> {
    if raw.len() > MAX_COMMITMENT_METADATA_BYTES {
        return Err("Commitment terms exceed the record budget".into());
    }
    let value: Value =
        serde_json::from_str(raw).map_err(|_| "Commitment terms must be valid JSON".to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "Commitment terms must be a JSON object".to_string())?;
    if let Some(reach) = object.get("reach") {
        if reach.as_str().is_none_or(|s| s.trim().is_empty()) {
            return Err("Commitment reach must be a nonempty string".into());
        }
    }
    if let Some(hints) = object.get("gateHints") {
        let hints = hints
            .as_array()
            .ok_or_else(|| "Commitment gateHints must be an array".to_string())?;
        for hint in hints {
            let hint = hint
                .as_object()
                .ok_or_else(|| "Commitment gate hint must be an object".to_string())?;
            for key in ["eprRef", "relation"] {
                if hint
                    .get(key)
                    .and_then(Value::as_str)
                    .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(format!(
                        "Commitment gate hint {key} must be a nonempty string"
                    ));
                }
            }
            if hint.get("label").is_some_and(|label| !label.is_string()) {
                return Err("Commitment gate hint label must be a string".into());
            }
        }
    }
    Ok(object.clone())
}

/// Validate explicit current terms even when they repeat the stored bytes.
/// Reach vocabulary remains owned by existing protocol classifiers.
pub fn validate_project_epr_current_terms(raw: &str) -> Result<(), String> {
    terms(raw).map(|_| ())
}

/// Permit only project-epr reach/audience changes on the existing lineage.
/// Every other key, including unknown future keys, remains immutable here.
/// Identical legacy bytes are readable without inventing a new declaration.
pub fn same_commitment_metadata(action: &str, old: &str, new: &str) -> Result<bool, String> {
    if old == new {
        return Ok(true);
    }
    if action != "project-epr" {
        return Ok(false);
    }
    let mut old = terms(old)?;
    let mut new = terms(new)?;
    for key in ["reach", "gateHints"] {
        old.remove(key);
        new.remove(key);
    }
    Ok(old == new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_update_wire_preserves_absent_current_terms() {
        let legacy = serde_json::json!({"id":"undertaking","state":"active","finished":false});
        let bytes = rmp_serde::to_vec_named(&legacy).unwrap();
        let input: crate::UpdateReaCommitmentStateInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(input.id, "undertaking");
        assert_eq!(input.finished, Some(false));
        assert!(input.project_epr_current_terms_json.is_none());
    }

    #[test]
    fn current_audience_changes_preserve_other_terms() {
        let old = r#"{"reach":"commons","gateHints":[],"urlPath":"/garden","responsiveReach":{"withinHours":72}}"#;
        let new = r#"{"responsiveReach":{"withinHours":72},"urlPath":"/garden","reach":"household","gateHints":[{"eprRef":"household-dowell","relation":"membershipPrerequisite","label":"Dowell"}]}"#;
        assert!(same_commitment_metadata("project-epr", old, new).unwrap());
        assert!(!same_commitment_metadata("provide", old, new).unwrap());
    }

    #[test]
    fn routing_redress_and_unknown_terms_require_supersession() {
        for key in [
            "urlPath",
            "mode",
            "hostnames",
            "channel",
            "baseHref",
            "entryFile",
            "responsiveReach",
            "hostingAgreementId",
            "futureTerm",
        ] {
            let old = serde_json::json!({"reach":"commons",(key):"original"}).to_string();
            let new = serde_json::json!({"reach":"private",(key):"changed"}).to_string();
            assert!(
                !same_commitment_metadata("project-epr", &old, &new).unwrap(),
                "{key}"
            );
        }
    }

    #[test]
    fn explicit_malformed_terms_refuse_but_unchanged_legacy_is_readable() {
        assert!(same_commitment_metadata("provide", "legacy", "legacy").unwrap());
        for bad in [
            "legacy",
            "[]",
            "null",
            r#"{"reach":null}"#,
            r#"{"reach":" "}"#,
            r#"{"gateHints":{}}"#,
            r#"{"gateHints":[{}]}"#,
            r#"{"gateHints":[{"eprRef":"x","relation":2}]}"#,
        ] {
            assert!(validate_project_epr_current_terms(bad).is_err(), "{bad}");
            assert!(
                same_commitment_metadata("project-epr", "{}", bad).is_err(),
                "{bad}"
            );
        }
        assert!(
            validate_project_epr_current_terms(&" ".repeat(MAX_COMMITMENT_METADATA_BYTES + 1))
                .is_err()
        );
    }
}
