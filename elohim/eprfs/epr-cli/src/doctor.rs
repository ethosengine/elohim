use std::{fs, path::Path};

use eprfs_host::HostProfile;
use serde_json::json;

use crate::{
    authority::{AuthorityIndex, PACKAGE_ROOT},
    error::Result,
    process,
    report::{Finding, FindingStatus, Report},
};

pub fn run(root: &Path) -> Result<Report> {
    let mut report = Report::new("doctor", root);

    required_path(
        &mut report,
        root,
        "governance.agents-root",
        "AGENTS.md",
        "Codex root governance entrypoint is present",
    );
    required_path(
        &mut report,
        root,
        "governance.claude-root",
        "CLAUDE.md",
        "Claude root governance entrypoint is present",
    );
    required_path(
        &mut report,
        root,
        "governance.root-manifest",
        ".epr-meta/manifest.md",
        "constitutional .epr-meta root is present",
    );
    required_path(
        &mut report,
        root,
        "governance.package-root",
        PACKAGE_ROOT,
        "Elohim package authority tree is present",
    );

    let index = AuthorityIndex::load(root)?;
    if index.package_count() > 0 {
        report.push(Finding::new(
            "governance.packages-indexed",
            FindingStatus::Pass,
            format!("{} capability packages indexed", index.package_count()),
        ));
    }
    for diagnostic in index.diagnostics() {
        report.push(
            Finding::new(
                "governance.package-path-collision",
                FindingStatus::Fail,
                diagnostic,
            )
            .blocking(),
        );
    }

    tool(&mut report, root, "git", true);
    tool(&mut report, root, "node", false);
    tool(&mut report, root, "pnpm", false);
    tool(&mut report, root, "python3", false);
    tool(&mut report, root, "cargo", false);

    check_hooks(&mut report, root);
    check_runtime_abis(&mut report, root);

    let host = HostProfile::current_platform();
    report.data = json!({
        "hostProfile": host,
        "packageCount": index.package_count(),
        "nativeEngine": {
            "metaResolution": true,
            "policyExpansion": true,
            "ruleEvaluation": true,
            "packageFidelity": "compatibility-adapter"
        }
    });
    report.recompute();
    Ok(report)
}

fn required_path(report: &mut Report, root: &Path, code: &str, relative: &str, summary: &str) {
    let path = root.join(relative);
    if path.exists() {
        report.push(Finding::new(code, FindingStatus::Pass, summary).path(relative));
    } else {
        report.push(
            Finding::new(code, FindingStatus::Fail, format!("missing {relative}"))
                .remediation(format!(
                    "restore `{relative}` from the repository projection"
                ))
                .path(relative)
                .blocking(),
        );
    }
}

fn tool(report: &mut Report, root: &Path, program: &str, required: bool) {
    match process::version(program, root) {
        Some(version) => report.push(Finding::new(
            format!("tool.{program}"),
            FindingStatus::Pass,
            format!("{program} available ({version})"),
        )),
        None if required => report.push(
            Finding::new(
                format!("tool.{program}"),
                FindingStatus::Fail,
                format!("required tool `{program}` is unavailable"),
            )
            .blocking(),
        ),
        None => report.push(
            Finding::new(
                format!("tool.{program}"),
                FindingStatus::Warn,
                format!("optional compatibility tool `{program}` is unavailable"),
            )
            .detail("The Rust governance kernel still works; a legacy gate may be unavailable until its native port is complete."),
        ),
    }
}

fn check_hooks(report: &mut Report, root: &Path) {
    let configured = process::run("git", &["config", "--get", "core.hooksPath"], root)
        .ok()
        .filter(|output| output.success)
        .map(|output| output.stdout);
    let Some(configured) = configured.filter(|value| !value.is_empty()) else {
        report.push(
            Finding::new(
                "git.hooks-path",
                FindingStatus::Fail,
                "Git hooks are not activated in this worktree",
            )
            .remediation("run `pnpm install` (or `pnpm exec husky`) in this worktree")
            .blocking(),
        );
        return;
    };

    let hook = root.join(&configured).join("pre-push");
    if hook.is_file() && is_executable(&hook) {
        report.push(Finding::new(
            "git.hooks-path",
            FindingStatus::Pass,
            format!("reach rehearsal hook active at {configured}/pre-push"),
        ));
    } else {
        report.push(
            Finding::new(
                "git.pre-push",
                FindingStatus::Fail,
                format!("configured hooks path `{configured}` has no executable pre-push hook"),
            )
            .remediation("activate Husky in this worktree with `pnpm install`")
            .path(hook)
            .blocking(),
        );
    }
}

fn check_runtime_abis(report: &mut Report, root: &Path) {
    if root.join(".claude/settings.json").is_file() {
        report.push(Finding::new(
            "runtime.claude-hooks",
            FindingStatus::Pass,
            "Claude project hooks are projected",
        ));
    } else {
        report.push(Finding::new(
            "runtime.claude-hooks",
            FindingStatus::Warn,
            "Claude project hook projection is missing",
        ));
    }

    if root.join(".agents/skills").is_dir() {
        report.push(Finding::new(
            "runtime.codex-skills",
            FindingStatus::Pass,
            "stock Codex repository skill root is projected",
        ));
    } else {
        report.push(
            Finding::new(
                "runtime.codex-skills",
                FindingStatus::Warn,
                "stock Codex expects repository skills under `.agents/skills`",
            )
            .remediation("project package-authoritative skills to the current Codex runtime ABI"),
        );
    }

    let toml_agents = fs::read_dir(root.join(".codex/agents"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(std::result::Result::ok)
        .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml"));
    if toml_agents {
        report.push(Finding::new(
            "runtime.codex-agents",
            FindingStatus::Pass,
            "stock Codex TOML agents are projected",
        ));
    } else {
        report.push(
            Finding::new(
                "runtime.codex-agents",
                FindingStatus::Warn,
                "no stock Codex `.codex/agents/*.toml` projections found",
            )
            .remediation(
                "add a current Codex agent projection renderer; keep package JSON authoritative",
            ),
        );
    }

    let inline_codex_hooks = fs::read_to_string(root.join(".codex/config.toml"))
        .ok()
        .is_some_and(|config| {
            config
                .lines()
                .map(str::trim)
                .any(|line| matches!(line, "[hooks]") || line.starts_with("[[hooks."))
        });
    if root.join(".codex/hooks.json").is_file() || inline_codex_hooks {
        report.push(Finding::new(
            "runtime.codex-hooks",
            FindingStatus::Pass,
            "Codex project lifecycle hooks are configured (runtime trust remains local)",
        ));
    } else {
        report.push(
            Finding::new(
                "runtime.codex-hooks",
                FindingStatus::Warn,
                "Codex project hooks are not projected",
            )
            .remediation("project the shared governance hooks to `.codex/hooks.json`"),
        );
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
