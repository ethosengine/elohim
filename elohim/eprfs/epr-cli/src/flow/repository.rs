//! Explicit local repository attribution. This declaration names a flow container,
//! never a participant, network authority, or identity inferred from a Git remote.
use super::{FlowError, FlowResult};
use cid::Cid;
use elohim_epr_rea::{atom_cid, AgentRef, PinnedRef};
use serde::Deserialize;
use std::path::Path;

pub const DECLARATION: &str = ".epr-meta/repository.yaml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    version: u32,
    agent: String,
}

/// Missing and malformed declarations refuse instead of borrowing Elohim's identity.
pub fn repository_agent(root: &Path) -> FlowResult<AgentRef> {
    let path = root.join(DECLARATION);
    let bytes = std::fs::read_to_string(&path).map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    let declaration: Declaration =
        serde_yaml::from_str(&bytes).map_err(|source| FlowError::Yaml {
            path: path.clone(),
            source,
        })?;
    let valid = declaration.agent.strip_prefix("repo:").is_some_and(|name| {
        !name.is_empty()
            && name.split('/').all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            })
    });
    if declaration.version != 1 || !valid {
        return Err(FlowError::InvalidArguments(format!(
            "{} requires version: 1 and an explicit repo:<namespace/name> agent",
            path.display()
        )));
    }
    Ok(AgentRef(declaration.agent))
}

pub fn repository_scope(root: &Path) -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: repository_agent(root)?.0,
        version: 1,
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn declared(agent: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".epr-meta")).unwrap();
        std::fs::write(
            dir.path().join(DECLARATION),
            format!("version: 1\nagent: {agent}\n"),
        )
        .unwrap();
        dir
    }
    #[test]
    fn distinct_repositories_never_share_attribution() {
        let a = declared("repo:example/a");
        let b = declared("repo:example/b");
        assert_ne!(
            repository_scope(a.path()).unwrap(),
            repository_scope(b.path()).unwrap()
        );
        assert_eq!(repository_agent(b.path()).unwrap().0, "repo:example/b");
    }
    #[test]
    fn elohim_declaration_preserves_historical_addresses() {
        let root = declared(super::super::REPO_AGENT);
        assert_eq!(
            repository_scope(root.path()).unwrap(),
            super::super::repo_scope_atom().unwrap()
        );
        assert_eq!(
            repository_agent(root.path()).unwrap(),
            super::super::repo_agent()
        );
    }
    #[test]
    fn absent_and_invalid_declarations_refuse() {
        let root = tempfile::tempdir().unwrap();
        assert!(repository_agent(root.path()).is_err());
        let root = declared("human:someone");
        assert!(repository_agent(root.path()).is_err());
    }
}
