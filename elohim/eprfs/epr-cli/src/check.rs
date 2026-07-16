use std::{fs, path::Path};

use eprfs_core::GovernanceRuleClass;
use serde_json::json;

use crate::{
    authority::{normalize, AuthorityIndex},
    error::Result,
    git::{self, ChangeKind, ChangedPath},
    report::{Finding, FindingStatus, Report},
    repository_validators::ElohimRepositoryValidators,
};

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub report: Report,
    pub needs_package_verify: bool,
    pub touches_eprfs: bool,
}

/// Evaluate local changes. Findings are advisory in this command; the
/// `blocks_reach` bit is evidence consumed later by `ready`.
pub fn run(root: &Path, changes: &[ChangedPath]) -> Result<CheckOutcome> {
    run_with_revisions(root, changes, None)
}

/// Evaluate a committed reach candidate exactly as it exists at `head`, using
/// `base` for prior-content and new-subtree comparisons. Uncommitted authoring
/// elsewhere in the shared working tree is deliberately outside this view.
pub fn run_at(
    root: &Path,
    changes: &[ChangedPath],
    head: &str,
    base: &str,
) -> Result<CheckOutcome> {
    run_with_revisions(root, changes, Some((head, base)))
}

fn run_with_revisions(
    root: &Path,
    changes: &[ChangedPath],
    revisions: Option<(&str, &str)>,
) -> Result<CheckOutcome> {
    let authority = AuthorityIndex::load(root)?;
    let mut report = Report::new("check", root);
    let mut needs_package_verify = false;
    let mut touches_eprfs = false;

    for diagnostic in authority.diagnostics() {
        report.push(
            Finding::new("authority.path-collision", FindingStatus::Fail, diagnostic).blocking(),
        );
    }

    for change in changes {
        let normalized = normalize(Path::new(&change.path));
        touches_eprfs |= normalized.starts_with("elohim/eprfs/");
        needs_package_verify |= AuthorityIndex::is_agentic_path(&normalized);

        if change.kind == ChangeKind::Conflicted {
            report.push(
                Finding::new(
                    "git.conflict",
                    FindingStatus::Warn,
                    "unresolved Git conflict cannot earn reach",
                )
                .path(&normalized)
                .blocking(),
            );
            continue;
        }

        if let Some(package) = authority.find(Path::new(&normalized)) {
            if package.is_package_master_projection(&normalized) {
                report.push(
                    Finding::new(
                        "authority.projection-edited",
                        FindingStatus::Warn,
                        format!(
                            "package-master projection changed; authority is `{}`",
                            package.package_path
                        ),
                    )
                    .remediation(format!(
                        "move the edit to `{}` and regenerate only {}:{}",
                        package.package_path, package.package_kind, package.package_id
                    ))
                    .path(&normalized)
                    .blocking(),
                );
            }
        }

        if change.is_deleted() {
            continue;
        }
        let target = root.join(&normalized);
        let content = match revisions {
            Some((head, _)) => git::content_at(root, head, &normalized),
            None => fs::read(&target)
                .ok()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        };
        let prior_content = match revisions {
            Some((_, base)) => git::content_at(root, base, &normalized),
            None => git::prior_content(root, &normalized),
        };
        let is_new_subdir = match revisions {
            Some((_, base)) => change.is_new() && git::parent_is_new_at(root, base, &normalized),
            None => change.is_new() && git::parent_is_new(root, &normalized),
        };
        let write = eprfs_meta::GovernanceWrite {
            path: normalized.clone(),
            content,
            prior_content,
            is_new: change.is_new(),
            is_new_subdir,
        };

        match eprfs_meta::evaluate_path_with(root, &target, &write, &ElohimRepositoryValidators) {
            Ok(evaluation) => {
                for diagnostic in evaluation.diagnostics {
                    report.push(
                        Finding::new(
                            format!("governance.{}", diagnostic.code),
                            FindingStatus::Warn,
                            diagnostic.message,
                        )
                        .path(&normalized)
                        .blocking(),
                    );
                }
                for verdict in evaluation.verdicts {
                    let (status, blocks_reach) = match verdict.class {
                        GovernanceRuleClass::Deny | GovernanceRuleClass::Ask => {
                            (FindingStatus::Warn, true)
                        }
                        GovernanceRuleClass::Inject => (FindingStatus::Info, false),
                        GovernanceRuleClass::Measure | GovernanceRuleClass::Dispatch => {
                            (FindingStatus::Info, false)
                        }
                    };
                    let mut finding = Finding::new(
                        format!("governance.rule.{}", verdict.rule_id),
                        status,
                        verdict.reason,
                    )
                    .path(&normalized);
                    if blocks_reach {
                        finding = finding.blocking();
                    }
                    report.push(finding);
                }
            }
            Err(error) => report.push(
                Finding::new(
                    "governance.meta-invalid",
                    FindingStatus::Warn,
                    format!("native `.epr-meta` evaluation failed: {error}"),
                )
                .remediation("repair the nearest `.epr-meta`; malformed governance may advise but cannot bind")
                .path(&normalized)
                .blocking(),
            ),
        }
    }

    if changes.is_empty() {
        report.push(Finding::new(
            "git.clean",
            FindingStatus::Pass,
            if revisions.is_some() {
                "requested reach contains no changed paths"
            } else {
                "working tree has no local changes"
            },
        ));
    } else {
        report.push(Finding::new(
            "git.changes",
            FindingStatus::Info,
            format!("{} changed path(s) evaluated", changes.len()),
        ));
    }

    if needs_package_verify {
        report.push(
            Finding::new(
                "package.verify.pending",
                FindingStatus::Warn,
                "agentic package fidelity verification is required before reach",
            )
            .remediation("run `epr ready` or `pnpm run elohim-agent:packages:verify`")
            .blocking(),
        );
    }

    report.data = json!({ "changedPaths": changes });
    report.recompute();
    Ok(CheckOutcome {
        report,
        needs_package_verify,
        touches_eprfs,
    })
}
