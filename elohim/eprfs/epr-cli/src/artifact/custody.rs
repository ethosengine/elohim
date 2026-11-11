//! Custody queries and local divergence observations over the existing Produce flow.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use elohim_epr_rea::{
    AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
};
use eprfs_core::BlobCid;
use eprfs_peer::{family_variants, fold_holders, HolderSelection, SiblingRecord};

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
        let variants: Vec<_> = family_variants(&records, family)
            .into_iter()
            .map(|variant| {
                let holders: Vec<_> = variant
                    .holders
                    .iter()
                    .map(|row| {
                        serde_json::json!({"device": row.device, "output": row.output,
                    "requires": row.requires, "self": row.device == found.device})
                    })
                    .collect();
                if !options.json {
                    println!(
                        "key {} requires {}",
                        variant.key,
                        variant
                            .requires
                            .as_ref()
                            .map_or_else(|| "unclaimed or conflicting".into(), ToString::to_string)
                    );
                    for row in &variant.holders {
                        let marker = if row.device == found.device {
                            " self"
                        } else {
                            ""
                        };
                        println!(
                            "  {} {} requires {}{marker}",
                            row.device,
                            row.output,
                            row.requires
                                .as_ref()
                                .map_or_else(|| "unclaimed".into(), ToString::to_string)
                        );
                    }
                }
                serde_json::json!({"key": variant.key, "outputs": variant.outputs,
                "requires": variant.requires, "holders": holders})
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
