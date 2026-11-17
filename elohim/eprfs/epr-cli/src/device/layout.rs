//! `epr device layout show|save`: the device layout every device-side command resolves, so
//! callers that are not Rust (berth, the pre-push scanner) read the same resolver.
use elohim_device_runtime::{
    bin_store::Refusal,
    layout::{self, Field},
};
use std::process::ExitCode;

const USAGE: &str = "usage: epr device layout show [--json] | epr device layout save [--field \
                     binStore|stateDir|cargoPool|projectRoot]...";

pub(super) fn run(args: &[String]) -> Result<ExitCode, String> {
    match args.first().map(String::as_str) {
        Some("show") => show(&args[1..]),
        Some("save") => save(&args[1..]),
        Some("--help" | "-h" | "help") | None => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Some(other) => Err(format!("unknown layout verb `{other}`\n{USAGE}")),
    }
}

fn refusal(error: Refusal) -> String {
    format!("REFUSED {}: {}", error.kind, error.detail)
}

fn show(args: &[String]) -> Result<ExitCode, String> {
    let json = match args {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => return Err(USAGE.into()),
    };
    let resolved = layout::resolve().map_err(refusal)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&resolved).map_err(|e| e.to_string())?
        );
    } else {
        for line in resolved.lines() {
            println!("{line}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn save(args: &[String]) -> Result<ExitCode, String> {
    let mut fields = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--field" => {
                let name = rest.next().ok_or("--field needs a value")?;
                let field = Field::ALL
                    .into_iter()
                    .find(|field| field.name() == name)
                    .ok_or_else(|| format!("unknown field `{name}`\n{USAGE}"))?;
                fields.push(field);
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
    }
    let resolved = layout::resolve().map_err(refusal)?;
    let saved = layout::save(&resolved, &fields).map_err(refusal)?;
    let config = resolved.config_dir().map_err(refusal)?;
    println!(
        "saved {}: {}",
        config.join(layout::SAVED_FILE).display(),
        serde_json::to_string(&saved).map_err(|e| e.to_string())?
    );
    Ok(ExitCode::SUCCESS)
}
