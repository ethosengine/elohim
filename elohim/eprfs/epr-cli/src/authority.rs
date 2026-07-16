use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

pub const PACKAGE_ROOT: &str = ".epr-meta/elohim/packages";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityKind {
    Package,
    RuntimeSource,
    Unmanaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageAuthority {
    pub kind: AuthorityKind,
    pub package_kind: String,
    pub package_id: String,
    pub package_path: String,
    pub master: Option<String>,
    pub source_path: Option<String>,
    pub projection_paths: Vec<String>,
    pub governance_ref: Option<String>,
    pub gates: Vec<String>,
}

impl PackageAuthority {
    pub fn is_package_master_projection(&self, path: &str) -> bool {
        self.kind == AuthorityKind::Package
            && self.master.as_deref() == Some("package")
            && self
                .projection_paths
                .iter()
                .any(|projection| projection == path)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuthorityIndex {
    by_path: BTreeMap<String, PackageAuthority>,
    packages: Vec<PackageAuthority>,
    diagnostics: Vec<String>,
}

impl AuthorityIndex {
    pub fn load(repo_root: &Path) -> Result<Self> {
        let root = repo_root.join(PACKAGE_ROOT);
        if !root.is_dir() {
            return Ok(Self::default());
        }

        let mut package_files = Vec::new();
        collect_json(&root, &mut package_files).map_err(|source| Error::Read {
            path: root.clone(),
            source,
        })?;
        package_files.sort();

        let mut index = Self::default();
        for package_path in package_files {
            let bytes = fs::read(&package_path).map_err(|source| Error::Read {
                path: package_path.clone(),
                source,
            })?;
            let package: Value =
                serde_json::from_slice(&bytes).map_err(|source| Error::PackageJson {
                    path: package_path.clone(),
                    source,
                })?;
            let relative_package = relative(repo_root, &package_path);
            let authority = authority_from_package(&package, relative_package.clone());

            index.insert_path(relative_package, authority.clone());
            if let Some(source) = authority.source_path.clone() {
                index.insert_path(source, authority.clone());
            }
            for projection in &authority.projection_paths {
                index.insert_path(projection.clone(), authority.clone());
            }
            index.packages.push(authority);
        }
        Ok(index)
    }

    pub fn find(&self, path: impl AsRef<Path>) -> Option<&PackageAuthority> {
        self.by_path.get(&normalize(path.as_ref()))
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn is_agentic_path(path: &str) -> bool {
        path.starts_with(".epr-meta/elohim/")
            || path.starts_with(".claude/skills/")
            || path.starts_with(".claude/agents/")
            || path.starts_with(".claude/commands/")
            || path.starts_with(".claude/hooks/")
            || path.starts_with(".codex/")
            || path.starts_with(".agents/skills/")
            || matches!(path, "AGENTS.md" | "CLAUDE.md" | ".mcp.json")
    }

    fn insert_path(&mut self, path: String, authority: PackageAuthority) {
        if let Some(existing) = self.by_path.insert(path.clone(), authority.clone()) {
            if existing.package_path != authority.package_path
                && !is_composed_projection(&path, &existing, &authority)
            {
                self.diagnostics.push(format!(
                    "path `{path}` is declared by both `{}` and `{}`",
                    existing.package_path, authority.package_path
                ));
            }
        }
    }
}

fn is_composed_projection(path: &str, left: &PackageAuthority, right: &PackageAuthority) -> bool {
    matches!(path, ".mcp.json" | ".codex/config.toml")
        && [left, right].iter().all(|package| {
            matches!(
                package.package_kind.as_str(),
                "McpServerPackage" | "McpProfilePackage"
            )
        })
}

fn authority_from_package(package: &Value, package_path: String) -> PackageAuthority {
    let metadata = package.get("metadata").unwrap_or(&Value::Null);
    let package_kind = package
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("UnknownPackage")
        .to_string();
    let package_id = metadata
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let master = metadata
        .get("master")
        .and_then(Value::as_str)
        .map(str::to_string);
    let source_path = package
        .get("source")
        .and_then(|source| source.get("path"))
        .and_then(Value::as_str)
        .map(normalize_str);
    let mut projection_paths = package
        .get("projections")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(_, projection)| projection.get("path").and_then(Value::as_str))
        .map(normalize_str)
        .collect::<Vec<_>>();
    projection_paths.sort();
    projection_paths.dedup();

    let governance = metadata.get("governance").unwrap_or(&Value::Null);
    let governance_ref = governance
        .get("eprRef")
        .and_then(Value::as_str)
        .map(str::to_string);
    let gates = governance
        .get("gates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    let kind = if master.as_deref() == Some("package") {
        AuthorityKind::Package
    } else if source_path.is_some() {
        AuthorityKind::RuntimeSource
    } else {
        AuthorityKind::Unmanaged
    };

    PackageAuthority {
        kind,
        package_kind,
        package_id,
        package_path,
        master,
        source_path,
        projection_paths,
        governance_ref,
        gates,
    }
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json(&path, out)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    normalize(path.strip_prefix(root).unwrap_or(path))
}

pub fn normalize(path: &Path) -> String {
    normalize_str(&path.to_string_lossy())
}

fn normalize_str(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn indexes_package_projection_and_source_authority() {
        let dir = TempDir::new().unwrap();
        let packages = dir.path().join(PACKAGE_ROOT).join("skills");
        fs::create_dir_all(&packages).unwrap();
        fs::write(
            packages.join("alpha.json"),
            r#"{
              "kind":"SkillPackage",
              "metadata":{"id":"alpha","master":"package","governance":{"eprRef":"epr:alpha","gates":["verify"]}},
              "source":{"path":".claude/skills/alpha/SKILL.md"},
              "projections":{"claude":{"path":".claude/skills/alpha/SKILL.md"},"codex":{"path":".codex/skills/alpha/SKILL.md"}}
            }"#,
        )
        .unwrap();

        let index = AuthorityIndex::load(dir.path()).unwrap();
        let authority = index
            .find(Path::new(".codex/skills/alpha/SKILL.md"))
            .unwrap();

        assert_eq!(authority.kind, AuthorityKind::Package);
        assert_eq!(authority.package_id, "alpha");
        assert!(authority.is_package_master_projection(".codex/skills/alpha/SKILL.md"));
        assert_eq!(index.package_count(), 1);
    }

    #[test]
    fn accepts_declared_mcp_composition_on_shared_runtime_files() {
        let left = PackageAuthority {
            kind: AuthorityKind::Package,
            package_kind: "McpServerPackage".into(),
            package_id: "one".into(),
            package_path: "one.json".into(),
            master: Some("package".into()),
            source_path: None,
            projection_paths: vec![".mcp.json".into()],
            governance_ref: None,
            gates: vec![],
        };
        let mut right = left.clone();
        right.package_id = "two".into();
        right.package_path = "two.json".into();
        assert!(is_composed_projection(".mcp.json", &left, &right));
    }
}
