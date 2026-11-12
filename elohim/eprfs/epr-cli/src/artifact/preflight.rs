//! §7.9 admission before executable recording. This ephemeral result is rebuilt from local
//! evidence; it is neither a new authority nor a persisted verdict.
use super::*;
use object::Object;

// The unconditional declared gap: K12 is blocked on H12. Dependency gaps are detected
// separately by cargo_project. Never claim env:reach from this observation.
pub(super) const REACH: &str = "reach: pending, not enforced";
const COMPILER: &str = "missing build-inputs: compiler/linker inputs incomplete";
const REQUIRES: &str = "missing Requires: unsupported artifact format or architecture";

pub(super) struct Prepared {
    pub key: ClosureKey,
    pub descriptor: MethodDescriptor,
    pub bytes: Bytes,
    pub gaps: Vec<String>,
}

pub(super) fn refuse(mut missing: Vec<String>) -> ArtifactResult<()> {
    missing.sort();
    missing.dedup();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(unrecorded(missing.join("\n")))
    }
}

pub(super) fn repository_scope(options: &Options) -> ArtifactResult<cid::Cid> {
    use crate::flow::repository::{self, DECLARATION};
    let agent = if options.repo.join(DECLARATION).exists() {
        repository::repository_agent(&options.repo)
    } else if let Some(name) = &options.repository {
        let agent = if name.starts_with("repo:") {
            name.clone()
        } else {
            format!("repo:{name}")
        };
        let declaration = serde_json::json!({"version": 1, "agent": agent});
        repository::parse_repository_agent(Path::new("--repository"), &declaration.to_string())
    } else {
        return Err(unrecorded(
            "missing attribution: repository identity (.epr-meta/repository.yaml or --repository)",
        ));
    }
    .map_err(|e| unrecorded(format!("missing attribution: repository identity: {e}")))?;
    repository::scope_of(&agent).map_err(unrecorded)
}

/// Evaluate every applicable item, with no attestation, payload-store open or flow-store write.
/// Independent checks continue after a failure; dependent checks report the absent prerequisite.
pub(super) fn check(options: &Options, descriptor: &MethodDescriptor) -> ArtifactResult<Prepared> {
    let mut missing = Vec::new();
    let mut gaps = vec!["reach".to_string()];
    if descriptor.name != MESH_BUILD {
        // No adapter proves even conditional applicability for an unknown recipe. An ELF alone
        // cannot establish its recipe, family, target realization or dispatch provenance.
        for item in [
            "missing recipe: unsupported method/validator mismatch",
            "missing recipe.view: unsupported recipe kind",
            "missing recipe.source-lock: Cargo.lock (unsupported recipe kind)",
            "missing recipe.source-commit: unsupported recipe kind",
            "missing build-inputs: compiler/linker inputs incomplete",
            "missing family: unsupported recipe kind",
            "missing Requires: unsupported artifact format or architecture (unsupported recipe kind)",
            "missing dispatch-evidence: requested narrowing is unproved (unsupported recipe kind)",
            "missing declared-target: build realization unsupported",
        ] { missing.push(item.into()); }
    } else {
        gaps = cargo_project::checklist(&options.repo, descriptor, &mut missing);
        let build_scripts = gaps.iter().any(|gap| gap == cargo_project::SCRIPTS_GAP);
        let watched_absent =
            compiler_inputs::check(options, descriptor, build_scripts, &mut missing);
        for path in watched_absent {
            eprintln!(
                "{}: watched-but-absent input: {}",
                cargo_project::SCRIPTS_GAP,
                path.display()
            );
        }
    }
    print_gaps(&gaps);
    if descriptor.step
        != options
            .project
            .as_ref()
            .map(|p| format!("{}:{p}", descriptor.name))
    {
        missing.push(format!(
            "missing recipe: the descriptor's step is {:?}, not {}:{}",
            descriptor.step,
            descriptor.name,
            options.project.as_deref().unwrap_or("")
        ));
    }
    let declared: Vec<_> = descriptor
        .non_path_inputs
        .keys()
        .filter(|n| n.starts_with("target."))
        .collect();
    if !declared.is_empty() {
        missing.push(format!("missing declared-target: build realization unsupported; the method declares {}, and no build applies a declared target yet (K6)", declared.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }
    if let Err(e) = check_build_env(descriptor) {
        missing.push(format!("{COMPILER}: {e}"));
    }
    let selection = clean_selection(&options.repo, descriptor);
    match &selection {
        Ok((key, _, effective)) => {
            if Some(key.input.to_string()).as_deref() != options.input.as_deref() {
                missing.push(format!("missing build-inputs: the closure moved during the build: it was {}, it is {} now", options.input.as_deref().unwrap_or("absent"), key.input));
            }
            if options
                .commit
                .as_ref()
                .is_some_and(|commit| commit != &key.head)
            {
                missing.push("missing recipe.source-commit: HEAD moved during the build".into());
            }
            if effective.family_cid().is_err() {
                missing.push("missing family: descriptor does not encode".into());
            }
        }
        Err(e) => missing.push(format!("{COMPILER}: {e}")),
    }
    let bytes_path = options.bytes.as_deref().unwrap_or_else(|| Path::new(""));
    let bytes = std::fs::read(bytes_path);
    match &bytes {
        Ok(bytes) => {
            if !object::File::parse(bytes.as_slice()).is_ok_and(|f| {
                f.format() == object::BinaryFormat::Elf
                    && f.architecture() == object::Architecture::X86_64
            }) {
                missing.push(REQUIRES.into());
            }
            if let Ok((key, selections, _)) = &selection {
                link_inputs(
                    key.input_lines.as_deref().unwrap_or(""),
                    bytes,
                    &mut missing,
                );
                if let Err(e) = check_linker(descriptor, selections, bytes) {
                    missing.push(format!("{COMPILER}: {e}"));
                }
            }
            if let Err(e) =
                requires::documents(bytes, bytes_path, options.requires.as_deref(), false)
            {
                let label = if e.to_string().contains("dispatch narrowing") {
                    "missing dispatch-evidence: requested narrowing is unproved"
                } else {
                    REQUIRES
                };
                missing.push(format!("{label}: {e}"));
            }
        }
        Err(e) => missing.push(format!("{REQUIRES}: {}: {e}", bytes_path.display())),
    }
    if let Err(e) = repository_scope(options) {
        missing.push(e.to_string().trim_start_matches("unrecorded: ").into());
    }
    if let Err(e) = device_key::resolve_path().and_then(|p| DeviceKey::load(&p)) {
        missing.push(format!("missing attribution: device key: {e}"));
    }
    match brit_bin() {
        Err(e) => missing.push(format!("missing attribution: brit-build-ref: {e}")),
        Ok(_) => {
            match brit(&options.repo, &["signer", "--json"])
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).map_err(Into::into))
            {
                Ok(signer) if signer["agentId"].as_str().is_some_and(|s| !s.is_empty()) => {}
                Ok(_) => missing.push("missing attribution: brit signer (agentId is null)".into()),
                Err(e) => missing.push(format!("missing attribution: brit signer: {e}")),
            }
        }
    }
    if let Err(e) = destinations::brit(&options.repo) {
        missing.push(format!("missing attribution: brit destination: {e}"));
    }
    let peer_options = peer::Options {
        root: options.repo.clone(),
        payload_store: options.payload_store.clone(),
        ..Default::default()
    };
    match peer::payload_store_dir(&peer_options) {
        Ok(dir) => {
            if let Err(e) = destinations::payload(&dir) {
                missing.push(e.to_string());
            }
            if let Err(e) = served_store_matches(&dir) {
                missing.push(e.to_string());
            }
        }
        Err(e) => missing.push(format!("missing recipe custody: {e}")),
    }
    if let Err(e) = peer::served_root(&options.repo)
        .map_err(unrecorded)
        .and_then(|root| destinations::flow(&root))
    {
        missing.push(format!("missing recipe custody: {e}"));
    }
    refuse(missing)?;
    let (key, _, descriptor) = selection?;
    Ok(Prepared {
        key,
        descriptor,
        bytes: Bytes::from(bytes?),
        gaps,
    })
}

pub(super) fn print_gaps(gaps: &[String]) {
    for gap in gaps.iter().filter(|g| *g != "reach") {
        eprintln!("{gap}: pending, not enforced");
    }
}

fn link_inputs(lines: &str, bytes: &[u8], missing: &mut Vec<String>) {
    let kind = object::File::parse(bytes).ok().map(|f| f.kind());
    let links: BTreeMap<_, _> = lines
        .lines()
        .filter_map(|l| l.strip_prefix("link:").and_then(|l| l.split_once('\t')))
        .collect();
    if links.is_empty() {
        missing.push(COMPILER.into());
    }
    for (name, evidence) in links {
        // ELF ET_DYN is the PIE startup path (Scrt1), ET_EXEC is the non-PIE path
        // (crt1). Only the alternate entry object is demonstrably inapplicable.
        if (name == "crt1.o" && kind == Some(object::ObjectKind::Dynamic))
            || (name == "Scrt1.o" && kind == Some(object::ObjectKind::Executable))
        {
            continue;
        }
        if !evidence
            .split_once('\t')
            .is_some_and(|(path, cid)| Path::new(path).is_absolute() && BlobCid::parse(cid).is_ok())
        {
            missing.push(format!(
                "missing build-inputs: required link input unresolved: {name}"
            ));
        }
    }
}
