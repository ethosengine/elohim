//! Addressed discipline meaning; repository policy supplies local standing.
use eprfs_core::BlobCid;
use eprfs_meta::portable_policy::{verify_policy, PortablePolicy, PORTABLE_POLICY_FORMAT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path},
};

pub const MAX_OBJECT_BYTES: usize = eprfs_meta::MAX_REGISTRY_BYTES;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub cid: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Discipline {
    pub policy: ObjectRef,
    pub guide: ObjectRef,
    pub method: ObjectRef,
}

pub fn relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn canonical(value: &Value) -> Result<Vec<u8>, String> {
    eprfs_meta::canonical_json(&serde_yaml::to_value(value).map_err(|e| e.to_string())?)
        .map(String::into_bytes)
        .map_err(|e| e.to_string())
}

fn terms() -> Value {
    json!({
        "automatic": {
            "rules": ["RS-GOV-04"],
            "coverage": "Rust allow/expect attributes, including inactive cfg branches and literal macro output",
            "rationale": "nonempty literal reason string; meaning requires review",
            "enrollment": "no-new findings immediately; per-touch paydown after policy reaches comparison baseline",
            "baseline": "repository-selected Git merge base; committed checks use committed policy and objects",
            "noNew": "finding identity counts cannot increase or trade one violation for another",
            "paydown": "each touched crate with baseline debt removes at least one finding",
            "scopeTransfer": "matching parsed rule/scope/attribute debt moved into changed excluded source is not paydown; unrelated additions and unparseable excluded fixtures remain outside enforcement",
            "strict": "explicit directory binding requires zero findings and cannot silently weaken",
            "exceptions": "explained allowances still require compiler, feature and clippy checks"
        },
        "review": {
            "guide": "applies as reviewed developer guidance beyond automated rule coverage",
            "rationale": "human or explicitly delegated review assesses necessity, narrowness and meaningful reasons",
            "limits": "no automatic semantic proof of reasons, Cargo/CLI suppressions, API quality or upstream ownership",
            "authority": "receiving repository chooses scope and provenance; discovery never grants standing or executes code"
        }
    })
}

/// An installed checker identifies the source and dependency resolution it implements.
/// Cargo.lock pins registry dependency bytes by checksum.
pub fn method_bytes() -> Result<Vec<u8>, String> {
    let sources: [(&str, &[u8]); 9] = [
        ("Cargo.toml", include_bytes!("../Cargo.toml")),
        (
            ".cargo/config.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../.cargo/config.toml"
            )),
        ),
        ("Cargo.lock", include_bytes!("../Cargo.lock")),
        ("src/lib.rs", include_bytes!("lib.rs")),
        ("src/main.rs", include_bytes!("main.rs")),
        ("src/repository.rs", include_bytes!("repository.rs")),
        ("src/discipline.rs", include_bytes!("discipline.rs")),
        ("scan/Cargo.toml", include_bytes!("../scan/Cargo.toml")),
        ("scan/src/lib.rs", include_bytes!("../scan/src/lib.rs")),
    ];
    let inventory: BTreeMap<_, _> = sources
        .into_iter()
        .map(|(name, bytes)| (name, BlobCid::compute_raw(bytes)))
        .collect();
    canonical(&json!({
        "format": "epr-rust-style-method@1",
        "checker": "elohim-rust-style",
        "version": env!("CARGO_PKG_VERSION"),
        "sources": inventory,
        "terms": terms()
    }))
}

fn policy_bytes(guide: &str, method: &str) -> Result<Vec<u8>, String> {
    let semantics = serde_yaml::to_value(json!({
        "id": "rust-developer-discipline", "version": 1,
        "rules": ["RS-GOV-04"], "guide": guide, "method": method,
        "terms": terms()
    }))
    .map_err(|e| e.to_string())?;
    PortablePolicy {
        format: PORTABLE_POLICY_FORMAT.into(),
        semantics: semantics.as_mapping().ok_or("invalid semantics")?.clone(),
    }
    .canonical_bytes()
    .map_err(|e| e.to_string())
}

impl Discipline {
    pub fn objects(&self) -> [&ObjectRef; 3] {
        [&self.policy, &self.guide, &self.method]
    }

    /// Verify before scanning source; no object supplies code to execute.
    pub fn verify(&self, files: &BTreeMap<String, String>) -> Result<(), String> {
        for object in self.objects() {
            if !relative_path(&object.path) {
                return Err("discipline object path must stay inside repository".into());
            }
            let bytes = files
                .get(&object.path)
                .ok_or_else(|| format!("missing discipline object {}", object.path))?
                .as_bytes();
            if bytes.len() > MAX_OBJECT_BYTES {
                return Err("discipline object exceeds size limit".into());
            }
            let expected = BlobCid::parse(&object.cid).map_err(|e| e.to_string())?;
            if BlobCid::compute_raw(bytes) != expected {
                return Err(format!("discipline object CID mismatch: {}", object.path));
            }
        }
        if files
            .get(&self.guide.path)
            .is_none_or(|guide| guide.trim().is_empty())
        {
            return Err("discipline guide must not be empty".into());
        }
        let policy = files
            .get(&self.policy.path)
            .ok_or("missing discipline policy")?;
        verify_policy(
            &BlobCid::parse(&self.policy.cid).map_err(|e| e.to_string())?,
            policy.as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        if policy.as_bytes() != policy_bytes(&self.guide.cid, &self.method.cid)? {
            return Err("unsupported discipline meaning or guide/method reference mismatch".into());
        }
        let method = files
            .get(&self.method.path)
            .ok_or("missing discipline method")?;
        if method.as_bytes() != method_bytes()? {
            return Err("discipline method differs from this installed checker; select the pinned checker explicitly".into());
        }
        Ok(())
    }
}

/// Generate an offer only. The caller must independently author local Policy standing.
pub fn bundle(guide: &[u8]) -> Result<(Discipline, BTreeMap<String, Vec<u8>>), String> {
    if guide.is_empty() || guide.len() > MAX_OBJECT_BYTES || std::str::from_utf8(guide).is_err() {
        return Err("guide must be nonempty bounded UTF-8 text".into());
    }
    let method = method_bytes()?;
    let policy = policy_bytes(
        &BlobCid::compute_raw(guide).to_string(),
        &BlobCid::compute_raw(&method).to_string(),
    )?;
    let mut objects = BTreeMap::new();
    let mut object = |bytes: &[u8], suffix: &str| {
        let cid = BlobCid::compute_raw(bytes).to_string();
        let path = format!("objects/{cid}.{suffix}");
        objects.insert(path.clone(), bytes.to_vec());
        ObjectRef { cid, path }
    };
    let discipline = Discipline {
        policy: object(&policy, "json"),
        guide: object(guide, "md"),
        method: object(&method, "json"),
    };
    Ok((discipline, objects))
}

pub fn export(guide: &Path, out: &Path) -> Result<Discipline, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(guide)
        .map_err(|e| e.to_string())?
        .take(MAX_OBJECT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let (discipline, objects) = bundle(&bytes)?;
    fs::create_dir_all(out.join("objects")).map_err(|e| e.to_string())?;
    if !fs::symlink_metadata(out.join("objects"))
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("objects directory must not be a symlink".into());
    }
    for (path, bytes) in objects {
        let path = out.join(path);
        match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
        {
            Ok(mut file) => file.write_all(&bytes).map_err(|e| e.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !fs::symlink_metadata(&path)
                    .map_err(|e| e.to_string())?
                    .is_file()
                    || fs::metadata(&path).map_err(|e| e.to_string())?.len() != bytes.len() as u64
                    || fs::read(&path).map_err(|e| e.to_string())? != bytes
                {
                    return Err("existing export object differs or is not regular".into());
                }
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let mut file = tempfile::NamedTempFile::new_in(out).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &discipline).map_err(|e| e.to_string())?;
    file.persist(out.join("discipline.json"))
        .map_err(|e| e.to_string())?;
    Ok(discipline)
}
