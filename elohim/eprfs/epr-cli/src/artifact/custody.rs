//! Custody queries and local divergence observations over the existing Produce flow.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use elohim_epr_rea::{
    AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
};
use eprfs_core::BlobCid;
use eprfs_peer::{family_variants, fold_holders, HolderSelection, Requires, SiblingRecord};

use super::{ArtifactError, ArtifactResult, Options};
use crate::peer::{self, SiblingArtifacts};

pub(super) fn parse_cid(value: &str) -> ArtifactResult<BlobCid> {
    BlobCid::parse(value).map_err(|e| ArtifactError::InvalidArguments(e.to_string()))
}

pub(super) fn records(found: &SiblingArtifacts) -> Vec<SiblingRecord> {
    found
        .rows
        .iter()
        .filter_map(|row| {
            Some(SiblingRecord {
                key: BlobCid::parse(&row.key).ok()?,
                family: row
                    .family
                    .as_deref()
                    .and_then(|cid| BlobCid::parse(cid).ok()),
                requires: row
                    .requires
                    .as_deref()
                    .and_then(|cid| BlobCid::parse(cid).ok()),
                device: row.device.clone(),
                output: BlobCid::parse(&row.output).ok()?,
                manifest: BlobCid::parse(&row.manifest).ok()?,
            })
        })
        .collect()
}

/// Each document's bytes when this device holds them and they hash to their CID. Local
/// possession only; this query never fetches from siblings.
fn local_documents(
    options: &Options,
    cids: &BTreeSet<BlobCid>,
) -> ArtifactResult<BTreeMap<BlobCid, Option<Vec<u8>>>> {
    let mut documents: BTreeMap<_, _> = cids.iter().cloned().map(|cid| (cid, None)).collect();
    if cids.is_empty() {
        return Ok(documents);
    }
    let payload_dir = peer::payload_store_dir(&peer::Options {
        root: options.repo.clone(),
        payload_store: options.payload_store.clone(),
        ..peer::Options::default()
    })?;
    if !payload_dir.exists() {
        return Ok(documents);
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let store =
            crate::payload::open_payload_store(&payload_dir, crate::payload::Access::Shared)
                .await
                .map_err(|e| ArtifactError::Gap(format!("local Requires store: {e}")))?
                .store;
        for (cid, slot) in &mut documents {
            if let Ok(blob) = store
                .fetch_blob(cid, eprfs_core::FetchPolicy::LocalOnly)
                .await
            {
                if BlobCid::compute_bytes(&blob.bytes) == *cid {
                    *slot = Some(blob.bytes.to_vec());
                }
            }
        }
        Ok(documents)
    })
}

fn requires_availability(
    options: &Options,
    cids: &BTreeSet<BlobCid>,
) -> ArtifactResult<BTreeMap<BlobCid, &'static str>> {
    Ok(local_documents(options, cids)?
        .into_iter()
        .map(|(cid, bytes)| {
            (
                cid,
                if bytes.is_some() {
                    "local"
                } else {
                    "unavailable"
                },
            )
        })
        .collect())
}

/// `requires` exit for an output this device's store has no custody Produce of.
const EXIT_UNKNOWN_OUTPUT: u8 = 2;
/// `requires` exit when a recorded requirement document is not held locally.
const EXIT_UNAVAILABLE: u8 = super::EXIT_GAP;
/// `requires` exit when the output's recorded requirement documents do not merge.
const EXIT_CONFLICT: u8 = 4;

/// `epr artifact requires --output <cid>`: the recorded requirement of one output, read from
/// this device's own flow store and payload store, never the network (plan item K13).
///
/// `self` is true only when this device's own key recorded a custody Produce of the output (a
/// local `epr artifact put`); a sibling's crossed Produce and a fetch never make it true, and no
/// flag can. `produce` names that record, the one an own-build claim may cite. Several Produce
/// events for one output merge through `Requires::try_from`; a Produce with no `env:requires`
/// slot adds nothing, and an output whose Produce events carry none at all is `unmeasured`.
pub(super) fn requires(options: &Options) -> ArtifactResult<ExitCode> {
    let output = options
        .output
        .as_ref()
        .ok_or_else(|| ArtifactError::InvalidArguments("requires needs --output CID".into()))?;
    let found = peer::custody_artifacts(&options.repo, options.handle.as_deref())?;
    let rows: Vec<_> = found
        .rows
        .iter()
        .filter(|row| BlobCid::parse(&row.output).is_ok_and(|cid| &cid == output))
        .collect();
    let mut own: Vec<_> = rows
        .iter()
        .filter(|row| row.device == found.device)
        .collect();
    own.sort_by(|a, b| (&a.at, &a.record).cmp(&(&b.at, &b.record)));
    let produce = own.first().map(|row| row.record.clone());
    let is_self = produce.is_some();
    let who = if is_self { "self" } else { "other" };
    let report = |line: String, fields: serde_json::Value| {
        if options.json {
            let mut value = serde_json::json!({
                "output": output, "self": is_self, "produce": produce,
            });
            value
                .as_object_mut()
                .expect("object literal")
                .extend(fields.as_object().expect("object literal").clone());
            println!("{value}");
        } else {
            println!("{line}");
        }
    };
    if rows.is_empty() {
        report(
            format!("unknown {output}: no custody Produce of it on this device's store"),
            serde_json::json!({"known": false, "requires": null, "path": null}),
        );
        return Ok(ExitCode::from(EXIT_UNKNOWN_OUTPUT));
    }
    let mut cids = BTreeSet::new();
    for slot in rows.iter().filter_map(|row| row.requires.as_deref()) {
        cids.insert(BlobCid::parse(slot).map_err(|e| {
            ArtifactError::Gap(format!(
                "requires gap: {output} carries an unreadable env:requires {slot}: {e}"
            ))
        })?);
    }
    if cids.is_empty() {
        report(
            format!("unmeasured {output}: no recorded requirement {who}"),
            serde_json::json!({
                "known": true, "requires": null, "path": null,
                "unmeasured": "no recorded requirement",
            }),
        );
        return Ok(ExitCode::SUCCESS);
    }
    let documents = local_documents(options, &cids)?;
    let missing: Vec<_> = documents
        .iter()
        .filter(|(_, bytes)| bytes.is_none())
        .map(|(cid, _)| cid.to_string())
        .collect();
    if !missing.is_empty() {
        report(
            format!(
                "unavailable {output}: requirement {} not held locally {who}",
                missing.join(", ")
            ),
            serde_json::json!({
                "known": true, "requires": null, "path": null, "unavailable": missing,
            }),
        );
        return Ok(ExitCode::from(EXIT_UNAVAILABLE));
    }
    let sources: Vec<_> = documents.keys().map(ToString::to_string).collect();
    let mut clauses: Vec<host_contract::Clause> = Vec::new();
    for (cid, bytes) in &documents {
        // Decoding is the checked merge, so a stored document that would not construct is
        // refused here rather than handed on as bytes.
        let document: host_contract::Requires = serde_json::from_slice(
            bytes.as_deref().expect("missing refused above"),
        )
        .map_err(|e| {
            ArtifactError::Gap(format!(
                "requires gap: {cid} is not a requirement document: {e}"
            ))
        })?;
        clauses.extend(Vec::<host_contract::Clause>::from(document));
    }
    let (requires, bytes) = match host_contract::Requires::try_from(clauses) {
        Err(error) => {
            report(
                format!("conflict {output}: MergeError: {error} {who}"),
                serde_json::json!({
                    "known": true, "requires": null, "path": null, "sources": sources,
                    "conflict": error.to_string(),
                }),
            );
            return Ok(ExitCode::from(EXIT_CONFLICT));
        }
        // One recorded document is handed on byte for byte; several are merged canonically.
        Ok(_) if documents.len() == 1 => {
            let (cid, bytes) = documents.into_iter().next().expect("one document");
            (cid, bytes.expect("missing refused above"))
        }
        Ok(merged) => {
            let (cid, bytes) = super::requires::canonical(&merged)?;
            (cid, bytes.to_vec())
        }
    };
    if let Some(path) = &options.write {
        std::fs::write(path, &bytes)?;
    }
    report(
        format!("requires {requires} {who}"),
        serde_json::json!({
            "known": true, "requires": requires, "path": options.write, "sources": sources,
        }),
    );
    Ok(ExitCode::SUCCESS)
}

fn requires_text(requires: &Requires, availability: &BTreeMap<BlobCid, &'static str>) -> String {
    let claim = |cid: &Option<BlobCid>| {
        cid.as_ref().map_or_else(
            || "Unclaimed".into(),
            |cid| format!("{cid} ({})", availability[cid]),
        )
    };
    match requires {
        Requires::Unanimous(cid) => format!("Unanimous({})", claim(&Some(cid.clone()))),
        Requires::Unclaimed => "Unclaimed".into(),
        Requires::Conflict(claims) => format!(
            "Conflict{{{}}}",
            claims.iter().map(claim).collect::<Vec<_>>().join(", ")
        ),
    }
}

pub(super) fn holders(options: &Options) -> ArtifactResult<ExitCode> {
    if [
        options.key.is_some(),
        options.output.is_some(),
        options.family.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
        != 1
    {
        return Err(ArtifactError::InvalidArguments(
            "holders needs exactly one of --key, --output or --family".into(),
        ));
    }
    let found = peer::custody_artifacts(&options.repo, options.handle.as_deref())?;
    let records = records(&found);
    if let Some(family) = &options.family {
        let variants = family_variants(&records, family);
        let cids: BTreeSet<_> = variants
            .iter()
            .flat_map(|v| v.claims.values().flatten())
            .filter_map(|c| c.requires.clone())
            .collect();
        let availability = requires_availability(options, &cids)?;
        let variants: Vec<_> = variants
            .into_iter()
            .map(|variant| {
                if !options.json {
                    println!(
                        "key {} requires {}",
                        variant.key,
                        requires_text(&variant.requires, &availability)
                    );
                }
                let holders: Vec<_> = variant
                    .holders
                    .iter()
                    .map(|row| {
                        let requires = Requires::from_claims(
                            variant.claims[&row.output]
                                .iter()
                                .filter(|claim| claim.device == row.device)
                                .map(|claim| claim.requires.clone()),
                        );
                        if !options.json {
                            let marker = if row.device == found.device {
                                " self"
                            } else {
                                ""
                            };
                            println!(
                                "  {} {} requires {}{marker}",
                                row.device,
                                row.output,
                                requires_text(&requires, &availability)
                            );
                        }
                        serde_json::json!({"device": row.device, "output": row.output,
                    "requires": requires, "self": row.device == found.device})
                    })
                    .collect();
                let claims: Vec<_> = variant
                    .claims
                    .iter()
                    .flat_map(|(output, claims)| {
                        let availability = &availability;
                        claims.iter().map(move |claim| {
                            serde_json::json!({"output": output, "device": claim.device,
                    "requires": claim.requires,
                    "availability": claim.requires.as_ref().map(|cid| availability[cid])})
                        })
                    })
                    .collect();
                serde_json::json!({"key": variant.key, "outputs": variant.outputs,
                "requires": variant.requires, "holders": holders, "claims": claims})
            })
            .collect();
        if options.json {
            println!(
                "{}",
                serde_json::json!({"family": family, "variants": variants})
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    let mut outputs: BTreeMap<BlobCid, BTreeSet<String>> = BTreeMap::new();
    // An output with no recorded holders still has a meaningful zero-custody warning.
    if let Some(output) = &options.output {
        outputs.insert(output.clone(), BTreeSet::new());
    }
    for row in fold_holders(&records) {
        if options.key.as_ref().is_some_and(|key| key != &row.key)
            || options
                .output
                .as_ref()
                .is_some_and(|output| output != &row.output)
        {
            continue;
        }
        outputs
            .entry(row.output.clone())
            .or_default()
            .insert(row.device.clone());
    }
    let min = options.min.unwrap_or(2);
    let mut holders = Vec::new();
    let mut summaries = Vec::new();
    for (output, devices) in outputs {
        // This is the caller's advisory query threshold, not a governed bound or admission gate.
        let below_min = (devices.len() as u64) < min;
        summaries.push(
            serde_json::json!({"output": output, "count": devices.len(), "belowMin": below_min}),
        );
        for device in &devices {
            let is_self = device == &found.device;
            holders.push(serde_json::json!({"device": device, "output": output, "self": is_self}));
            if !options.json {
                let marker = if is_self { " self" } else { "" };
                println!("{device} {output}{marker}");
            }
        }
        if below_min && !options.json {
            eprintln!(
                "warning: {} holder(s) of {output}, fewer than {min}",
                devices.len()
            );
        }
    }
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "holders": holders,
                "belowMin": summaries.iter().any(|row| row["belowMin"] == true),
                "outputs": summaries,
            })
        );
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn record_divergence(
    root: &Path,
    device: &str,
    key: &BlobCid,
    selection: &HolderSelection,
) -> ArtifactResult<()> {
    if selection.outputs.len() < 2 || selection.rule == "expected" {
        return Ok(());
    }
    let local_error =
        |e: elohim_epr_rea::FabricError| ArtifactError::Gap(format!("fetch gap: local-store: {e}"));
    let store = SidecarFlowStore::open(root).map_err(local_error)?;
    // Serialize the check and append: simultaneous fetches cannot duplicate the observation.
    let mut flow = store.transaction().map_err(local_error)?;
    let key_slot = format!("env:key={key}");
    let mut outputs: Vec<_> = selection.outputs.iter().map(ToString::to_string).collect();
    outputs.sort();
    let outputs_slot = format!("env:outputs={}", outputs.join(","));
    if flow
        .events()
        .map_err(local_error)?
        .iter()
        .any(|(_, event)| {
            event.action == ReaVerb::Cite
                && event
                    .classified_as
                    .iter()
                    .any(|s| s == "env:kind=divergence")
                && event.classified_as.contains(&key_slot)
                && event.classified_as.contains(&outputs_slot)
        })
    {
        return Ok(());
    }
    let scope = crate::flow::repository::repository_scope(root)
        .map_err(|e| ArtifactError::Gap(format!("fetch gap: local-store: {e}")))?;
    flow.append(FlowRecord::Event(FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef(device.into()),
        receiver: AgentRef(device.into()),
        resource: *key.as_cid(),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: crate::flow::note::NOTE_UNIT.into(),
        },
        process: None,
        in_scope_of: scope,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        classified_as: vec![
            "run:observation".into(),
            key.to_string(),
            "env:kind=divergence".into(),
            key_slot,
            outputs_slot,
            format!("env:chosen={}", selection.output),
            format!("env:rule={}", selection.rule),
        ],
    }))
    .map_err(local_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_divergence_observations_share_one_key_and_output_set() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".epr-meta")).unwrap();
        std::fs::write(
            root.path().join(".epr-meta/repository.yaml"),
            "version: 1\nagent: repo:test/custody\n",
        )
        .unwrap();
        let key = BlobCid::compute(b"key");
        let x = BlobCid::compute_bytes(b"x");
        let y = BlobCid::compute_bytes(b"y");
        let chosen = HolderSelection {
            output: x.clone(),
            holders: vec![],
            rule: "plurality",
            outputs: vec![x, y],
        };
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            for i in 0..4 {
                let mut choice = chosen.clone();
                if i % 2 == 0 {
                    choice.outputs.reverse();
                }
                let (root, key, barrier) = (root.path(), &key, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    record_divergence(root, "device", key, &choice).unwrap();
                });
            }
        });
        let store = SidecarFlowStore::open(root.path()).unwrap();
        assert_eq!(store.events().unwrap().len(), 1);
        // A genuinely new output set is new evidence, even for the same chosen output/key.
        let mut changed = chosen.clone();
        changed.outputs.push(BlobCid::compute_bytes(b"z"));
        changed.rule = "expected";
        record_divergence(root.path(), "device", &key, &changed).unwrap();
        assert_eq!(
            store.events().unwrap().len(),
            1,
            "an explicit expectation records no divergence"
        );
        changed.rule = "own";
        record_divergence(root.path(), "device", &key, &changed).unwrap();
        assert_eq!(store.events().unwrap().len(), 2);
    }
}
