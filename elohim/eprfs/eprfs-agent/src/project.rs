//! Emit an eprfs ProjectionManifest from capabilities + bindings.

use bytes::Bytes;
use eprfs_core::storage::EprfsStorage;
use eprfs_core::{
    EntryKind, EprRef, ProjectionEntry, ProjectionId, ProjectionManifest, ProjectionPath,
    ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
};

use crate::binding::ProjectionBinding;
use crate::canonical::CanonicalAgent;
use crate::error::Result;

pub async fn project<S: EprfsStorage>(
    agents: &[CanonicalAgent],
    bindings: &[ProjectionBinding],
    storage: &S,
) -> Result<ProjectionManifest> {
    let mut entries = Vec::new();

    for agent in agents {
        for binding in bindings {
            let bytes = binding.render(agent);
            let size = bytes.len() as u64;
            let blob = storage.put_blob(Bytes::from(bytes)).await?;
            let path = ProjectionPath::new(binding.target_path(&agent.slug))?;

            entries.push(ProjectionEntry {
                path,
                kind: EntryKind::File,
                source: Some(ProjectionSource::new(
                    "elohim-agent",
                    ProjectionSourceKind::Content,
                    agent.slug.clone(),
                )),
                epr: None,
                blob: Some(blob),
                size_bytes: Some(size),
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: serde_json::Value::Null,
            });
        }
    }

    let manifest = ProjectionManifest {
        root: ProjectionRoot {
            id: ProjectionId::new("elohim-agent"),
            root: EprRef::new("epr:elohim-agent:capabilities"),
        },
        entries,
        metadata: serde_json::Value::Null,
    };
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanonicalAgent, ProjectionBinding};
    use eprfs_storage::MemoryStorage;

    fn sample() -> CanonicalAgent {
        CanonicalAgent::parse(
            "---\nname: code-reviewer\ndescription: Reviews.\ntools: Bash\n---\n\nBody.\n",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn projects_one_agent_to_two_runtime_entries() {
        let storage = MemoryStorage::default();
        let manifest = project(
            &[sample()],
            &[
                ProjectionBinding::claude_agent(),
                ProjectionBinding::codex_agent(),
            ],
            &storage,
        )
        .await
        .unwrap();

        assert_eq!(manifest.entries.len(), 2);
        let paths: Vec<_> = manifest
            .entries
            .iter()
            .map(|e| e.path.as_path().to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&".claude/agents/code-reviewer.md".to_string()));
        assert!(paths.contains(&".codex/agents/code-reviewer.md".to_string()));
        // validate() ran inside project(); the two surfaces have DIFFERENT CIDs.
        let claude = manifest
            .entries
            .iter()
            .find(|e| e.path.as_path().starts_with(".claude"))
            .unwrap();
        let codex = manifest
            .entries
            .iter()
            .find(|e| e.path.as_path().starts_with(".codex"))
            .unwrap();
        assert_ne!(claude.blob, codex.blob);
        // size_bytes reflects the actual rendered length for both surfaces.
        assert_eq!(
            claude.size_bytes,
            Some(ProjectionBinding::claude_agent().render(&sample()).len() as u64)
        );
        assert_eq!(
            codex.size_bytes,
            Some(ProjectionBinding::codex_agent().render(&sample()).len() as u64)
        );
    }
}
