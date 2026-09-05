use ark_core::manifest::RuntimeManifest;
use k8s_bridge::{render_envelope, verify, DriftVerdict, Result};
use std::{env, fs, path::Path, process::ExitCode};

fn run(args: &[String]) -> Result<u8> {
    match args.first().map(String::as_str) {
        Some("observe") if args.len() == 1 => {
            println!("not yet: Station 3b");
            Ok(64)
        }
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
            eprintln!("usage: k8s-bridge render <manifest.json> [--human <name>] | verify --deployments <path> --manifests-dir <dir> | observe");
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
