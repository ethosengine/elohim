use super::*;

const HELPER: &str = "brit-rebase-retained";

#[test]
fn historical_install_upgrade_session_and_rollback_keep_one_release() -> Result<()> {
    let mut f = Fixture::new("brit");
    let full = f.store.config.bins.clone();
    f.store.config.bins.retain(|bin| bin != HELPER);
    let old = f.release("old");
    f.land(&old)?;
    f.store.pin(&identity("old-session"), None, false, None)?;
    f.store.config.bins = full;
    assert!(f.store.verified(&old.cid));
    assert_eq!(f.store.resolve_current(None)?.cid, Some(old.cid.clone()));
    let absent = f.store.resolve_current(Some(HELPER)).unwrap_err();
    assert_eq!(absent.kind, "candidate-absent");
    assert!(absent.detail.contains(HELPER) && absent.detail.contains(&old.cid));

    // Fresh installation of the historical transport also remains supported.
    let imported = f.store.ingest(
        &f.path("old"),
        &IngestOptions {
            source_commit: Some(f.commit()),
            tree_clean: Some(true),
            ..Default::default()
        },
    )?;
    assert_eq!(imported.cid, old.cid);

    f.state.lock().expect("fixture state").head = "b".repeat(40);
    let new = f.release("new");
    f.land(&new)?;
    assert_eq!(
        f.store.resolve_current(Some(HELPER))?.cid,
        Some(new.cid.clone())
    );
    assert_eq!(
        f.store
            .resolve(
                Some(HELPER),
                Some(&identity("new-session")),
                &|_| None,
                true
            )?
            .cid,
        Some(new.cid.clone())
    );
    let absent = f
        .store
        .resolve(
            Some(HELPER),
            Some(&identity("old-session")),
            &|_| None,
            true,
        )
        .unwrap_err();
    assert!(absent.detail.contains(&old.cid));
    assert_eq!(
        f.store
            .resolve(None, Some(&identity("old-session")), &|_| None, true)?
            .cid,
        Some(old.cid.clone())
    );
    let absent = f
        .store
        .resolve(
            Some(HELPER),
            None,
            &|key| (key == "BRIT_PIN_CID").then(|| old.cid.clone()),
            true,
        )
        .unwrap_err();
    assert!(absent.detail.contains(&old.cid));

    f.state.lock().expect("fixture state").head = "a".repeat(40);
    f.store
        .land(&old.cid, &f.commit(), f.tmp.path(), Transition::Revert)?;
    assert_eq!(f.store.resolve_current(None)?.cid, Some(old.cid.clone()));
    assert!(f
        .store
        .resolve_current(Some(HELPER))
        .unwrap_err()
        .detail
        .contains(&old.cid));
    assert_eq!(
        f.store
            .resolve(
                Some(HELPER),
                Some(&identity("new-session")),
                &|_| None,
                true
            )?
            .cid,
        Some(new.cid)
    );
    Ok(())
}

#[test]
fn helper_is_registered_and_override_is_explicit() -> Result<()> {
    let f = Fixture::new("brit");
    assert_eq!(
        f.registry.invoked(Path::new(HELPER))?,
        ("brit".into(), HELPER.into())
    );
    let release = f.release("fresh");
    f.land(&release)?;
    let candidate = f.path("candidate");
    atomic_bytes(&candidate, b"#!/bin/sh\nexit 0\n", 0o755)?;
    let resolved = f.store.resolve(
        Some(HELPER),
        None,
        &|key| (key == "BRIT_REBASE_RETAINED_BIN").then(|| candidate.display().to_string()),
        true,
    )?;
    assert_eq!(resolved.path, candidate);
    assert_eq!(resolved.source, "candidate");
    Ok(())
}

#[test]
fn published_sha256_transport_preserves_native_identity_and_rejects_tamper() -> Result<()> {
    let f = Fixture::new("brit");
    let source = f.path("published");
    fs::create_dir(&source)?;
    let mut names = f.store.config.bins.clone();
    names.sort();
    // NIST SHA-256("abc"), independent of the native BLAKE3 byte-profile identity.
    let sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let mut checksums = String::new();
    for name in &names {
        atomic_bytes(&source.join(name), b"abc", 0o755)?;
        checksums.push_str(&format!("{sha256}  {name}\n"));
    }
    fs::write(source.join("SHA256SUMS"), checksums)?;
    let options = IngestOptions {
        source_commit: Some(f.commit()),
        tree_clean: Some(true),
        ..Default::default()
    };
    let release = f.store.ingest(&source, &options)?;
    let native_cid = eprfs_core::BlobCid::compute_bytes(b"abc").to_string();
    assert!(release.bins.values().all(|cid| cid == &native_cid));
    assert_eq!(
        fs::read(f.store.artifact(&release.cid)?.join("sums.cbor"))?,
        sums_manifest(&release.bins)?
    );
    assert!(f.store.verified(&release.cid));

    fs::write(source.join("brit"), b"abx")?;
    assert_eq!(
        f.store.ingest(&source, &options).unwrap_err().kind,
        "checksums-mismatch"
    );
    assert!(f.store.verified(&release.cid));
    Ok(())
}

#[test]
fn declared_helper_cannot_be_missing_or_corrupt() -> Result<()> {
    let f = Fixture::new("brit");
    let release = f.release("fresh");
    let options = IngestOptions {
        source_commit: Some(f.commit()),
        tree_clean: Some(true),
        ..Default::default()
    };
    let helper = f.path("fresh").join(HELPER);
    let original = fs::read(&helper)?;
    fs::write(&helper, b"corrupt")?;
    assert_eq!(
        f.store.ingest(&f.path("fresh"), &options).unwrap_err().kind,
        "checksums-mismatch"
    );
    fs::remove_file(&helper)?;
    assert_eq!(
        f.store.ingest(&f.path("fresh"), &options).unwrap_err().kind,
        "checksums-mismatch"
    );
    // The same failures through the published SHA256SUMS transport.
    fs::remove_file(f.path("fresh/sums.cbor"))?;
    let artifact = f.store.artifact(&release.cid)?;
    let sums = release
        .bins
        .iter()
        .filter(|(name, _)| name.as_str() != "rakia")
        .map(|(name, _)| {
            let cid = eprfs_core::BlobCid::compute_raw(&fs::read(artifact.join(name))?);
            let digest = cid
                .as_cid()
                .hash()
                .digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            Ok(format!("{digest}  {name}\n"))
        })
        .collect::<Result<Vec<_>>>()?
        .concat();
    fs::write(f.path("fresh/SHA256SUMS"), sums)?;
    assert_eq!(
        f.store.ingest(&f.path("fresh"), &options).unwrap_err().kind,
        "checksums-mismatch"
    );
    atomic_bytes(&helper, &original, 0o755)?;
    assert_eq!(f.store.ingest(&f.path("fresh"), &options)?.cid, release.cid);
    fs::write(&helper, b"corrupt")?;
    assert_eq!(
        f.store.ingest(&f.path("fresh"), &options).unwrap_err().kind,
        "checksums-mismatch"
    );
    // Losing a different executable never qualifies as historical compatibility.
    fs::remove_file(f.path("fresh/brit-verify"))?;
    assert_eq!(
        f.store.ingest(&f.path("fresh"), &options).unwrap_err().kind,
        "binary-unusable"
    );
    Ok(())
}

#[test]
fn missing_primary_still_requests_bootstrap() -> Result<()> {
    let f = Fixture::new("epr");
    let release = f.release("fresh");
    f.land(&release)?;
    fs::remove_file(f.store.artifact(&release.cid)?.join("epr"))?;
    assert_eq!(
        f.store.resolve_current(None).unwrap_err().kind,
        "head-absent"
    );
    Ok(())
}

#[test]
fn historical_launcher_install_preserves_plain_brit_without_borrowing_helper() -> Result<()> {
    let mut f = Fixture::new("brit");
    let full = f.store.config.bins.clone();
    f.store.config.bins.retain(|bin| bin != HELPER);
    let old = f.release("old");
    f.land(&old)?;
    f.store.config.bins = full;
    atomic_bytes(&f.path("cargo-bin/brit"), b"plain brit", 0o755)?;
    let result = f.install()?;
    let adopted = Path::new(result["adopted"]["brit"].as_str().expect("adopted path"));
    assert_eq!(fs::read(adopted)?, b"plain brit");
    assert!(!adopted.with_file_name(HELPER).exists());
    assert!(f.path("cargo-bin").join(HELPER).is_symlink());
    assert!(f
        .store
        .resolve_current(Some(HELPER))
        .unwrap_err()
        .detail
        .contains(&old.cid));
    assert_eq!(f.store.head("current"), Some(old.cid));
    Ok(())
}

#[test]
fn followed_download_accepts_both_inventories_and_build_requests_four() -> Result<()> {
    for historical in [false, true] {
        let mut f = Fixture::new("brit");
        let full = f.store.config.bins.clone();
        if historical {
            f.store.config.bins.retain(|bin| bin != HELPER);
        }
        let expected = f.release("download");
        f.store.config.bins = full;
        f.state.lock().expect("fixture state").download = Some(f.path("download"));
        let release =
            f.store
                .build_release(f.tmp.path(), &f.commit(), "test", &mut Vec::new(), &|_| {
                    None
                })?;
        assert_eq!(release.cid, expected.cid);
        assert_eq!(release.bins.contains_key(HELPER), !historical);
        let state = f.state.lock().expect("fixture state");
        let build = state
            .calls
            .iter()
            .find(|c| c.program == "cargo" && c.args[0] == "build")
            .expect("source fallback");
        let bins = build
            .args
            .windows(2)
            .filter(|pair| pair[0] == "--bin")
            .map(|pair| pair[1].as_str())
            .collect::<Vec<_>>();
        assert_eq!(bins, ["brit", HELPER, "brit-build-ref", "brit-verify"]);
        let download = state
            .calls
            .iter()
            .find(|c| c.program == "gh")
            .expect("release fallback");
        assert_eq!(
            download.args[2],
            format!("brit-build-{}", &state.head[..12])
        );
    }
    Ok(())
}
