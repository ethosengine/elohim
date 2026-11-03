//! Opt-in portable export. Legacy canon-lift artifacts and pins remain untouched.

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use eprfs_core::BlobCid;
use eprfs_meta::portable_policy::{split_registry_row, verify_policy};
use serde::Serialize;

use super::{load_registry, CanonLiftError, CanonLiftResult, REGISTRIES};

const OUT_DIR: &str = ".eprfs/exports/portable-canon-v2";

fn invalid(message: impl Into<String>) -> CanonLiftError {
    CanonLiftError::InvalidArguments(message.into())
}

fn retain_object(path: &Path, bytes: &[u8]) -> CanonLiftResult<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !fs::symlink_metadata(path)?.file_type().is_file() {
                return Err(invalid("portable object is not a regular file"));
            }
            let mut held = Vec::new();
            fs::File::open(path)?
                .take(eprfs_meta::MAX_REGISTRY_BYTES as u64 + 1)
                .read_to_end(&mut held)?;
            if held != bytes {
                return Err(invalid(
                    "existing portable object differs; preserve it for diagnosis",
                ));
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportRow {
    registry: String,
    id: String,
    version: u64,
    legacy_cid: String,
    content_cid: BlobCid,
    content_path: String,
    standing: eprfs_meta::portable_policy::PolicyStanding,
}

/// Export is an offer. A receiving scope must authorize its own adoption separately.
pub fn run(args: &[String]) -> CanonLiftResult<ExitCode> {
    let mut root = PathBuf::from(".");
    let mut out = None;
    let mut expected = None;
    let mut file = None;
    let mut write = false;
    let mut portable = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--portable" => portable = true,
            "--write" => write = true,
            "--json" => {}
            "--root" => {
                root = PathBuf::from(iter.next().ok_or_else(|| invalid("--root needs a path"))?)
            }
            "--out" => {
                out = Some(PathBuf::from(
                    iter.next().ok_or_else(|| invalid("--out needs a path"))?,
                ))
            }
            "--verify-portable" => {
                expected = Some(
                    iter.next()
                        .ok_or_else(|| invalid("--verify-portable needs a CID"))?,
                )
            }
            "--file" => {
                file = Some(PathBuf::from(
                    iter.next().ok_or_else(|| invalid("--file needs a path"))?,
                ))
            }
            _ => return Err(invalid(format!("unknown portable argument: {arg}"))),
        }
    }
    if let Some(expected) = expected {
        if portable || write || out.is_some() {
            return Err(invalid("verification cannot export or write"));
        }
        let file = file.ok_or_else(|| invalid("--verify-portable requires --file"))?;
        let expected = BlobCid::parse(expected).map_err(|error| invalid(error.to_string()))?;
        let mut bytes = Vec::new();
        fs::File::open(file)?
            .take(eprfs_meta::MAX_REGISTRY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let policy =
            verify_policy(&expected, &bytes).map_err(|error| invalid(error.to_string()))?;
        println!(
            "{}",
            serde_json::to_string(
                &serde_json::json!({"verified": true, "cid": expected, "policy": policy})
            )?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if !portable || file.is_some() {
        return Err(invalid(
            "export requires --portable; --file requires --verify-portable",
        ));
    }
    let mut exported = Vec::new();
    let mut objects = Vec::new();
    for (rel, version_key, rows_key) in REGISTRIES {
        for row in load_registry(&root, rel, version_key, rows_key)? {
            if row.pin_state() != "verified" {
                return Err(invalid(format!(
                    "{}@{} legacy pin is {}",
                    row.id,
                    row.version,
                    row.pin_state()
                )));
            }
            let (policy, standing) =
                split_registry_row(&row.raw).map_err(|error| invalid(error.to_string()))?;
            let bytes = policy
                .canonical_bytes()
                .map_err(|error| invalid(error.to_string()))?;
            let cid = policy.cid().map_err(|error| invalid(error.to_string()))?;
            let content_path = format!("objects/{cid}.json");
            objects.push((content_path.clone(), bytes));
            exported.push(ExportRow {
                registry: rel.into(),
                id: row.id,
                version: row.version,
                legacy_cid: row.cid,
                content_cid: cid,
                content_path,
                standing,
            });
        }
    }
    exported.sort_by(|a, b| (&a.registry, &a.id, a.version).cmp(&(&b.registry, &b.id, b.version)));
    let document = serde_json::to_string_pretty(&serde_json::json!({
        "format": "epr-portable-canon@2",
        "authority": "source standing is an offer, not receiving-space adoption",
        "policies": exported
    }))?;
    if write {
        let dir = out.unwrap_or_else(|| root.join(OUT_DIR));
        fs::create_dir_all(dir.join("objects"))?;
        if !fs::symlink_metadata(dir.join("objects"))?
            .file_type()
            .is_dir()
        {
            return Err(invalid("portable objects directory must not be a symlink"));
        }
        for (path, bytes) in objects {
            retain_object(&dir.join(path), &bytes)?;
        }
        let index = dir.join("portable-canon.json");
        match fs::symlink_metadata(&index) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(invalid(
                    "portable index must be a regular file, not a symlink",
                ));
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        // Atomic replacement cannot overwrite a symlink or hard-link target,
        // even if another writer changes the index after the metadata check.
        let mut temporary = tempfile::NamedTempFile::new_in(&dir)?;
        writeln!(temporary, "{document}")?;
        temporary.persist(index).map_err(|error| error.error)?;
    }
    println!("{document}");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temporary root");
        fs::create_dir_all(dir.path().join(".claude/epr-meta")).expect("registry directory");
        let mut row: serde_yaml::Mapping = serde_yaml::from_str(
            "id: rule\nversion: 1\nclass: deny\nbinding: binding-local\nwhy: preserve semantics\n",
        )
        .expect("row");
        let body = eprfs_meta::canonical_body(&row).expect("legacy body");
        let pin = format!(
            "sha256:{}",
            eprfs_meta::hex_lower(elohim_epr::cid::compute_cid(&body).hash().digest())
        );
        row.insert(
            serde_yaml::Value::String("contentHash".into()),
            serde_yaml::Value::String(pin),
        );
        fs::write(
            dir.path().join(".claude/epr-meta/policies.yaml"),
            format!(
                "epr-meta-policies-version: 1\npolicies:\n{}",
                serde_yaml::to_string(&vec![row]).expect("rows")
            ),
        )
        .expect("registry");
        fs::write(
            dir.path().join(".claude/epr-meta/concerns.yaml"),
            "concern-canon-version: 1\nconcerns: []\n",
        )
        .expect("concerns");
        dir
    }

    #[cfg(unix)]
    #[test]
    fn portable_index_symlink_cannot_overwrite_legacy_registry() {
        let source = source();
        let out = tempfile::tempdir().expect("export root");
        let registry = source.path().join(".claude/epr-meta/policies.yaml");
        let before = fs::read(&registry).expect("registry bytes");
        std::os::unix::fs::symlink(&registry, out.path().join("portable-canon.json"))
            .expect("index symlink");
        assert!(run(&[
            "--portable".into(),
            "--write".into(),
            "--root".into(),
            source.path().display().to_string(),
            "--out".into(),
            out.path().display().to_string(),
        ])
        .is_err());
        assert_eq!(fs::read(registry).expect("preserved registry"), before);
        assert!(fs::symlink_metadata(out.path().join("portable-canon.json"))
            .expect("preserved link")
            .file_type()
            .is_symlink());
    }

    #[test]
    fn portable_roundtrip_keeps_legacy_outputs_and_pins() {
        let source = source();
        let legacy = super::super::project(source.path()).expect("legacy projection");
        let out = tempfile::tempdir().expect("export root");
        let args = vec![
            "--portable".into(),
            "--write".into(),
            "--root".into(),
            source.path().display().to_string(),
            "--out".into(),
            out.path().display().to_string(),
        ];
        assert_eq!(run(&args).expect("export"), ExitCode::SUCCESS);
        assert_eq!(run(&args).expect("repeat export"), ExitCode::SUCCESS);
        assert_eq!(
            legacy.files,
            super::super::project(source.path())
                .expect("unchanged legacy projection")
                .files
        );
        let index: serde_json::Value = serde_json::from_slice(
            &fs::read(out.path().join("portable-canon.json")).expect("index"),
        )
        .expect("index JSON");
        let row = &index["policies"][0];
        let cid = row["contentCid"].as_str().expect("CID");
        let file = out.path().join(row["contentPath"].as_str().expect("path"));
        assert_eq!(
            run(&[
                "--verify-portable".into(),
                cid.into(),
                "--file".into(),
                file.display().to_string()
            ])
            .expect("standalone verification"),
            ExitCode::SUCCESS
        );
        fs::write(&file, "tampered").expect("damage fixture");
        assert!(
            run(&args).is_err(),
            "export must preserve tampered objects for diagnosis"
        );
        assert_eq!(
            fs::read_to_string(file).expect("retained bytes"),
            "tampered"
        );
    }

    #[test]
    fn malformed_address_is_not_a_policy() {
        assert!(run(&[
            "--verify-portable".into(),
            "not-a-cid".into(),
            "--file".into(),
            "unused".into()
        ])
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn immutable_object_does_not_follow_symlink() {
        let dir = tempfile::tempdir().expect("temporary root");
        let target = dir.path().join("target");
        fs::write(&target, "held").expect("target");
        let object = dir.path().join("object");
        std::os::unix::fs::symlink(&target, &object).expect("fixture link");
        assert!(retain_object(&object, b"replacement").is_err());
        assert_eq!(
            fs::read_to_string(target).expect("target unchanged"),
            "held"
        );
    }
}
