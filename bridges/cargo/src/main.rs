//! `cargo-bridge` — the operator's CLI over one Rust workspace. Exit codes: 0 done (including a
//! stale `verify`, which prints a rung-one notice), 1 a stale `verify` under `RELIANCE=strict`,
//! 64 usage, 65 malformed input, 74 sidecar I/O. Every refusal writes nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use cargo_bridge::{observe, reliance, verify, BridgeError, Config, Inputs};

const USAGE: &str = "usage:
  cargo-bridge observe [input] [common]   append the reliance records (idempotent)
  cargo-bridge verify  [input] [common]   compare with the store; notice when stale (RELIANCE=strict refuses)
  cargo-bridge offer   [input] [common]   write the workspace's offer document under --root
input:  --manifest-path <Cargo.toml> (runs `cargo metadata --format-version 1 --locked`)
      | --metadata <metadata.json> [--lockfile <Cargo.lock>]
common: [--root DIR] [--store DIR] [--workspace NAME] [--occurred-at RFC3339] [--author PARTICIPANT] [--json]";

#[derive(Default)]
struct Args {
    root: Option<PathBuf>,
    store: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    metadata: Option<PathBuf>,
    lockfile: Option<PathBuf>,
    workspace: Option<String>,
    occurred_at: Option<String>,
    author: Option<String>,
    json: bool,
}

fn parse(rest: &[String]) -> Result<Args, BridgeError> {
    let mut a = Args::default();
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| BridgeError::Malformed(format!("{flag} needs a value")))
        };
        match flag.as_str() {
            "--root" => a.root = Some(value()?.into()),
            "--store" => a.store = Some(value()?.into()),
            "--manifest-path" => a.manifest_path = Some(value()?.into()),
            "--metadata" => a.metadata = Some(value()?.into()),
            "--lockfile" => a.lockfile = Some(value()?.into()),
            "--workspace" => a.workspace = Some(value()?),
            "--occurred-at" => a.occurred_at = Some(value()?),
            "--author" => a.author = Some(value()?),
            "--json" => a.json = true,
            other => {
                return Err(BridgeError::Malformed(format!(
                    "unknown argument `{other}`\n{USAGE}"
                )))
            }
        }
    }
    Ok(a)
}

fn absolute(path: &Path) -> Result<PathBuf, BridgeError> {
    std::fs::canonicalize(path)
        .map_err(|e| BridgeError::Malformed(format!("cannot resolve {}: {e}", path.display())))
}

fn metadata_text(a: &Args) -> Result<String, BridgeError> {
    match (&a.metadata, &a.manifest_path) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| BridgeError::Malformed(format!("cannot read {}: {e}", path.display()))),
        (None, Some(manifest)) => {
            let out = Command::new("cargo")
                .args([
                    "metadata",
                    "--format-version",
                    "1",
                    "--locked",
                    "--manifest-path",
                ])
                .arg(manifest)
                .output()
                .map_err(|e| BridgeError::Malformed(format!("cannot run cargo metadata: {e}")))?;
            if !out.status.success() {
                return Err(BridgeError::Malformed(format!(
                    "cargo metadata failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )));
            }
            String::from_utf8(out.stdout)
                .map_err(|e| BridgeError::Malformed(format!("cargo metadata is not UTF-8: {e}")))
        }
        (None, None) => Err(BridgeError::Malformed(format!(
            "give --manifest-path or --metadata\n{USAGE}"
        ))),
    }
}

/// The lockfile's last commit time: unchanged lockfile, unchanged records.
fn lockfile_commit_time(lockfile: &Path) -> Result<String, BridgeError> {
    let dir = lockfile.parent().unwrap_or(Path::new("."));
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["log", "-1", "--format=%cI", "--"])
        .arg(lockfile)
        .output()
        .map_err(|e| BridgeError::Malformed(format!("cannot read the lockfile's history: {e}")))?;
    let time = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || time.is_empty() {
        return Err(BridgeError::Malformed(format!(
            "{} has no commit to date the observation by — pass --occurred-at",
            lockfile.display()
        )));
    }
    Ok(time)
}

fn run(command: &str, a: &Args) -> Result<u8, BridgeError> {
    let root = absolute(a.root.as_deref().unwrap_or(Path::new(".")))?;
    // Where the records go (default: the repository root's own `.eprfs/status`). Paths in the
    // records are always relative to `--root`, so a scratch store holds the same records.
    let store = match &a.store {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| BridgeError::Io(e.to_string()))?;
            absolute(dir)?
        }
        None => root.clone(),
    };
    let metadata = metadata_text(a)?;
    let parsed = cargo_bridge::metadata::parse_metadata(&metadata)?;
    let lockfile = match &a.lockfile {
        Some(path) => path.clone(),
        None => Path::new(&parsed.workspace_root).join("Cargo.lock"),
    };
    let lock_bytes = std::fs::read(&lockfile)
        .map_err(|e| BridgeError::Malformed(format!("cannot read {}: {e}", lockfile.display())))?;
    let workspace = a.workspace.clone().unwrap_or_else(|| {
        Path::new(&parsed.workspace_root)
            .file_name()
            .map_or_else(|| "workspace".into(), |n| n.to_string_lossy().into_owned())
    });
    let occurred_at = match &a.occurred_at {
        Some(t) => t.clone(),
        None => lockfile_commit_time(&lockfile)?,
    };
    let mut config = Config::for_repository(&workspace, &root.to_string_lossy(), &occurred_at);
    if let Some(author) = &a.author {
        config.author = author.clone();
    }
    let inputs = Inputs {
        metadata,
        lockfile: lock_bytes,
    };
    match command {
        "observe" => {
            let outcome = observe(&store, &inputs, &config)?;
            if a.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).unwrap_or_default()
                );
            } else {
                let r = &outcome.report;
                println!(
                    "observed workspace `{}` — process {}",
                    r.workspace, outcome.process
                );
                println!(
                    "  offer {} (proposed until a distinct Steward approves)",
                    outcome.offer
                );
                println!(
                    "  {} resolved, {} members, {} cited: {} registry with checksum, {} registry \
                     without, {} git, {} path (coordinate-only)",
                    r.packages_resolved,
                    r.workspace_members,
                    r.cited,
                    r.registry_with_checksum,
                    r.registry_without_checksum,
                    r.git,
                    r.path
                );
                println!(
                    "  {} presences held in trust, {} direct edges",
                    r.presences, r.edges
                );
                println!(
                    "  {} appended, {} already present",
                    outcome.appended, outcome.present
                );
            }
            Ok(0)
        }
        "verify" => {
            let outcome = verify(&store, &inputs, &config)?;
            if a.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).unwrap_or_default()
                );
            } else {
                println!("{}", outcome.notice());
            }
            let strict = std::env::var("RELIANCE").is_ok_and(|v| v == "strict");
            Ok(if outcome.is_fresh() || !strict { 0 } else { 1 })
        }
        "offer" => {
            let reliance = reliance(&inputs, &config)?;
            let path = cargo_bridge::observe::write_offer(&store, &reliance)?;
            println!("wrote {} — offer {}", path.display(), reliance.offer_cid);
            Ok(0)
        }
        other => Err(BridgeError::Malformed(format!(
            "unknown command `{other}`\n{USAGE}"
        ))),
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = argv.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(64);
    };
    let result = parse(rest).and_then(|a| run(command, &a));
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("cargo-bridge: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
