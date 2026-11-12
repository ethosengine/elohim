//! Archive CLI adapters. Packing records payload observations; putting shares the normal
//! custody/Produce tail after manifest validation, without claiming an ELF linker witness.
use super::*;

pub(super) fn run(verb: Option<&str>, options: &Options) -> ArtifactResult<ExitCode> {
    match verb {
        Some("pack") => {
            let from = options.from.as_ref().ok_or_else(|| {
                ArtifactError::InvalidArguments("sysroot pack requires --from".into())
            })?;
            let triple = Options::required(&options.triple, "--triple")?;
            let out = options.out.as_ref().ok_or_else(|| {
                ArtifactError::InvalidArguments("sysroot pack requires --out".into())
            })?;
            let (bytes, manifest) = sysroot::pack(from, triple)?;
            let parent = out
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            std::fs::create_dir_all(parent)?;
            let mut temp = tempfile::NamedTempFile::new_in(parent)?;
            std::io::Write::write_all(&mut temp, &bytes)?;
            temp.as_file().sync_all()?;
            temp.persist(out).map_err(|e| e.error)?;
            if options.json {
                println!(
                    "{}",
                    serde_json::json!({"output": BlobCid::compute_bytes(&bytes), "manifest": manifest.cid()?, "bytes": bytes.len()})
                );
            } else {
                println!("{}", BlobCid::compute_bytes(&bytes));
            }
            Ok(ExitCode::SUCCESS)
        }
        Some("put") => put(options),
        _ => Err(ArtifactError::InvalidArguments(
            "sysroot requires pack or put".into(),
        )),
    }
}

fn put(options: &Options) -> ArtifactResult<ExitCode> {
    let path = options
        .archive
        .as_ref()
        .ok_or_else(|| ArtifactError::InvalidArguments("sysroot put requires --archive".into()))?;
    let bytes = Bytes::from(sysroot::limits::read_archive(path)?);
    let manifest = sysroot::verify(&bytes)?;
    let input = manifest.cid()?;
    let project = options.project.as_deref().unwrap_or("sysroot");
    // This recipe is compiled into the packer; a change to serialization or evidence extraction
    // changes the method, even if it happens to produce the same payload on one input.
    let recipe = [
        include_bytes!("sysroot.rs").as_slice(),
        include_bytes!("sysroot_evidence.rs").as_slice(),
        include_bytes!("sysroot_limits.rs").as_slice(),
    ]
    .concat();
    let descriptor = MethodDescriptor {
        name: "sysroot-pack".into(),
        tools: BTreeMap::new(),
        flags: Vec::new(),
        params: None,
        exec: None,
        step: Some(format!("sysroot-pack:{project}")),
        embedder: None,
        view: Vec::new(),
        non_path_inputs: [
            ("sysroot.manifest".into(), input.to_string()),
            (
                "sysroot.recipe".into(),
                BlobCid::compute_bytes(&recipe).to_string(),
            ),
            ("sysroot.triple".into(), manifest.triple.clone()),
        ]
        .into(),
        deterministic: true,
    };
    let method = descriptor.cid().map_err(unrecorded)?;
    let derivation = DerivationKey {
        method: method.clone(),
        input: KeyInput::Raw(input.clone()),
    };
    let head = git_ok(&options.repo, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    if options
        .commit
        .as_ref()
        .is_some_and(|commit| commit != &head)
    {
        return Err(unrecorded("HEAD moved before recording"));
    }
    let key = ClosureKey {
        method,
        input,
        key_cid: key_cid(&derivation).map_err(unrecorded)?,
        basis: "basis-key:sysroot-manifest",
        head,
        dirty: Vec::new(),
        key: derivation,
        input_lines: None,
    };
    record_artifact(options, &descriptor, &key, bytes, Some(&manifest), &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_boundary_refuses_both_method_validator_mismatches_before_any_write() {
        let root = tempfile::tempdir().unwrap();
        let (bytes, manifest) = sysroot::pack(root.path(), "test").unwrap();
        for (method, validated) in [("mesh-build", Some(&manifest)), ("sysroot-pack", None)] {
            let descriptor = MethodDescriptor {
                name: method.into(),
                tools: BTreeMap::new(),
                flags: vec![],
                params: None,
                exec: None,
                step: None,
                embedder: None,
                view: vec![],
                non_path_inputs: BTreeMap::new(),
                deterministic: true,
            };
            let method = descriptor.cid().unwrap();
            let input = manifest.cid().unwrap();
            let derivation = DerivationKey {
                method: method.clone(),
                input: KeyInput::Raw(input.clone()),
            };
            let key = ClosureKey {
                method,
                input,
                key_cid: key_cid(&derivation).unwrap(),
                basis: "test",
                head: "test".into(),
                dirty: vec![],
                key: derivation,
                input_lines: None,
            };
            // No repository or signer exists here: pairing must refuse before inspecting or writing either.
            let options = Options {
                repo: root.path().to_path_buf(),
                ..Options::default()
            };
            let error = record_artifact(
                &options,
                &descriptor,
                &key,
                Bytes::from(bytes.clone()),
                validated,
                &[],
            )
            .unwrap_err();
            assert!(
                error.to_string().contains("method/validator mismatch"),
                "{error}"
            );
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
    }
}
