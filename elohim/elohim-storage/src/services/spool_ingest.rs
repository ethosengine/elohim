//! Ingest of ark death-witness spool files into private content and blob storage.

use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use ark_core::DeathWitness;
use tracing::{info, warn};

use crate::{
    blob_store::BlobStore, db::content_diesel::CreateContentInput, error::StorageError,
    services::ContentService,
};

/// Configuration for polling one ark spool.
pub struct SpoolIngestConfig {
    /// Ark spool root (`<data_root>/ark`).
    pub spool_root: PathBuf,
    /// Delay between directory snapshots.
    pub poll: Duration,
}

/// Pulls complete death-witness files from an ark spool into storage.
pub struct SpoolIngest {
    content: Arc<ContentService>,
    blobs: Arc<BlobStore>,
    cfg: SpoolIngestConfig,
    /// This storage peer's canonical Holochain agent CID, resolved at
    /// composition time. Never a transport id.
    self_agent: Option<String>,
    seen: HashSet<String>,
}

impl SpoolIngest {
    /// Construct an ark spool ingester.
    pub fn new(
        cfg: SpoolIngestConfig,
        content: Arc<ContentService>,
        blobs: Arc<BlobStore>,
        self_agent: Option<String>,
    ) -> Self {
        Self {
            content,
            blobs,
            cfg,
            self_agent,
            seen: HashSet::new(),
        }
    }

    /// Run one bounded directory pass and return the witness CIDs fully ingested.
    pub async fn run_once(&mut self) -> Result<Vec<String>, StorageError> {
        let witness_dir = self.cfg.spool_root.join("witnesses");
        let mut entries = tokio::fs::read_dir(&witness_dir).await?;
        let mut paths = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("cbor") {
                paths.push(path);
            }
        }
        paths.sort();

        // bounded-work: exactly one finite directory snapshot per tick; failures wait for the
        // next configured poll rather than entering a retry ladder inside this pass.
        let mut ingested = Vec::new();
        for path in paths {
            let Some(filename_cid) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
            else {
                warn!(path = %path.display(), "spool-ingest: refusing non-UTF-8 witness filename");
                continue;
            };

            if self.seen.contains(&filename_cid) {
                continue;
            }

            let bytes = match tokio::fs::read(&path).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    warn!(path = %path.display(), error = %error, "spool-ingest: witness read failed");
                    continue;
                }
            };
            let witness: DeathWitness = match serde_ipld_dagcbor::from_slice(&bytes) {
                Ok(witness) => witness,
                Err(error) => {
                    warn!(path = %path.display(), error = %error, "spool-ingest: refusing invalid death witness");
                    continue;
                }
            };
            let actual_cid = match witness.cid() {
                Ok(cid) => cid,
                Err(error) => {
                    warn!(path = %path.display(), error = %error, "spool-ingest: refusing unaddressable death witness");
                    continue;
                }
            };
            if actual_cid != filename_cid {
                warn!(
                    path = %path.display(),
                    filename_cid = %filename_cid,
                    actual_cid = %actual_cid,
                    "spool-ingest: refusing mislabelled death witness"
                );
                continue;
            }

            let (_, expected_hash) = BlobStore::compute_addresses(&bytes);
            let witness_digest = BlobStore::parse_content_address(&filename_cid)?;
            let bytes_digest = BlobStore::parse_content_address(&expected_hash)?;
            if witness_digest != bytes_digest {
                warn!(
                    path = %path.display(),
                    filename_cid = %filename_cid,
                    bytes_digest = %bytes_digest,
                    "spool-ingest: refusing witness bytes whose digest differs from filename"
                );
                continue;
            }

            let row_exists = self.content.get(&filename_cid)?.is_some();
            if self.blobs.exists_by_address(&expected_hash).await? {
                let present_bytes = self.blobs.get(&expected_hash).await?;
                let present_digest = BlobStore::parse_content_address(
                    &BlobStore::compute_addresses(&present_bytes).1,
                )?;
                if present_digest == witness_digest && row_exists {
                    self.seen.insert(filename_cid);
                    continue;
                }
                if present_digest != witness_digest {
                    warn!(
                        filename_cid = %filename_cid,
                        expected_digest = %witness_digest,
                        actual_digest = %present_digest,
                        "spool-ingest: repairing blob whose bytes do not match its address"
                    );
                    self.blobs.delete(&expected_hash).await?;
                }
            }

            if !row_exists {
                self.content.create(witness_content_input(
                    &witness,
                    &filename_cid,
                    &bytes,
                    self.self_agent.as_deref(),
                ))?;
            }

            // Deliberately after the private row: blob_reach treats row-less bytes as servable.
            let stored = self.blobs.store(&bytes).await?;
            let hash_digest = BlobStore::parse_content_address(&stored.hash)?;
            let blob_cid_digest = BlobStore::parse_content_address(&stored.cid)?;
            if witness_digest != hash_digest || witness_digest != blob_cid_digest {
                return Err(StorageError::HashMismatch {
                    expected: witness_digest,
                    actual: format!("hash={hash_digest}, cid={blob_cid_digest}"),
                });
            }

            self.seen.insert(filename_cid.clone());
            info!(
                witness_cid = %filename_cid,
                blob_hash = %stored.hash,
                blob_cid = %stored.cid,
                "spool-ingest: death witness stored"
            );
            ingested.push(filename_cid);
        }

        Ok(ingested)
    }

    /// Spawn the non-fatal polling loop.
    pub fn spawn(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let poll = self.cfg.poll.max(Duration::from_secs(1));
            let mut ticker = tokio::time::interval(poll);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // bounded-work: one run_once snapshot per tick; errors are logged and retried only
            // after the poll interval, so missed ticks never accumulate into a catch-up burst.
            loop {
                ticker.tick().await;
                if let Err(error) = self.run_once().await {
                    warn!(error = %error, "spool-ingest: pass failed; retrying on next poll");
                }
            }
        })
    }
}

/// Build the private content row corresponding to one death witness.
pub fn witness_content_input(
    witness: &DeathWitness,
    cid: &str,
    bytes: &[u8],
    self_agent: Option<&str>,
) -> CreateContentInput {
    let (blob_cid, blob_hash) = BlobStore::compute_addresses(bytes);
    let metadata = serde_json::json!({
        "kind": "death-witness",
        "incident": witness.incident,
        "process": witness.process,
        "pid": witness.pid,
        "exit": witness.exit,
        "died_at_epoch_ms": witness.died_at_epoch_ms,
        "artifact_sha256": witness.artifact_sha256,
    });

    CreateContentInput {
        id: cid.to_string(),
        title: format!(
            "death witness: {} {} ({})",
            witness.process,
            witness.exit.same_cause_token(),
            witness.died_at_epoch_ms
        ),
        description: None,
        content_type: "issue-report".to_string(),
        content_format: "json".to_string(),
        blob_hash: Some(blob_hash),
        blob_cid: Some(blob_cid.to_string()),
        content_size_bytes: Some(i32::try_from(bytes.len()).unwrap_or(i32::MAX)),
        metadata_json: Some(metadata.to_string()),
        reach: "private".to_string(),
        // The mesh berth passport has no node. Prefer this peer's resolved
        // agent CID so its local unanchored row still names the ward; when it
        // is unavailable, preserve the passport value/None. The composition
        // root resolves this through the same session/cell-key seam used by
        // custody authoring, so a transport id can never be stamped here.
        created_by: self_agent
            .map(str::to_string)
            .or_else(|| witness.passport.node.clone()),
        tags: Vec::new(),
        content_body: None,
        dht_anchor_hash: None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ark_core::{
        DeathWitness, EffectiveTier, ExitClass, Passport, ProcessPassport, RestartVerdict,
        PASSPORT_KIND, WITNESS_KIND,
    };
    use serde_json::Value;

    use super::*;
    use crate::{
        db::{content_diesel, AppContext},
        services::Services,
        test_util::test_pool,
    };

    fn witness(died_at_epoch_ms: u64) -> DeathWitness {
        DeathWitness {
            schema: 1,
            kind: WITNESS_KIND.to_string(),
            incident: elohim_epr::cid::compute_cid(b"incident").to_string(),
            process: "conductor".to_string(),
            incarnation: 1,
            pid: 42,
            artifact_sha256: "ab".repeat(32),
            artifact_path: "/opt/elohim/conductor".to_string(),
            started_at_epoch_ms: 1_000,
            died_at_epoch_ms,
            uptime_ms: died_at_epoch_ms - 1_000,
            exit: ExitClass::Signaled {
                signal: 9,
                core_dumped: false,
            },
            last_stderr: vec!["fatal".to_string()],
            last_stdout: vec!["Conductor ready.".to_string()],
            sample: None,
            last_intent: None,
            passport: Passport {
                schema: 1,
                kind: PASSPORT_KIND.to_string(),
                manifest: elohim_epr::cid::compute_cid(b"runtime-manifest").to_string(),
                node: Some("human-jessica".to_string()),
                incarnation: 1,
                ark_version: "0.1.0".to_string(),
                processes: vec![ProcessPassport {
                    name: "conductor".to_string(),
                    artifact_sha256: "ab".repeat(32),
                    artifact_path: "/opt/elohim/conductor".to_string(),
                    pid: Some(42),
                    started_at_epoch_ms: Some(1_000),
                    ready: false,
                    effective_tier: EffectiveTier::None,
                    deaths_in_window: 1,
                }],
                last_verdict: Some(RestartVerdict::Stop),
                updated_at_epoch_ms: died_at_epoch_ms,
            },
            verdict: Some(RestartVerdict::Stop),
            refusal: None,
            bounded_by: None,
            pain: None,
        }
    }

    fn write_witness(spool_root: &Path, witness: &DeathWitness, filename_cid: &str) -> Vec<u8> {
        let witnesses = spool_root.join("witnesses");
        std::fs::create_dir_all(&witnesses).unwrap();
        let bytes = witness.canonical_bytes().unwrap();
        std::fs::write(witnesses.join(format!("{filename_cid}.cbor")), &bytes).unwrap();
        bytes
    }

    #[test]
    fn witness_row_stamps_the_resolved_self_agent_when_the_passport_has_no_node() {
        let mut death_witness = witness(2_000);
        death_witness.passport.node = None;
        let bytes = death_witness.canonical_bytes().unwrap();
        let cid = death_witness.cid().unwrap();
        let row =
            witness_content_input(&death_witness, &cid, &bytes, Some("uhCAkResolvedSelfAgent"));
        assert_eq!(row.created_by.as_deref(), Some("uhCAkResolvedSelfAgent"));
    }

    #[test]
    fn witness_row_preserves_the_passport_or_none_when_self_agent_is_unresolved() {
        let death_witness = witness(2_000);
        let bytes = death_witness.canonical_bytes().unwrap();
        let cid = death_witness.cid().unwrap();
        let row = witness_content_input(&death_witness, &cid, &bytes, None);
        assert_eq!(
            row.created_by.as_deref(),
            death_witness.passport.node.as_deref()
        );

        let mut unlabelled = death_witness;
        unlabelled.passport.node = None;
        let bytes = unlabelled.canonical_bytes().unwrap();
        let cid = unlabelled.cid().unwrap();
        let row = witness_content_input(&unlabelled, &cid, &bytes, None);
        assert_eq!(row.created_by, None);
    }

    async fn ingester(
        spool_root: &Path,
        blob_root: &Path,
    ) -> (
        SpoolIngest,
        Arc<ContentService>,
        Arc<BlobStore>,
        crate::db::DbPool,
    ) {
        let pool = test_pool();
        let services = Services::new_without_events(pool.clone());
        let content = Arc::clone(&services.content);
        let blobs = Arc::new(BlobStore::new(blob_root).await.unwrap());
        let ingest = SpoolIngest::new(
            SpoolIngestConfig {
                spool_root: spool_root.to_path_buf(),
                poll: Duration::from_secs(5),
            },
            Arc::clone(&content),
            Arc::clone(&blobs),
            None,
        );
        (ingest, content, blobs, pool)
    }

    #[tokio::test]
    async fn ingests_a_witness_row_before_blob_with_one_digest() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let blob_root = tmp.path().join("pantry");
        let witness = witness(2_000);
        let cid = witness.cid().unwrap();
        let bytes = write_witness(&spool_root, &witness, &cid);
        let expected_hash = BlobStore::compute_hash(&bytes);
        let (mut ingest, content, blobs, _) = ingester(&spool_root, &blob_root).await;

        assert_eq!(ingest.run_once().await.unwrap(), vec![cid.clone()]);

        let row = content.get(&cid).unwrap().expect("witness content row");
        assert_eq!(row.id, cid);
        assert_eq!(row.reach, "private");
        assert_eq!(row.blob_hash.as_deref(), Some(expected_hash.as_str()));
        let metadata: Value = serde_json::from_str(row.metadata_json.as_deref().unwrap()).unwrap();
        assert_eq!(metadata["kind"], "death-witness");
        assert!(blobs.exists_by_address(&expected_hash).await.unwrap());
        assert_eq!(
            BlobStore::parse_content_address(row.blob_cid.as_deref().unwrap()).unwrap(),
            BlobStore::parse_content_address(&row.id).unwrap()
        );
    }

    #[tokio::test]
    async fn rerun_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let witness = witness(2_000);
        let cid = witness.cid().unwrap();
        write_witness(&spool_root, &witness, &cid);
        let (mut ingest, _, _, pool) = ingester(&spool_root, &tmp.path().join("pantry")).await;

        assert_eq!(ingest.run_once().await.unwrap(), vec![cid]);
        assert!(ingest.run_once().await.unwrap().is_empty());
        let mut conn = pool.get().unwrap();
        assert_eq!(
            content_diesel::content_count(&mut conn, &AppContext::default_lamad()).unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn a_mislabelled_file_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let death_witness = witness(2_000);
        let actual_cid = death_witness.cid().unwrap();
        let other_cid = witness(3_000).cid().unwrap();
        let bytes = write_witness(&spool_root, &death_witness, &other_cid);
        let expected_hash = BlobStore::compute_hash(&bytes);
        let (mut ingest, content, blobs, _) =
            ingester(&spool_root, &tmp.path().join("pantry")).await;

        assert!(ingest.run_once().await.unwrap().is_empty());
        assert!(content.get(&actual_cid).unwrap().is_none());
        assert!(content.get(&other_cid).unwrap().is_none());
        assert!(!blobs.exists_by_address(&expected_hash).await.unwrap());
    }

    #[tokio::test]
    async fn row_is_written_before_blob() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let blob_root = tmp.path().join("pantry");
        let witness = witness(2_000);
        let cid = witness.cid().unwrap();
        let bytes = write_witness(&spool_root, &witness, &cid);
        let expected_hash = BlobStore::compute_hash(&bytes);
        let (mut ingest, content, blobs, pool) = ingester(&spool_root, &blob_root).await;
        let obstruction = blob_root.join("blobs");
        std::fs::write(&obstruction, b"not a directory").unwrap();

        assert!(ingest.run_once().await.is_err());
        assert!(content.get(&cid).unwrap().is_some());
        assert!(!blobs.exists_by_address(&expected_hash).await.unwrap());

        std::fs::remove_file(obstruction).unwrap();
        assert_eq!(ingest.run_once().await.unwrap(), vec![cid]);
        assert!(blobs.exists_by_address(&expected_hash).await.unwrap());
        let mut conn = pool.get().unwrap();
        assert_eq!(
            content_diesel::content_count(&mut conn, &AppContext::default_lamad()).unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn a_partial_blob_is_repaired_on_the_next_pass() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let blob_root = tmp.path().join("pantry");
        let witness = witness(2_000);
        let cid = witness.cid().unwrap();
        let bytes = write_witness(&spool_root, &witness, &cid);
        let expected_hash = BlobStore::compute_hash(&bytes);
        let hash_part = expected_hash.strip_prefix("sha256-").unwrap();
        let blob_path = blob_root
            .join("blobs")
            .join(&hash_part[..4])
            .join(&expected_hash);
        std::fs::create_dir_all(blob_path.parent().unwrap()).unwrap();
        std::fs::write(&blob_path, &bytes[..bytes.len() / 2]).unwrap();
        let (mut ingest, _, blobs, pool) = ingester(&spool_root, &blob_root).await;

        assert_eq!(ingest.run_once().await.unwrap(), vec![cid.clone()]);

        let mut conn = pool.get().unwrap();
        assert_eq!(
            content_diesel::content_count(&mut conn, &AppContext::default_lamad()).unwrap(),
            1
        );
        let repaired = blobs.get(&expected_hash).await.unwrap();
        assert_eq!(repaired, bytes);
        assert_eq!(
            BlobStore::parse_content_address(&BlobStore::compute_hash(&repaired)).unwrap(),
            BlobStore::parse_content_address(&cid).unwrap()
        );
    }

    #[tokio::test]
    async fn noncanonical_bytes_are_refused_before_any_write() {
        let tmp = tempfile::tempdir().unwrap();
        let spool_root = tmp.path().join("ark");
        let death_witness = witness(2_000);
        let cid = death_witness.cid().unwrap();
        let canonical = death_witness.canonical_bytes().unwrap();
        let schema_marker = b"\x66schema\x01";
        let marker_start = canonical
            .windows(schema_marker.len())
            .position(|window| window == schema_marker)
            .unwrap();
        let mut noncanonical = canonical.clone();
        let schema_value = marker_start + schema_marker.len() - 1;
        noncanonical.splice(schema_value..=schema_value, [0x18, 0x01]);
        assert_ne!(noncanonical, canonical);
        let decoded: DeathWitness = serde_ipld_dagcbor::from_slice(&noncanonical).unwrap();
        assert_eq!(decoded.cid().unwrap(), cid);
        let witnesses = spool_root.join("witnesses");
        std::fs::create_dir_all(&witnesses).unwrap();
        std::fs::write(witnesses.join(format!("{cid}.cbor")), &noncanonical).unwrap();
        let unexpected_hash = BlobStore::compute_hash(&noncanonical);
        let (mut ingest, content, blobs, _) =
            ingester(&spool_root, &tmp.path().join("pantry")).await;

        assert!(ingest.run_once().await.unwrap().is_empty());
        assert!(content.get(&cid).unwrap().is_none());
        assert!(!blobs.exists_by_address(&unexpected_hash).await.unwrap());
    }
}
