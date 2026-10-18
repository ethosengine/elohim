use elohim_views::app_lifecycle::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use ts_rs::TS;

fn check<T: DeserializeOwned + Serialize>(fixtures: &Value, name: &str) {
    let source = fixtures[name].clone();
    let decoded: T = serde_json::from_value(source.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), source, "{name}");
    let mut invalid = source;
    invalid
        .as_object_mut()
        .unwrap()
        .insert("undeclared".into(), Value::Bool(true));
    assert!(serde_json::from_value::<T>(invalid).is_err(), "{name}");
}

#[test]
fn shared_schema_fixtures_roundtrip_without_wire_transforms() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/app-lifecycle.json")).unwrap();
    check::<AppReleaseView>(&fixtures, "app-release");
    check::<AppOpenInput>(&fixtures, "app-open-input");
    check::<AppInstanceView>(&fixtures, "app-instance");
    check::<AppResidencyView>(&fixtures, "app-residency");
    check::<AppContinuationView>(&fixtures, "app-continuation");
    check::<AppAttachmentInput>(&fixtures, "app-attachment-input");
    check::<AppRouteView>(&fixtures, "app-route");
    check::<AppSyncTargetView>(&fixtures, "app-sync-target");
    check::<AppSyncDemandView>(&fixtures, "app-sync-demand");
    check::<AppReleaseAnswer>(&fixtures, "app-release-answer");
    check::<AppInstanceAnswer>(&fixtures, "app-instance-answer");
    check::<AppSyncProgressView>(&fixtures, "app-sync-progress");
}

#[test]
fn reasons_cannot_relabel_timeout_as_observed_absence() {
    for invalid in [
        serde_json::json!({"state":"absent","reason":"timeout"}),
        serde_json::json!({"state":"unreachable","reason":"observed_absent"}),
        serde_json::json!({"state":"unreachable","reason":"made_up"}),
    ] {
        assert!(serde_json::from_value::<AppReleaseAnswer>(invalid.clone()).is_err());
        assert!(serde_json::from_value::<AppInstanceAnswer>(invalid).is_err());
    }
}

#[test]
fn reason_subsets_and_urgency_match_their_existing_schema_vocabulary() {
    let reasons: Value = serde_json::from_str(include_str!(
        "../../../elohim/sdk/schemas/v1/enums/answer-reason.schema.json"
    ))
    .unwrap();
    let native = serde_json::json!([
        AppAbsentReason::ObservedAbsent,
        AppUnreachableReason::Timeout,
        AppUnreachableReason::TransportError,
        AppUnreachableReason::Refused,
        AppUnreachableReason::Unverifiable,
        AppUnreachableReason::NotYetDelivered
    ]);
    assert_eq!(native, reasons["enum"]);
    let urgency: Value = serde_json::from_str(include_str!(
        "../../../elohim/sdk/schemas/v1/enums/app-sync-urgency.schema.json"
    ))
    .unwrap();
    assert_eq!(
        serde_json::json!([AppSyncUrgency::Background, AppSyncUrgency::Foreground]),
        urgency["enum"]
    );
}

fn fresh<T: TS + 'static>() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../elohim/sdk/storage-client-ts/src/generated")
        .join(format!("{}.ts", T::name()));
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        T::export_to_string().unwrap(),
        "regenerate app lifecycle bindings with cargo test --lib app_lifecycle::"
    );
}

#[test]
fn generated_clients_match_native_contract() {
    fresh::<AppReleaseView>();
    fresh::<AppOpenInput>();
    fresh::<AppInstanceView>();
    fresh::<AppResidencyView>();
    fresh::<AppContinuationView>();
    fresh::<AppAttachmentInput>();
    fresh::<AppRouteView>();
    fresh::<AppSyncTargetView>();
    fresh::<AppSyncUrgency>();
    fresh::<AppSyncDemandView>();
    fresh::<AppReleaseAnswer>();
    fresh::<AppInstanceAnswer>();
    fresh::<AppAbsentReason>();
    fresh::<AppUnreachableReason>();
    fresh::<AppSyncProgressView>();
}
