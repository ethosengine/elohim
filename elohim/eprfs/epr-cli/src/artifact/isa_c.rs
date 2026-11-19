//! The declared target's C level, realized by checks at put (backlog steward-device-arrival row
//! 29.2; decision memo "PUT CHECKS"): a v3-default gcc put AVX2 into ring's C and a v2 sibling
//! died with SIGILL running the drawn `epr`.
//!
//! A method that declares `target.isa` and `target.cflags` is accepted only when
//! 1. `target.cflags` is exactly `-march=<target.isa>`, the recorded `CFLAGS_<triple>` is
//!    `target.cflags` (a recorded `CXXFLAGS_<triple>` too) and no other C-family variable is
//!    recorded: string equality, with no model of the cc crate;
//! 2. every `GNU C` producer string in the bytes' `.debug_str` carries the declared `-march`
//!    and no other `-m<isa>` token (`-mno-*`, `-m64` and `-mtune=` are ignored); a bin with no
//!    such string is not applicable when its lock closure holds no `ring`, and fails when it does.
//!
//! Only then is the Produce stamped with one [`SLOT`]`=<target.isa>`, which a sibling draws by.
//! Any other declared target input (`arch`, `os`, `libc`, `sysroot`) is still unrealized (K6).
//!
//! Residuals, recorded rather than solved: the producer string is a ring-unit proxy for the C
//! driver, not a reading of every C object (a C unit built without debug info is invisible); a
//! producer string in `.debug_info` as an inline `DW_FORM_string` is not read; the lock closure is
//! a superset of what the build compiles (features and platforms are not resolved).
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use eprfs_core::MethodDescriptor;
use object::{Object, ObjectSection};

use super::{cargo_project, git_ok, host, is_c_family_var, CRATE_INPUT, DIAGNOSTIC_C_ENV};

/// The Produce slot naming the C baseline put checked the build against.
pub(super) const SLOT: &str = "env:isa-c";

/// The C-bearing crate whose presence in a bin's lock closure makes an unread C level a failure.
const C_CRATE: &str = "ring";

/// The declared target inputs these checks realize.
const REALIZED: [&str; 2] = ["target.isa", "target.cflags"];

const DECLARED: &str = "missing declared-target";

/// What the descriptor declares for its C, once the declaration itself is coherent.
pub(super) struct Declared<'a> {
    /// `target.isa`, the marker's value.
    pub isa: &'a str,
    /// The one `-march=<isa>` token of `target.cflags`.
    pub march: &'a str,
}

/// The descriptor's declared target, checked without reading the bytes: `Ok(None)` when it
/// declares none, `Ok(Some(_))` when the declaration is one these checks realize and the record
/// applies it (check 1), else every reason it is not.
pub(super) fn declared(descriptor: &MethodDescriptor) -> Result<Option<Declared<'_>>, Vec<String>> {
    let inputs = &descriptor.non_path_inputs;
    let targets: Vec<&str> = inputs
        .keys()
        .map(String::as_str)
        .filter(|name| name.starts_with("target."))
        .collect();
    if targets.is_empty() {
        return Ok(None);
    }
    let mut missing = Vec::new();
    let unrealized: Vec<&str> = targets
        .iter()
        .copied()
        .filter(|name| !REALIZED.contains(name))
        .collect();
    if !unrealized.is_empty() {
        missing.push(format!(
            "{DECLARED}: build realization unsupported; the method declares {}, and no build \
             applies it yet (K6)",
            unrealized.join(", ")
        ));
    }
    let isa = inputs.get("target.isa").map(String::as_str);
    let cflags = inputs.get("target.cflags").map(String::as_str);
    let (isa, cflags) = match (isa, cflags) {
        (Some(isa), Some(cflags)) => (isa, cflags),
        (Some(isa), None) => {
            missing.push(format!(
                "{DECLARED}: build realization unsupported; target.isa {isa} declares no \
                 target.cflags, so nothing applies it to the C (K6)"
            ));
            return Err(missing);
        }
        (None, Some(_)) => {
            missing.push(format!(
                "{DECLARED}: build realization unsupported; target.cflags declares no \
                 target.isa, so the C level has no name to record (K6)"
            ));
            return Err(missing);
        }
        (None, None) => return Err(missing),
    };
    // Ruling 2 (L2a rework): the declaration is exactly `-march=<target.isa>`, one token and no
    // other flag, so no record or descriptor can declare an ISA flag beside it, even when every
    // recorded variable agrees with that extra flag.
    let marches: Vec<&str> = cflags
        .split_whitespace()
        .filter(|token| token.starts_with("-march="))
        .collect();
    let march = match marches.as_slice() {
        [march] => Some(*march),
        _ => {
            missing.push(format!(
                "{DECLARED}: target.cflags {cflags:?} names no single -march"
            ));
            None
        }
    };
    if let Some(level) = march.and_then(|m| m.strip_prefix("-march=")) {
        if level != isa {
            missing.push(format!(
                "{DECLARED}: target.cflags declares -march={level}, not target.isa {isa}"
            ));
        }
    }
    let exact = format!("-march={isa}");
    if cflags != exact {
        missing.push(format!(
            "{DECLARED}: target.cflags {cflags:?} is not exactly {exact}; a declared target's C \
             flags are its -march alone"
        ));
    }
    missing.extend(record_applies(descriptor, cflags));
    match march {
        Some(march) if missing.is_empty() => Ok(Some(Declared { isa, march })),
        _ => Err(missing),
    }
}

/// Check 1, the contract (row 29.2, ruled at the L2a review): `CFLAGS_<triple>` and
/// `CXXFLAGS_<triple>`, when recorded, each equal `target.cflags` exactly; any other recorded
/// C-family variable refuses. No C-family variable may select an instruction set other than the
/// declared one: the gate exports both variables with the declared value and the producer records
/// both. `describe` records and `put` re-checks the environment (`is_build_var`), so the record is
/// what the build ran under.
///
/// Extended at the L2a rework (ruling 4): the C family is every C tool and flag cc 1.11.1 reads
/// (`is_c_family_var`: the archiver and ranlib, `HOST_AR`, `TARGET_AR`, `AR_<triple>` in both
/// spellings, `ARFLAGS` and `RANLIBFLAGS` in every form, and cc's own `CC_*` switches), and every
/// one of them refuses here except a diagnostic-only switch (`DIAGNOSTIC_C_ENV`:
/// `CC_ENABLE_DEBUG_OUTPUT`), which changes no byte, is declared by the record's gate export, and
/// is recorded and allowed. A switch that changes what cc builds (`CC_FORCE_DISABLE`,
/// `CRATE_CC_NO_DEFAULTS`) keeps refusing.
fn record_applies(descriptor: &MethodDescriptor, cflags: &str) -> Vec<String> {
    let triple = host::target(descriptor);
    if triple.is_empty() {
        return vec![format!(
            "{DECLARED}: the method names no target triple, so no C flags variable is its own"
        )];
    }
    let underscored = triple.replace('-', "_");
    let c = format!("CFLAGS_{underscored}");
    let cxx = format!("CXXFLAGS_{underscored}");
    let inputs = &descriptor.non_path_inputs;
    let mut missing = Vec::new();
    match inputs.get(&c) {
        Some(value) if value == cflags => {}
        Some(value) => missing.push(format!(
            "{DECLARED}: target.cflags is not applied: the method records {c}={value}, not \
             {cflags}"
        )),
        None => missing.push(format!(
            "{DECLARED}: target.cflags is not applied: the method records no {c}"
        )),
    }
    if let Some(value) = inputs.get(&cxx).filter(|value| *value != cflags) {
        missing.push(format!(
            "{DECLARED}: target.cflags is not applied: the method records {cxx}={value}, not \
             {cflags}"
        ));
    }
    for name in inputs.keys() {
        if is_c_family_var(name)
            && *name != c
            && *name != cxx
            && !DIAGNOSTIC_C_ENV.contains(&name.as_str())
        {
            missing.push(format!(
                "{DECLARED}: the method records the C-family variable {name}; a declared \
                 target's C flags are {c} (and {cxx}) alone"
            ));
        }
    }
    missing
}

/// The record a declared target must agree with, read at the checkout's HEAD.
const POLICY: &str = "genesis/agentic/pool-policy.json";

/// Ruling 3 (L2a rework): one triple source, the compiler's, and the record is the authority, not
/// the descriptor's declaration alone. `declared_targets` in pool-policy.json at the checkout's
/// HEAD (read with this device's declared checkout trust) must hold exactly the descriptor's
/// triple (the one `rustc -vV` named to describe, never a same-architecture neighbour such as
/// `x86_64-unknown-linux-musl` or `-gnux32`), listing the bin's cargo workspace (the directory of
/// the `Cargo.lock` cargo uses for its crate), with the same `isa` and `cflags`. Every reason it
/// does not is returned.
pub(super) fn policy_declares(
    repo: &Path,
    descriptor: &MethodDescriptor,
    declared: &Declared<'_>,
) -> Vec<String> {
    let triple = host::target(descriptor);
    let crate_dir = descriptor
        .non_path_inputs
        .get(CRATE_INPUT)
        .map_or(".", String::as_str);
    let Some(workspace) = cargo_project::lock_path(repo, &repo.join(crate_dir))
        .and_then(|lock| lock.parent().map(Path::to_path_buf))
        .map(|dir| {
            let relative = dir.strip_prefix(repo).unwrap_or(&dir).to_string_lossy();
            if relative.is_empty() {
                ".".to_owned()
            } else {
                relative.replace('\\', "/")
            }
        })
    else {
        return vec![format!(
            "{DECLARED}: {crate_dir} has no Cargo.lock, so its workspace is not one {POLICY} can \
             declare"
        )];
    };
    let output = crate::git::command(repo)
        .args(["show", &format!("HEAD:{POLICY}")])
        .output();
    let text = match output {
        Ok(output) if output.status.success() => output.stdout,
        Ok(output) => {
            return vec![format!(
                "{DECLARED}: {POLICY} does not read at HEAD ({}), so no record declares this \
                 target",
                String::from_utf8_lossy(&output.stderr).trim()
            )]
        }
        Err(e) => return vec![format!("{DECLARED}: git show HEAD:{POLICY}: {e}")],
    };
    let policy: serde_json::Value = match serde_json::from_slice(&text) {
        Ok(policy) => policy,
        Err(e) => return vec![format!("{DECLARED}: {POLICY} at HEAD is not JSON: {e}")],
    };
    let Some(record) = policy["declared_targets"].get(triple) else {
        return vec![format!(
            "{DECLARED}: {POLICY} at HEAD declares no target {triple:?}, the triple the method was \
             built for; a declared target is the record's, not the descriptor's alone"
        )];
    };
    let mut missing = Vec::new();
    let listed = record["workspaces"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w.as_str() == Some(workspace.as_str())));
    if !listed {
        missing.push(format!(
            "{DECLARED}: {POLICY} at HEAD declares {triple} for {}, not the bin's workspace \
             {workspace}",
            record["workspaces"]
        ));
    }
    for (field, value) in [("isa", declared.isa), ("cflags", declared.march)] {
        if record[field].as_str() != Some(value) {
            missing.push(format!(
                "{DECLARED}: {POLICY} at HEAD declares {triple} {field} {}, and the method \
                 declares {value}",
                record[field]
            ));
        }
    }
    missing
}

/// Check 2 over the bytes: every `GNU C` producer string carries `march` and no other ISA token.
/// `Ok(n)` is the number of distinct producer strings read (zero is the caller's to judge).
pub(super) fn producers(bytes: &[u8], march: &str) -> Result<usize, Vec<String>> {
    let file = object::File::parse(bytes).map_err(|e| {
        vec![format!(
            "{DECLARED}: the bytes do not parse as an object: {e}"
        )]
    })?;
    let Some(section) = file.section_by_name(".debug_str") else {
        return Ok(0);
    };
    let data = section
        .uncompressed_data()
        .map_err(|e| vec![format!("{DECLARED}: .debug_str does not read: {e}")])?;
    let strings: BTreeSet<String> = data
        .split(|byte| *byte == 0)
        .filter(|s| s.starts_with(b"GNU C"))
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    let mut missing = Vec::new();
    for producer in &strings {
        let mut marched = false;
        for token in producer.split_whitespace() {
            if token.starts_with("-march=") {
                marched = true;
                if token != march {
                    missing.push(format!(
                        "{DECLARED}: the C is not at {march}: `{producer}` carries {token}"
                    ));
                }
            } else if token.starts_with("-m")
                && token != "-m64"
                && !token.starts_with("-mno-")
                && !token.starts_with("-mtune=")
            {
                missing.push(format!(
                    "{DECLARED}: the C is not at {march} alone: `{producer}` carries {token}"
                ));
            }
        }
        if !marched {
            missing.push(format!(
                "{DECLARED}: the C level is unread: `{producer}` carries no -march"
            ));
        }
    }
    if missing.is_empty() {
        Ok(strings.len())
    } else {
        Err(missing)
    }
}

/// Whether the descriptor's bin links [`C_CRATE`]: its package (the crate's `Cargo.toml` at
/// HEAD) reaches it through the dependency edges of the lock at HEAD.
pub(super) fn links_c_crate(repo: &Path, descriptor: &MethodDescriptor) -> Result<bool, String> {
    let crate_dir = descriptor
        .non_path_inputs
        .get(CRATE_INPUT)
        .map_or(".", String::as_str);
    let crate_path = repo.join(crate_dir);
    let at_head = |path: &Path| {
        let relative = path.strip_prefix(repo).unwrap_or(path).to_string_lossy();
        git_ok(repo, &["show", &format!("HEAD:{relative}")])
            .map_err(|e| format!("{relative} does not read at HEAD: {e}"))
    };
    let manifest: toml::Value = at_head(&crate_path.join("Cargo.toml"))?
        .parse()
        .map_err(|e| format!("{crate_dir}/Cargo.toml is not TOML: {e}"))?;
    let package = manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{crate_dir}/Cargo.toml names no package"))?;
    let lock_path = cargo_project::lock_path(repo, &crate_path)
        .ok_or_else(|| format!("{crate_dir} has no Cargo.lock"))?;
    let lock: toml::Value = at_head(&lock_path)?
        .parse()
        .map_err(|e| format!("the Cargo.lock is not TOML: {e}"))?;
    lock_closure_holds(&lock, package, C_CRATE)
}

/// Whether `needle` is in the dependency closure of the local package `root` in `lock`. An edge
/// is `name`, `name version` or `name version (source)`; one that names no version reaches every
/// locked package of that name, so the closure can only over-state.
pub(super) fn lock_closure_holds(
    lock: &toml::Value,
    root: &str,
    needle: &str,
) -> Result<bool, String> {
    let packages: Vec<&toml::Value> = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .map(|packages| packages.iter().collect())
        .unwrap_or_default();
    let field = |package: &toml::Value, name: &str| {
        package
            .get(name)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, package) in packages.iter().enumerate() {
        if let Some(name) = field(package, "name") {
            by_name.entry(name).or_default().push(index);
        }
    }
    let mut pending: Vec<usize> = by_name
        .get(root)
        .into_iter()
        .flatten()
        .copied()
        .filter(|index| packages[*index].get("source").is_none())
        .collect();
    if pending.is_empty() {
        return Err(format!("the Cargo.lock locks no local package {root}"));
    }
    let mut seen = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if !seen.insert(index) {
            continue;
        }
        let package = packages[index];
        if field(package, "name").as_deref() == Some(needle) {
            return Ok(true);
        }
        for edge in package
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
        {
            let mut parts = edge.split_whitespace();
            let name = parts.next().unwrap_or_default();
            let version = parts.next();
            pending.extend(
                by_name
                    .get(name)
                    .into_iter()
                    .flatten()
                    .copied()
                    .filter(|candidate| {
                        version.is_none()
                            || field(packages[*candidate], "version").as_deref() == version
                    }),
            );
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "isa_c_tests.rs"]
mod tests;
