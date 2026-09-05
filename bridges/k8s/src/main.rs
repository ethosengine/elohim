use ark_core::manifest::RuntimeManifest;
use k8s_bridge::{render_envelope, verify, DriftVerdict, Result};
use std::{env, fs, path::Path, process::ExitCode};

fn run(args: &[String]) -> Result<u8> {
    match args.first().map(String::as_str) {
        Some("observe") => observe(&args[1..]),
        Some("render") if args.len() == 2 || (args.len() == 4 && args[2] == "--human") => {
            let manifest = RuntimeManifest::from_json(&fs::read_to_string(&args[1])?)?;
            let rendered = render_envelope(&manifest)?;
            eprintln!(
                "{}manifest CID: {}",
                if args.len() == 4 {
                    format!("{}: ", args[3])
                } else {
                    String::new()
                },
                manifest.cid()?
            );
            println!("{}", serde_json::to_string(&rendered)?);
            Ok(0)
        }
        Some("verify")
            if args.len() == 5 && args[1] == "--deployments" && args[3] == "--manifests-dir" =>
        {
            let verdicts = verify(Path::new(&args[2]), Path::new(&args[4]))?;
            let fresh = verdicts.iter().all(|v| *v == DriftVerdict::Fresh);
            println!("{}", serde_json::to_string(&verdicts)?);
            // 2026-09-05: all active humans pinned; Unpinned now refuses.
            Ok(if fresh { 0 } else { 65 })
        }
        _ => {
            eprintln!("usage: k8s-bridge render <manifest.json> [--human <name>] | verify --deployments <path> --manifests-dir <dir> | observe --ledger <path> [--dry-run] [--prometheus-url <url>]");
            Ok(64)
        }
    }
}

fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(65)
        }
    }
}

fn observe(args: &[String]) -> Result<u8> {
    observe_with(args, k8s_bridge::Observation::fetch)
}

fn observe_with(
    args: &[String],
    fetch: impl FnOnce(&str) -> Result<k8s_bridge::Observation>,
) -> Result<u8> {
    let mut ledger_path = None;
    let mut url = None;
    let mut dry_run = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ledger" => ledger_path = Some(args.next().ok_or("--ledger requires a path")?),
            "--prometheus-url" => {
                url = Some(
                    args.next()
                        .ok_or("--prometheus-url requires a URL")?
                        .clone(),
                )
            }
            "--dry-run" => dry_run = true,
            _ => return Err(format!("unknown observe argument: {arg}").into()),
        }
    }
    let url = url
        .or_else(|| env::var("PROMETHEUS_URL").ok())
        .filter(|s| !s.trim().is_empty())
        .ok_or("observe refuses: set PROMETHEUS_URL or pass --prometheus-url")?;
    let path = ledger_path.ok_or("observe requires --ledger <path>")?;
    let mut ledger = serde_json::from_str(&fs::read_to_string(path)?)?;
    let obs = fetch(&url)?;
    let warnings = obs.warnings(&ledger)?;
    let diff = k8s_bridge::fold_observation(&mut ledger, &obs)?;
    for warning in warnings {
        eprintln!("{warning}");
    }
    for change in diff {
        println!("{}: {} → {}", change.field, change.old, change.new);
    }
    if !dry_run {
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&ledger)?),
        )?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    #[test]
    fn dry_run_preserves_file_and_write_promotes_with_explicit_url() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.json");
        let original = include_str!("../../../genesis/data/rakia/compute-capacity.json");
        fs::write(&path, original).unwrap();
        for dry in [true, false] {
            let mut args = vec![
                "--ledger".into(),
                path.to_str().unwrap().into(),
                "--prometheus-url".into(),
                "https://chosen.example/proxy".into(),
            ];
            if dry {
                args.push("--dry-run".into());
            }
            let result = observe_with(&args, |url| {
                assert_eq!(url, "https://chosen.example/proxy");
                let mut obs = k8s_bridge::Observation {
                    cpu_m: Default::default(),
                    memory_bytes: Default::default(),
                    ready: Default::default(),
                    timestamp: "2026-09-06T00:00:00Z".into(),
                    host: "chosen.example".into(),
                };
                let ledger: Value = serde_json::from_str(original).unwrap();
                for group in ledger["cluster"]["nodeTypes"].as_object().unwrap().values() {
                    for node in group["nodes"].as_array().unwrap() {
                        let name = node["name"].as_str().unwrap();
                        obs.cpu_m
                            .insert(name.into(), node["allocatable"]["cpu_m"].as_u64().unwrap());
                        obs.memory_bytes.insert(
                            name.into(),
                            node["allocatable"]["memory_Mi"].as_u64().unwrap() * 1024 * 1024,
                        );
                        obs.ready.insert(name.into(), true);
                    }
                }
                obs.cpu_m.insert("extra-node".into(), 1000);
                Ok(obs)
            })
            .unwrap();
            assert_eq!(result, 0);
            let written = fs::read_to_string(&path).unwrap();
            if dry {
                assert_eq!(written, original);
            } else {
                let ledger: Value = serde_json::from_str(&written).unwrap();
                assert_eq!(ledger["snapshotTimestamp"], json!("2026-09-06T00:00:00Z"));
                assert!(!written.contains("extra-node"));
                assert!(!written.contains("https://chosen.example"));
            }
        }
    }
}
