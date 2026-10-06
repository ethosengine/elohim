// `async_trait` marks its generated futures `#[must_use]`; clippy 1.99 reads
// that as doubled on methods returning `Result`. The attribute is the macro's.
#![allow(clippy::double_must_use)]

use bytes::Bytes;
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, EntryKind, EprRecord, EprRef, EprfsError,
    EprfsStorage, FetchPolicy, MaterializationPolicy, ProjectionEntry, ProjectionManifest,
    ProjectionPath, ProjectionRoot, ProjectionStatus, Result,
};
use eprfs_host::HostProfile;
use eprfs_local::{verify_projection, LocalMaterializer};

#[derive(Clone, Copy)]
enum Reply {
    Handle,
    Error,
}

struct FakeStorage {
    presence: BlobPresence,
    handle: BlobHandle,
    reply: Reply,
    expected_policy: FetchPolicy,
}

#[async_trait::async_trait]
impl EprfsStorage for FakeStorage {
    async fn resolve_epr(&self, _: &EprRef) -> Result<EprRecord> {
        unreachable!("materialization does not resolve EPRs")
    }

    async fn has_blob(&self, _: &BlobCid) -> Result<BlobPresence> {
        Ok(self.presence.clone())
    }

    async fn fetch_blob(&self, _: &BlobCid, policy: FetchPolicy) -> Result<BlobHandle> {
        assert_eq!(policy, self.expected_policy);
        match self.reply {
            Reply::Handle => Ok(self.handle.clone()),
            Reply::Error => Err(EprfsError::Storage("fetch unavailable".into())),
        }
    }

    async fn put_blob(&self, _: Bytes) -> Result<BlobCid> {
        unreachable!("materialization does not publish blobs")
    }

    async fn publish_attestation(&self, _: AttestationDraft) -> Result<EprRef> {
        unreachable!("materialization does not publish attestations")
    }
}

fn manifest(path: &str, kind: EntryKind, cid: BlobCid) -> ProjectionManifest {
    ProjectionManifest {
        root: ProjectionRoot {
            id: eprfs_core::ProjectionId::new("integrity"),
            root: "epr:integrity".into(),
        },
        entries: vec![ProjectionEntry {
            path: ProjectionPath::new(path).unwrap(),
            kind,
            source: None,
            epr: None,
            blob: Some(cid),
            size_bytes: None,
            executable: false,
            status: ProjectionStatus::Unknown,
            metadata: serde_json::Value::Null,
        }],
        metadata: serde_json::Value::Null,
    }
}

fn storage(
    presence: BlobPresence,
    cid: BlobCid,
    bytes: &'static [u8],
    reply: Reply,
) -> FakeStorage {
    let expected_policy = if presence == BlobPresence::Local {
        FetchPolicy::LocalOnly
    } else {
        FetchPolicy::FetchIfMissing
    };
    FakeStorage {
        presence,
        handle: BlobHandle {
            cid,
            bytes: Bytes::from_static(bytes),
        },
        reply,
        expected_policy,
    }
}

#[tokio::test]
async fn wrong_cid_and_corrupt_bytes_leave_file_and_symlink_destinations_unchanged() {
    // The expected raw CID is an external golden value, not computed by the verifier under test.
    let expected =
        BlobCid::parse("bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
    let kinds = [EntryKind::File, EntryKind::Symlink];
    let presences = [BlobPresence::Local, BlobPresence::Remote];

    for kind in kinds {
        #[cfg(not(unix))]
        if kind == EntryKind::Symlink {
            continue;
        }
        for presence in presences.clone() {
            for wrong_cid in [false, true] {
                for existing in [false, true] {
                    let dir = tempfile::tempdir().unwrap();
                    let dest = dir.path().join("entry");
                    if existing {
                        tokio::fs::write(&dest, b"keep existing destination")
                            .await
                            .unwrap();
                    }

                    let returned_cid = if wrong_cid {
                        BlobCid::compute_raw(b"other")
                    } else {
                        expected.clone()
                    };
                    let returned_bytes = if wrong_cid {
                        b"hello world".as_slice()
                    } else {
                        b"corrupt bytes".as_slice()
                    };
                    let fake = storage(
                        presence.clone(),
                        returned_cid,
                        returned_bytes,
                        Reply::Handle,
                    );
                    let policy = if presence == BlobPresence::Local {
                        MaterializationPolicy::LocalOnly
                    } else {
                        MaterializationPolicy::FetchMissing
                    };
                    let result =
                        LocalMaterializer::with_host_profile(fake, HostProfile::linux_native())
                            .materialize(
                                &manifest("entry", kind.clone(), expected.clone()),
                                dir.path(),
                                policy,
                            )
                            .await;
                    assert!(matches!(result, Err(EprfsError::Storage(_))));
                    let message = result.unwrap_err().to_string();
                    assert!(message.contains("blob integrity verification failed"));
                    assert!(!message.contains("corrupt bytes"));
                    if existing {
                        assert_eq!(
                            tokio::fs::read(&dest).await.unwrap(),
                            b"keep existing destination"
                        );
                    } else {
                        assert!(!dest.exists());
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn raw_and_legacy_dag_cbor_cids_materialize_after_verification() {
    let raw =
        BlobCid::parse("bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
    let legacy =
        BlobCid::parse("bafyreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();

    for cid in [raw, legacy] {
        for presence in [BlobPresence::Local, BlobPresence::Remote] {
            for kind in [EntryKind::File, EntryKind::Symlink] {
                #[cfg(not(unix))]
                if kind == EntryKind::Symlink {
                    continue;
                }
                let dir = tempfile::tempdir().unwrap();
                let fake = storage(presence.clone(), cid.clone(), b"hello world", Reply::Handle);
                let policy = if presence == BlobPresence::Local {
                    MaterializationPolicy::LocalOnly
                } else {
                    MaterializationPolicy::FetchMissing
                };
                let report =
                    LocalMaterializer::with_host_profile(fake, HostProfile::linux_native())
                        .materialize(
                            &manifest("entry", kind.clone(), cid.clone()),
                            dir.path(),
                            policy,
                        )
                        .await
                        .unwrap();
                assert_eq!(
                    report.files_fetched,
                    usize::from(presence == BlobPresence::Remote)
                );
                let path = dir.path().join("entry");
                if kind == EntryKind::File {
                    assert_eq!(tokio::fs::read(&path).await.unwrap(), b"hello world");
                    let drifts =
                        verify_projection(&manifest("entry", kind, cid.clone()), dir.path())
                            .await
                            .unwrap();
                    assert_eq!(drifts[0].actual, Some(drifts[0].expected.clone()));
                } else {
                    #[cfg(unix)]
                    assert_eq!(
                        tokio::fs::read_link(&path).await.unwrap(),
                        std::path::PathBuf::from("hello world")
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn fetch_error_propagates_without_touching_destination() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("entry");
    tokio::fs::write(&dest, b"keep").await.unwrap();
    let cid = BlobCid::compute_raw(b"hello world");
    let fake = storage(
        BlobPresence::Remote,
        cid.clone(),
        b"hello world",
        Reply::Error,
    );
    let error = LocalMaterializer::new(fake)
        .materialize(
            &manifest("entry", EntryKind::File, cid),
            dir.path(),
            MaterializationPolicy::FetchMissing,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("fetch unavailable"));
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"keep");
}

#[tokio::test]
async fn unsupported_codec_refuses_materialization_and_drift_verification() {
    // CIDv1 dag-pb (0x70) over the same SHA-256 digest as the raw hello-world vector.
    // It is syntactically valid, but BlobCid deliberately supports only raw and DAG-CBOR.
    let unsupported =
        BlobCid::parse("bafybeifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("entry");
    tokio::fs::write(&dest, b"keep").await.unwrap();
    let fake = storage(
        BlobPresence::Local,
        unsupported.clone(),
        b"hello world",
        Reply::Handle,
    );
    let projection = manifest("entry", EntryKind::File, unsupported);
    let error = LocalMaterializer::new(fake)
        .materialize(&projection, dir.path(), MaterializationPolicy::LocalOnly)
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("blob integrity verification failed"));
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"keep");

    let error = verify_projection(&projection, dir.path())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unsupported blob codec"));
}

#[cfg(unix)]
#[tokio::test]
async fn corrupt_reply_preserves_existing_native_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("entry");
    std::os::unix::fs::symlink("old-target", &dest).unwrap();
    let requested = BlobCid::compute_raw(b"new-target");
    let fake = storage(
        BlobPresence::Remote,
        requested.clone(),
        b"wrong-target",
        Reply::Handle,
    );
    let error = LocalMaterializer::with_host_profile(fake, HostProfile::linux_native())
        .materialize(
            &manifest("entry", EntryKind::Symlink, requested),
            dir.path(),
            MaterializationPolicy::FetchMissing,
        )
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("blob integrity verification failed"));
    assert_eq!(
        tokio::fs::read_link(&dest).await.unwrap(),
        std::path::PathBuf::from("old-target")
    );
}

#[tokio::test]
async fn serialized_escape_cannot_write_outside_projection_root() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("projection");
    tokio::fs::create_dir(&target).await.unwrap();
    let outside = dir.path().join("outside");
    tokio::fs::write(&outside, b"keep outside sentinel")
        .await
        .unwrap();

    let bytes = b"replacement";
    let cid = BlobCid::compute_raw(bytes);
    let mut serialized =
        serde_json::to_value(manifest("inside", EntryKind::File, cid.clone())).unwrap();
    serialized["entries"][0]["path"] = serde_json::json!("../outside");

    // Rejection may happen while decoding or at the materializer's validation boundary.
    if let Ok(untrusted) = serde_json::from_value::<ProjectionManifest>(serialized) {
        let fake = storage(BlobPresence::Local, cid, bytes, Reply::Handle);
        let result = LocalMaterializer::new(fake)
            .materialize(&untrusted, &target, MaterializationPolicy::LocalOnly)
            .await;
        assert!(
            result.is_err(),
            "malicious manifest reached a destination write"
        );
    }
    assert_eq!(
        tokio::fs::read(&outside).await.unwrap(),
        b"keep outside sentinel"
    );
}
