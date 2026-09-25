//! `jenkins-bridge` — the operator's CLI over Jenkins' own archive. Exit codes: 0 done,
//! 3 governance refusal (offer not minted/approved, recipe not projected), 64 usage,
//! 65 malformed input, 74 sidecar I/O. Every refusal writes nothing.

use std::path::PathBuf;
use std::process::ExitCode;

use jenkins_bridge::card::{self, LastObserved};
use jenkins_bridge::observe::{DEFAULT_OFFER, DEFAULT_RECIPES};
use jenkins_bridge::translate::DEFAULT_JENKINS;
use jenkins_bridge::{mint_offer, observe, BridgeError, BuildInputs, Context};

const USAGE: &str = "usage:
  jenkins-bridge observe --stages <wfapi.json> [--graph <actual-build-graph.json>] [common]
  jenkins-bridge drift   --stages <wfapi.json> [--graph <actual-build-graph.json>] [common]
  jenkins-bridge card    [--stages <wfapi.json> [--graph <graph.json>]] [--out <elohim-governance.json>] [common]
  jenkins-bridge offer   status|mint [common]
common: [--root DIR] [--offer PATH] [--recipes PATH] [--jenkins-url URL] [--json]";

#[derive(Default)]
struct Args {
    root: Option<PathBuf>,
    offer: Option<String>,
    recipes: Option<String>,
    stages: Option<PathBuf>,
    graph: Option<PathBuf>,
    out: Option<PathBuf>,
    jenkins: Option<String>,
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
            "--offer" => a.offer = Some(value()?),
            "--recipes" => a.recipes = Some(value()?),
            "--stages" => a.stages = Some(value()?.into()),
            "--graph" => a.graph = Some(value()?.into()),
            "--out" => a.out = Some(value()?.into()),
            "--jenkins-url" => a.jenkins = Some(value()?),
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

fn read(path: &PathBuf) -> Result<String, BridgeError> {
    std::fs::read_to_string(path)
        .map_err(|e| BridgeError::Malformed(format!("cannot read {}: {e}", path.display())))
}

fn inputs(a: &Args) -> Result<Option<BuildInputs>, BridgeError> {
    let Some(stages) = &a.stages else {
        return Ok(None);
    };
    Ok(Some(BuildInputs {
        stages: read(stages)?,
        graph: a.graph.as_ref().map(read).transpose()?,
        jenkins_base: a.jenkins.clone().unwrap_or_else(|| DEFAULT_JENKINS.into()),
    }))
}

fn context(a: &Args) -> Result<Context, BridgeError> {
    let root = a.root.clone().unwrap_or_else(|| PathBuf::from("."));
    Context::load(
        &root,
        a.offer.as_deref().unwrap_or(DEFAULT_OFFER),
        a.recipes.as_deref().unwrap_or(DEFAULT_RECIPES),
    )
}

fn need_inputs(a: &Args) -> Result<BuildInputs, BridgeError> {
    inputs(a)?.ok_or_else(|| {
        BridgeError::Malformed(format!("--stages <wfapi.json> is required\n{USAGE}"))
    })
}

fn print(json: bool, value: &serde_json::Value, text: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    } else {
        text();
    }
}

fn run(args: &[String]) -> Result<u8, BridgeError> {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("{USAGE}");
        return Ok(64);
    };
    match command {
        "observe" => {
            let a = parse(&args[1..])?;
            let ctx = context(&a)?;
            let outcome = observe(&ctx, &need_inputs(&a)?)?;
            let value = serde_json::to_value(&outcome).unwrap_or_default();
            print(a.json, &value, || {
                println!(
                    "jenkins-bridge observe — elohim-edge #{} ({})",
                    outcome.build, outcome.basis
                );
                println!("  {}", outcome.stakes);
                println!("  process {} (spec {})", outcome.process, outcome.spec);
                println!(
                    "  {} appended, {} already present",
                    outcome.appended, outcome.present
                );
                println!("  verify: epr flow walk {}", outcome.process);
            });
            Ok(0)
        }
        "drift" => {
            let a = parse(&args[1..])?;
            let ctx = context(&a)?;
            let translation = ctx.translate(&need_inputs(&a)?)?;
            let standing = ctx.require_active()?;
            let report = ctx.drift_of(&translation);
            let value = serde_json::json!({
                "build": translation.build, "basis": translation.basis,
                "stakes": standing.stakes_line(), "clean": report.is_clean(),
                "findings": report.findings,
            });
            print(a.json, &value, || {
                println!(
                    "jenkins-bridge drift — elohim-edge #{} vs {}@{} ({})",
                    translation.build, ctx.recipe.id, ctx.recipe.version, translation.basis
                );
                println!("  {}", standing.stakes_line());
                if report.is_clean() {
                    println!("  clean — no recipe drift, no undisclosed capability");
                }
                for f in &report.findings {
                    println!("  {}", f.line());
                }
            });
            Ok(0)
        }
        "card" => {
            let a = parse(&args[1..])?;
            let ctx = context(&a)?;
            let translation = inputs(&a)?.map(|i| ctx.translate(&i)).transpose()?;
            let drift = translation.as_ref().map(|t| ctx.drift_of(t));
            let stewards = elohim_epr_cli::flow::memory::execute(
                &ctx.root,
                "collective",
                Some(&ctx.offer.declaration().collective),
                None,
            )
            .map(|v| v["stewards"].clone())
            .map_err(|e| BridgeError::Governance(e.to_string()))?;
            let last = translation
                .as_ref()
                .zip(drift.as_ref())
                .map(|(t, d)| LastObserved {
                    translation: t,
                    drift: d,
                });
            let value = card::render(&ctx, stewards, last)?;
            let text = format!(
                "{}\n",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
            match &a.out {
                Some(out) => std::fs::write(out, &text)
                    .map_err(|e| BridgeError::Io(format!("cannot write {}: {e}", out.display())))?,
                None => print!("{text}"),
            }
            Ok(0)
        }
        "offer" => {
            let sub = args.get(1).map(String::as_str);
            let a = parse(args.get(2..).unwrap_or(&[]))?;
            let ctx = context(&a)?;
            match sub {
                Some("mint") => {
                    let (cid, new) = mint_offer(&ctx)?;
                    println!(
                        "offer {} {} — proposed; it activates only on a distinct Steward's verdict",
                        cid,
                        if new { "minted" } else { "already minted" }
                    );
                }
                Some("status") => {}
                _ => {
                    eprintln!("{USAGE}");
                    return Ok(64);
                }
            }
            let standing = ctx.standing()?;
            let value = serde_json::to_value(&standing).unwrap_or_default();
            print(a.json, &value, || {
                println!("{}", standing.line());
                if let jenkins_bridge::Standing::Minted(s) = &standing {
                    if let Some(m) = &s.missing {
                        println!("  missing: {m}");
                    }
                    for i in &s.ignored {
                        println!("  ignored verdict: {i}");
                    }
                }
            });
            Ok(0)
        }
        "--help" | "-h" => {
            println!("{USAGE}");
            Ok(0)
        }
        _ => {
            eprintln!("{USAGE}");
            Ok(64)
        }
    }
}

fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
