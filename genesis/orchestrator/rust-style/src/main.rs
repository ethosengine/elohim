use elohim_rust_style::{discipline, repository};
use std::{io::Read, path::PathBuf};

fn run() -> Result<i32, String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("epr-rust-style --root PATH [--base REF] [--head REF] [--json] [--path FILE --stdin]\nepr-rust-style --export-discipline --guide FILE --out DIR\nAll cfg branches; baseline derives from Git. Export offers content, not adoption. Exit 1 = violations, 2 = unmeasured.");
        return Ok(0);
    }
    let mut root = PathBuf::from(".");
    let mut base = "origin/dev".to_string();
    let mut head = None;
    let mut path = None;
    let mut json = false;
    let mut stdin = false;
    let mut scope = None;
    let mut export_discipline = false;
    let mut guide = None;
    let mut out = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--export-discipline" => export_discipline = true,
            "--guide" => guide = Some(PathBuf::from(iter.next().ok_or("--guide requires a path")?)),
            "--out" => out = Some(PathBuf::from(iter.next().ok_or("--out requires a path")?)),
            "--root" => root = PathBuf::from(iter.next().ok_or("--root requires a path")?),
            "--base" => base = iter.next().ok_or("--base requires a ref")?.clone(),
            "--head" => head = Some(iter.next().ok_or("--head requires a ref")?.clone()),
            "--path" => path = Some(iter.next().ok_or("--path requires a path")?.clone()),
            "--scope" => scope = Some(iter.next().ok_or("--scope requires a path")?.clone()),
            "--json" => json = true,
            "--stdin" => stdin = true,
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    if export_discipline {
        if head.is_some() || path.is_some() || scope.is_some() || stdin {
            return Err("discipline export cannot evaluate source".into());
        }
        let closure = discipline::export(
            &guide.ok_or("--guide required")?,
            &out.ok_or("--out required")?,
        )?;
        println!(
            "{}",
            serde_json::to_string_pretty(&closure).map_err(|e| e.to_string())?
        );
        return Ok(0);
    }
    if guide.is_some() || out.is_some() {
        return Err("--guide/--out require --export-discipline".into());
    }
    if head.is_some() && (path.is_some() || stdin) {
        return Err("--head cannot use working-tree prospective stdin mode".into());
    }
    if stdin && path.is_none() {
        return Err("--stdin requires --path".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if let Some(path) = path {
        if !stdin {
            return Err("--path requires --stdin".into());
        }
        if PathBuf::from(&path).is_absolute()
            || PathBuf::from(&path)
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("path must stay inside repository".into());
        }
        let mut source = String::new();
        std::io::stdin()
            .take(32 * 1024 * 1024 + 1)
            .read_to_string(&mut source)
            .map_err(|e| e.to_string())?;
        if source.len() > 32 * 1024 * 1024 {
            return Err("source exceeds limit".into());
        }
        let failures = repository::prospective(&root, &path, &source)?;
        println!(
            "{}",
            serde_json::json!({"version":1,"failures":failures,"rule":"RS-GOV-04","paydown":"checked at reach boundary"})
        );
        return Ok(i32::from(!failures.is_empty()));
    }
    let base = repository::merge_base(&root, &base, head.as_deref().unwrap_or("HEAD"))?;
    let report = repository::evaluate(&root, &base, head.as_deref(), scope.as_deref())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "[rust-style] RS-GOV-04: {} files, {} crates; baseline {}; {}",
            report.scanned_files,
            report.crates.len(),
            report.base,
            if report.enrollment {
                "enrollment: no-new enforced; paydown starts after policy lands"
            } else {
                "paydown active"
            }
        );
        for (name, c) in &report.crates {
            if c.touched || c.strict_files > 0 {
                println!(
                    "[rust-style] {name}: debt {} -> {}, strict files {}{}",
                    c.before,
                    c.after,
                    c.strict_files,
                    if c.touched { ", touched" } else { "" }
                );
            }
        }
        for failure in &report.failures {
            eprintln!("[rust-style] REFUSED {failure}");
        }
        println!(
            "[rust-style] {}",
            if report.failures.is_empty() {
                "PASS"
            } else {
                "FAIL"
            }
        );
    }
    Ok(i32::from(!report.failures.is_empty()))
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("[rust-style] UNMEASURED: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
