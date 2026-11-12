//! Accept an existing verified witness on retry; published notes are never rolled back.
use super::*;

pub(super) fn publish(
    repo: &Path,
    args: &[&str],
    key: &ClosureKey,
    step: &str,
    output: &BlobCid,
    agent: &str,
) -> ArtifactResult<()> {
    // Brit verifies the signature and the step binding before returning this typed body.
    // A missing/unverifiable note returns no JSON; it cannot satisfy an idempotent retry.
    let existing = brit(
        repo,
        &["build", "get", "--step", step, "--commit", &key.head],
    )?;
    if let Ok(body) = serde_json::from_str::<serde_json::Value>(&existing) {
        let cid_matches = |field: &str, cid: &BlobCid| {
            body[field] == cid.to_string()
                || body[field] == serde_json::json!(cid.as_cid().to_bytes())
        };
        if body["stepName"] == step
            && body["inputsHash"] == key.key_cid.to_string()
            && body["agentId"] == agent
            && body["success"] == true
            && cid_matches("outputCid", output)
            && cid_matches("manifestCid", &key.method)
        {
            return Ok(());
        }
    }
    brit(repo, args)?;
    Ok(())
}
