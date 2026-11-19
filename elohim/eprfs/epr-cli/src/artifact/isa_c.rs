//! The declared target's C level, realized by checks at put (backlog steward-device-arrival row
//! 29.2; decision memo "PUT CHECKS"): a v3-default gcc put AVX2 into ring's C and a v2 sibling
//! died with SIGILL running the drawn `epr`.
//!
//! A method that declares `target.isa` and `target.cflags` is accepted only when
//! 1. `target.cflags` is exactly `-march=<target.isa>`, the recorded `CFLAGS_<triple>` is
//!    `target.cflags` (a recorded `CXXFLAGS_<triple>` too) and no other C-family variable is
//!    recorded: string equality, with no model of the cc crate;
//! 2. pool-policy.json `declared_targets` at the checkout's HEAD declares exactly the method's
//!    triple, for the bin's workspace, with the same `isa` and `cflags` ([`policy_declares`]);
//! 3. the build that produced the bytes shows, in its own compile commands, every cc-built crate
//!    at the declared `-march` alone, or a named exception (`isa_c_evidence`, from the cargo
//!    `--message-format=json` lines put is handed as `--build-messages`). Strings in the bytes
//!    decide nothing: one acceptable producer string once passed the whole binary, and bundled
//!    SQLite leaves none (L2a review, ruling 1).
//!
//! Only then is the Produce stamped with one [`SLOT`]`=<target.isa>`, which a sibling draws by.
//! Any other declared target input (`arch`, `os`, `libc`, `sysroot`) is still unrealized (K6).
use std::path::Path;

use eprfs_core::MethodDescriptor;

use super::{cargo_project, host, is_c_family_var, CRATE_INPUT, DIAGNOSTIC_C_ENV};

/// The Produce slot naming the C baseline put checked the build against.
pub(super) const SLOT: &str = "env:isa-c";

/// The bin the method builds (`--bin=<name>` or `--bin <name>`), whose executable the build
/// messages must name.
pub(super) fn bin_of(descriptor: &MethodDescriptor) -> Option<&str> {
    let mut flags = descriptor.flags.iter();
    while let Some(flag) = flags.next() {
        if let Some(bin) = flag.strip_prefix("--bin=") {
            return Some(bin);
        }
        if flag == "--bin" {
            return flags.next().map(String::as_str);
        }
    }
    None
}

/// A declared target is a native build (coordinator ruling on the L2a rework, point 1): the
/// method's triple is the host its recorded rustc names, so the declared `CFLAGS_<triple>` reach
/// host-side C too and every build script's commands are its own evidence. A cross build is not
/// realized here and refuses; so does a method whose rustc names no host.
pub(super) fn native(descriptor: &MethodDescriptor) -> Vec<String> {
    let Some(rustc_host) = descriptor
        .tools
        .get("rustc")
        .and_then(|rustc| rustc.lines().find_map(|line| line.strip_prefix("host: ")))
        .map(str::trim)
    else {
        return vec![format!(
            "{DECLARED}: the method's rustc names no host triple, so the build cannot be shown \
             to be native"
        )];
    };
    let triple = host::target(descriptor);
    if triple == rustc_host {
        Vec::new()
    } else {
        vec![format!(
            "{DECLARED}: the method builds for {triple} on a {rustc_host} host, a cross build; a \
             declared target is realized only for a native build"
        )]
    }
}

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

#[cfg(test)]
#[path = "isa_c_tests.rs"]
mod tests;
