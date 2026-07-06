use eprfs_agent::{normalize, project, CanonicalAgent, ProjectionBinding};
use eprfs_core::MaterializationPolicy;
use eprfs_local::{has_drift, verify_projection, LocalMaterializer};
use eprfs_storage::MemoryStorage;

const CODE_REVIEWER: &str = include_str!("fixtures/code-reviewer.md");

#[tokio::test]
async fn claude_round_trip_is_lossless_and_drift_is_detectable() {
    // 1. author-once -> canonical
    let agent = CanonicalAgent::parse(CODE_REVIEWER).unwrap();

    // 2. project -> 3. manifest (claude + codex, one source two surfaces)
    let storage = MemoryStorage::default();
    let manifest = project(
        &[agent],
        &[
            ProjectionBinding::claude_agent(),
            ProjectionBinding::codex_agent(),
        ],
        &storage,
    )
    .await
    .unwrap();

    // 4. materialize to a scratch dir
    let dir = tempfile::tempdir().unwrap();
    let materializer = LocalMaterializer::new(storage);
    let report = materializer
        .materialize(&manifest, dir.path(), MaterializationPolicy::LocalOnly)
        .await
        .unwrap();
    assert_eq!(report.files_written, 2);

    // 5a. ACCEPTANCE: the .claude projection is normalized-equal to the authored file
    let projected = tokio::fs::read(dir.path().join(".claude/agents/code-reviewer.md"))
        .await
        .unwrap();
    assert_eq!(
        normalize(&projected),
        normalize(CODE_REVIEWER.as_bytes()),
        "claude projection must round-trip losslessly against the authored capability"
    );

    // 5b. the .codex projection is a DISTINCT surface from the SAME source
    let codex = tokio::fs::read_to_string(dir.path().join(".codex/agents/code-reviewer.md"))
        .await
        .unwrap();
    assert!(codex.contains("# code-reviewer"));
    assert!(codex.contains("You are the Code Review Specialist"));

    // 6. drift is detectable: a clean tree has none...
    let clean = verify_projection(&manifest, dir.path()).await.unwrap();
    assert!(
        !has_drift(&clean),
        "freshly materialized tree must be clean"
    );

    // ...and a hand-edit trips projection-drift-detected.
    tokio::fs::write(
        dir.path().join(".claude/agents/code-reviewer.md"),
        b"---\nname: code-reviewer\n---\nhand edited\n",
    )
    .await
    .unwrap();
    let drifted = verify_projection(&manifest, dir.path()).await.unwrap();
    assert!(
        has_drift(&drifted),
        "a hand-edited surface must report drift"
    );
}
