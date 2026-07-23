use std::{env, path::PathBuf, process::ExitCode};

use elohim_epr_cli::{
    check, doctor,
    error::{Error, Result},
    explain, flow, git,
    git::{ChangeKind, ChangedPath},
    ready,
    report::{FindingStatus, Report},
    setup,
};

fn main() -> ExitCode {
    // `flow` is its own command family (value-chain projection + walk); it renders its own
    // output rather than a Report, so it is dispatched before the Report-shaped commands.
    let raw: Vec<String> = env::args().skip(1).collect();
    if raw.first().map(String::as_str) == Some("flow") {
        return match flow::run(&raw[1..]) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("epr flow: {error}");
                ExitCode::from(2)
            }
        };
    }

    match run() {
        Ok((report, exit_on_not_ready)) => {
            let json = env::args().any(|arg| arg == "--json");
            if json {
                match serde_json::to_string_pretty(&report) {
                    Ok(value) => println!("{value}"),
                    Err(error) => {
                        eprintln!("epr: cannot serialize report: {error}");
                        return ExitCode::from(2);
                    }
                }
            } else {
                print_human(&report);
            }
            if exit_on_not_ready && !report.ready {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("epr: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(Report, bool)> {
    let (repo, args) = global_args(env::args().skip(1).collect())?;
    let start = repo.unwrap_or(env::current_dir().map_err(|source| Error::Read {
        path: PathBuf::from("."),
        source,
    })?);
    let root = git::discover_root(&start)?;
    let Some(command) = args.first().map(String::as_str) else {
        return Err(Error::InvalidArguments(usage().into()));
    };

    match command {
        "setup" if args.len() == 1 => Ok((setup::run(&root)?, true)),
        "doctor" if args.len() == 1 => Ok((doctor::run(&root)?, true)),
        "explain" if args.len() == 2 => {
            let path = PathBuf::from(&args[1]);
            let path = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            Ok((explain::run(&root, &path)?, false))
        }
        "check" => {
            let changes = if args.len() == 1 {
                git::working_tree_changes(&root)?
            } else {
                args[1..]
                    .iter()
                    .map(|path| ChangedPath {
                        path: path.clone(),
                        kind: if root.join(path).exists() {
                            ChangeKind::Modified
                        } else {
                            ChangeKind::Deleted
                        },
                    })
                    .collect()
            };
            Ok((check::run(&root, &changes)?.report, false))
        }
        "ready" => {
            let (target, deep) = ready_args(&args[1..])?;
            let target = target.unwrap_or_else(|| git::default_target(&root));
            let changes = git::changes_against(&root, &target)?;
            Ok((ready::run(&root, &target, &changes, deep)?, true))
        }
        _ => Err(Error::InvalidArguments(usage().into())),
    }
}

fn global_args(args: Vec<String>) -> Result<(Option<PathBuf>, Vec<String>)> {
    let mut repo = None;
    let mut retained = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => index += 1,
            "--repo" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(Error::InvalidArguments("--repo needs a path".into()));
                };
                repo = Some(PathBuf::from(path));
                index += 2;
            }
            value => {
                retained.push(value.to_string());
                index += 1;
            }
        }
    }
    Ok((repo, retained))
}

fn ready_args(args: &[String]) -> Result<(Option<String>, bool)> {
    let mut target = None;
    let mut deep = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" => {
                target = Some(
                    args.get(index + 1)
                        .ok_or_else(|| Error::InvalidArguments("--target needs a ref".into()))?
                        .clone(),
                );
                index += 2;
            }
            "--deep" => {
                deep = true;
                index += 1;
            }
            other => {
                return Err(Error::InvalidArguments(format!(
                    "unknown ready argument `{other}`"
                )))
            }
        }
    }
    Ok((target, deep))
}

fn print_human(report: &Report) {
    println!(
        "EPR {} — {} [{}]",
        report.command, report.repository, report.scope
    );
    println!("  {}", report.local_agency);
    for finding in &report.findings {
        let marker = match finding.status {
            FindingStatus::Pass => "✓",
            FindingStatus::Info => "·",
            FindingStatus::Warn => "!",
            FindingStatus::Refer => "→",
            FindingStatus::Fail => "✗",
        };
        let reach = if finding.blocks_reach { " [reach]" } else { "" };
        println!("{marker} {}{reach}: {}", finding.code, finding.summary);
        if let Some(path) = &finding.path {
            println!("    path: {path}");
        }
        if let Some(remediation) = &finding.remediation {
            println!("    fix: {remediation}");
        }
    }
    let outcome = match report.command.as_str() {
        "explain" => "EXPLAINED: authority and directory governance resolved for this path",
        "check" if !report.ready => {
            "LOCAL: advisory findings exist; your working tree remains yours"
        }
        _ if report.ready => "READY: governance floor is coherent for requested reach",
        _ => "NOT READY: resolve reach-blocking findings before push",
    };
    println!("{outcome}");
    if report.command == "explain" && !report.data.is_null() {
        if let Ok(value) = serde_json::to_string_pretty(&report.data) {
            println!("{value}");
        }
    }
}

fn usage() -> &'static str {
    "usage: epr [--repo PATH] [--json] <setup|doctor|explain PATH|check [PATH...]|ready [--target REF] [--deep]>"
}
