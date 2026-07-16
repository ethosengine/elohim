use std::path::Path;

use serde_json::json;

use crate::{
    authority::{normalize, AuthorityIndex, AuthorityKind},
    error::Result,
    report::{Finding, FindingStatus, Report},
};

pub fn run(root: &Path, path: &Path) -> Result<Report> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let normalized = normalize(relative);
    let target = root.join(&normalized);
    let authority = AuthorityIndex::load(root)?;
    let package = authority.find(Path::new(&normalized)).cloned();
    let resolution = eprfs_meta::resolve_path(root, &target)?;

    let mut report = Report::new("explain", root);
    match package.as_ref() {
        Some(package) if package.is_package_master_projection(&normalized) => report.push(
            Finding::new(
                "authority.package-projection",
                FindingStatus::Warn,
                format!(
                    "`{normalized}` is generated from package `{}`",
                    package.package_path
                ),
            )
            .remediation(format!(
                "edit `{}`, then project only {}:{} and verify",
                package.package_path, package.package_kind, package.package_id
            ))
            .path(&normalized)
            .blocking(),
        ),
        Some(package) if package.kind == AuthorityKind::Package => report.push(
            Finding::new(
                "authority.package",
                FindingStatus::Pass,
                format!("`{normalized}` is an authoritative package source"),
            )
            .path(&normalized),
        ),
        Some(package) if package.kind == AuthorityKind::RuntimeSource => report.push(
            Finding::new(
                "authority.runtime-source",
                FindingStatus::Info,
                format!(
                    "`{normalized}` is the runtime source mirrored by `{}`",
                    package.package_path
                ),
            )
            .path(&normalized),
        ),
        _ => report.push(
            Finding::new(
                "authority.unmanaged",
                FindingStatus::Info,
                "path is not an Elohim capability-package projection",
            )
            .path(&normalized),
        ),
    }

    if resolution.records.is_empty() {
        report.push(Finding::new(
            "governance.no-cascade",
            FindingStatus::Warn,
            "no `.epr-meta` manifest applies to this path",
        ));
    } else {
        report.push(Finding::new(
            "governance.cascade",
            FindingStatus::Pass,
            format!(
                "{} manifest(s), {} inline rule(s), {} policy binding(s) apply",
                resolution.records.len(),
                resolution.effective_rules.len(),
                resolution.effective_policies.len()
            ),
        ));
    }
    report.data = json!({
        "path": normalized,
        "authority": package,
        "governance": resolution,
    });
    report.recompute();
    Ok(report)
}
