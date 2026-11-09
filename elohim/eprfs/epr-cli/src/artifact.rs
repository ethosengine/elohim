//! `epr artifact` — derivation artifacts this device builds, records and draws from its person's
//! other devices (artifact-store design §3).
//!
//! - `describe` writes the `mesh-build` method descriptor for one crate binary (§3.a.1): its
//!   tools, flags, declared view and non-path inputs. Its CID is the method CID.
//! - `key` computes the derivation key for a descriptor at this checkout's HEAD: `(method CID,
//!   input)`, and the key's own CID, which is what a build attestation's `inputs_hash` and a
//!   crossing record's `key` name.
//! - `put` records a built artifact (§3.a.3): the descriptor under its CID in the payload store,
//!   the brit build attestation, and one custody Produce through the tier put that is both this
//!   device's custody ledger and, with `--cross`, the `artifact-produce@1` crossing record.
//! - `fetch` asks this device's running `epr peer serve` to draw the artifact for the key from
//!   the person's other devices (§3.b), and answers the installed artifact or a typed gap.
//!
//! **The key input is the labelled interim (§3.a.2).** `KeyInput::Projected(project_view(...))`
//! needs the HEAD tree as an eprfs Tree v1 closure in a store, and no producer of one exists on this
//! tree yet (Track A's F1r). The input is therefore `KeyInput::Raw(<closure CID>)`, the
//! `epr bin` source-state closure form (`path\ttree-or-blob-oid\n` per declared view path at HEAD,
//! addressed under the byte profile), computed over the descriptor's own normalized view, and it
//! is labelled `basis-key:git-tree` wherever it is recorded. One function computes it, for the
//! producer and the consumer alike, so the two sides cannot derive two keys for one closure. A
//! closure with uncommitted changes inside the view has no key: the bytes would not be the HEAD
//! tree's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use elohim_epr_rea::{AgentRef, SidecarFlowStore};
use eprfs_core::{
    BlobCid, DerivationBasis, DerivationKey, DerivationRecord, EprRef, KeyInput, MethodDescriptor,
    TreePath, View, Witness,
};
use eprfs_peer::{
    ensure_queue_dir, fetch_cancel_path, fetch_done_path, fetch_queue_dir, fetch_request_path,
    key_cid, put_local_blob_with, put_method_record, read_queue_file, resolve_holders,
    write_queue_file, Bytes, FetchDone, FetchFor, FetchGap, FetchRequest, FetchTransport,
    LocalBlobContext, SiblingRecord, ARTIFACT_PRODUCE_KIND, ARTIFACT_PRODUCE_MEASURE, FETCH_STALL,
    FETCH_TOTAL,
};
use serde::Serialize;

use crate::device_key::{self, DeviceKey};
use crate::peer::{self, PeerCliError, ServeLock};

/// The method name every mesh build descriptor carries.
pub const MESH_BUILD: &str = "mesh-build";

/// The slot naming the interim key basis (§3.a.2): the input is a git-tree closure CID, not a
/// projected eprfs root.
pub const BASIS_KEY_SLOT: &str = "basis-key:git-tree";

/// The exit code of a typed refusal: an unrecorded put, a fetch gap, a closure with no key. The
/// caller proceeds (the build succeeds unrecorded; the build runs locally).
pub const EXIT_GAP: u8 = 3;

/// Non-path inputs that change only how fast a build runs, never its bytes.
const RESOURCE_CAPS: [&str; 2] = ["CARGO_BUILD_JOBS", "RUST_TEST_THREADS"];

/// Why an `epr artifact` verb could not run.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    /// A typed refusal: the work proceeds without the record or the fetch.
    #[error("{0}")]
    Gap(String),
    #[error("{0}")]
    Peer(#[from] PeerCliError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

type ArtifactResult<T> = Result<T, ArtifactError>;

pub fn usage() -> String {
    "usage: epr artifact <\n  \
     describe --project P --bin B [--features F] --rustflags R --sources MANIFEST#STEP \
     --crate DIR [--env K=V]... [--repo ROOT] [--out FILE] \
     (the mesh-build method descriptor: tools, flags, declared view, non-path inputs)\n  \
     | key --method FILE [--repo ROOT] [--print input|key] [--json] \
     (the derivation key at HEAD; exits 3 when the view has uncommitted changes)\n  \
     | put --method FILE --input CID --bytes PATH --project P [--commit SHA] [--duration-ms N] \
     [--cross] [--repo ROOT] [--payload-store DIR] [--json] \
     (record a built artifact: descriptor, brit build attestation, one custody Produce that \
     crosses with --cross; exits 3 `unrecorded: <reason>`)\n  \
     | fetch --method FILE --dest PATH [--repo ROOT] [--handle H] [--transport tier|iroh-blobs] \
     [--stall S] [--total S] [--json] \
     (draw the artifact for the key from admitted siblings through this device's epr peer \
     serve; exits 3 `fetch gap: <kind>: <reason>`)>"
        .to_string()
}

/// Entry point for `epr artifact …`.
pub fn run(args: &[String]) -> ArtifactResult<ExitCode> {
    let verb = args.first().map(String::as_str);
    if matches!(verb, Some("--help" | "-h" | "help")) {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    let options = Options::parse(args.get(1..).unwrap_or_default())?;
    let outcome = match verb {
        Some("describe") => describe(&options),
        Some("key") => key(&options),
        Some("put") => put(&options),
        Some("fetch") => fetch(&options),
        Some(other) => {
            return Err(ArtifactError::InvalidArguments(format!(
                "unknown artifact verb `{other}`{}\n{}",
                crate::hint::did_you_mean(
                    other,
                    "epr artifact",
                    &["describe", "key", "put", "fetch"]
                ),
                usage()
            )))
        }
        None => return Err(ArtifactError::InvalidArguments(usage())),
    };
    match outcome {
        Err(ArtifactError::Gap(reason)) => {
            if options.json {
                println!("{}", serde_json::json!({ "gap": reason }));
            } else {
                println!("{reason}");
            }
            Ok(ExitCode::from(EXIT_GAP))
        }
        other => other,
    }
}

#[derive(Debug, Default)]
struct Options {
    repo: PathBuf,
    project: Option<String>,
    bin: Option<String>,
    features: Option<String>,
    rustflags: Option<String>,
    sources: Option<String>,
    crate_dir: Option<String>,
    env: Vec<String>,
    out: Option<PathBuf>,
    method: Option<PathBuf>,
    print: Option<String>,
    input: Option<String>,
    bytes: Option<PathBuf>,
    commit: Option<String>,
    duration_ms: u64,
    cross: bool,
    payload_store: Option<PathBuf>,
    dest: Option<PathBuf>,
    handle: Option<String>,
    transport: FetchTransport,
    stall: Option<u64>,
    total: Option<u64>,
    json: bool,
}

impl Options {
    fn parse(args: &[String]) -> ArtifactResult<Options> {
        let mut options = Options {
            repo: PathBuf::from("."),
            ..Options::default()
        };
        let mut i = 0;
        while i < args.len() {
            let flag = args[i].as_str();
            match flag {
                "--json" => {
                    options.json = true;
                    i += 1;
                    continue;
                }
                "--cross" => {
                    options.cross = true;
                    i += 1;
                    continue;
                }
                _ => {}
            }
            let value = args.get(i + 1).cloned().ok_or_else(|| {
                ArtifactError::InvalidArguments(format!("{flag} needs a value\n{}", usage()))
            })?;
            let seconds = |value: &str| {
                value
                    .parse::<u64>()
                    .map_err(|e| ArtifactError::InvalidArguments(format!("{flag} {value}: {e}")))
            };
            match flag {
                "--repo" => options.repo = PathBuf::from(value),
                "--project" => options.project = Some(value),
                "--bin" => options.bin = Some(value),
                "--features" => options.features = Some(value),
                "--rustflags" => options.rustflags = Some(value),
                "--sources" => options.sources = Some(value),
                "--crate" => options.crate_dir = Some(value),
                "--env" => options.env.push(value),
                "--out" => options.out = Some(PathBuf::from(value)),
                "--method" => options.method = Some(PathBuf::from(value)),
                "--print" => options.print = Some(value),
                "--input" => options.input = Some(value),
                "--bytes" => options.bytes = Some(PathBuf::from(value)),
                "--commit" => options.commit = Some(value),
                "--duration-ms" => options.duration_ms = seconds(&value)?,
                "--payload-store" => options.payload_store = Some(PathBuf::from(value)),
                "--dest" => options.dest = Some(PathBuf::from(value)),
                "--handle" => options.handle = Some(value),
                "--transport" => {
                    options.transport = match value.as_str() {
                        "tier" => FetchTransport::Tier,
                        "iroh-blobs" => FetchTransport::IrohBlobs,
                        other => {
                            return Err(ArtifactError::InvalidArguments(format!(
                                "--transport {other}: expected tier or iroh-blobs"
                            )))
                        }
                    }
                }
                "--stall" => options.stall = Some(seconds(&value)?),
                "--total" => options.total = Some(seconds(&value)?),
                other => {
                    return Err(ArtifactError::InvalidArguments(format!(
                        "unknown artifact argument `{other}`\n{}",
                        usage()
                    )))
                }
            }
            i += 2;
        }
        options.repo = std::fs::canonicalize(&options.repo).unwrap_or(options.repo);
        Ok(options)
    }

    fn required<'a>(value: &'a Option<String>, flag: &str) -> ArtifactResult<&'a str> {
        value.as_deref().ok_or_else(|| {
            ArtifactError::InvalidArguments(format!("{flag} needs a value\n{}", usage()))
        })
    }
}

// ---------------------------------------------------------------------------
// the key
// ---------------------------------------------------------------------------

/// The derivation key of a descriptor at a checkout's HEAD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosureKey {
    pub method: BlobCid,
    pub input: BlobCid,
    /// The DAG-CBOR CID of the [`DerivationKey`].
    pub key_cid: BlobCid,
    pub basis: &'static str,
    pub head: String,
    /// Paths inside the view with uncommitted changes (`git status --porcelain`); a dirty closure
    /// has no usable key.
    pub dirty: Vec<String>,
    #[serde(skip)]
    pub key: DerivationKey,
}

fn git(repo: &Path, args: &[&str]) -> ArtifactResult<crate::process::CommandOutput> {
    crate::process::run("git", args, repo)
        .map_err(|e| ArtifactError::Gap(format!("git {} failed: {e}", args.join(" "))))
}

fn git_ok(repo: &Path, args: &[&str]) -> ArtifactResult<String> {
    let out = git(repo, args)?;
    if out.success {
        Ok(out.stdout)
    } else {
        Err(ArtifactError::Gap(format!(
            "git {} failed: {}",
            args.join(" "),
            out.stderr
        )))
    }
}

/// The `non_path_inputs` entry naming the crate directory (relative to the checkout), so the
/// closure can find every Cargo configuration file cargo reads for that crate.
pub const CRATE_INPUT: &str = "crate";

/// Build variables cargo reads that change where or how fast a build runs, never its bytes.
const BENIGN_BUILD_ENV: [&str; 7] = [
    "CARGO_HOME",
    "RUSTUP_HOME",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_JOBS",
    "RUST_TEST_THREADS",
    "RUST_BACKTRACE",
    "RUST_LOG",
];

/// The native toolchain variables a mesh build passes to cargo; `describe` records each one set.
pub const NATIVE_BUILD_ENV: [&str; 9] = [
    "CC",
    "CXX",
    "AR",
    "LIBCLANG_PATH",
    "PKG_CONFIG_PATH",
    "OPENSSL_DIR",
    "OPENSSL_LIB_DIR",
    "OPENSSL_INCLUDE_DIR",
    "PROTOC",
];

fn is_build_var(name: &str) -> bool {
    name.starts_with("CARGO_") || name.starts_with("RUST") || NATIVE_BUILD_ENV.contains(&name)
}

/// Why `env` is not the environment `descriptor` records (review H3), or `None` when it is: every
/// `CARGO_*`, `RUST*` and native toolchain variable set must be benign or recorded with the same
/// value, and every recorded variable must be set to it (an empty `RUSTFLAGS` and an unset one are
/// one build).
pub fn build_env_refusal(
    descriptor: &MethodDescriptor,
    env: impl IntoIterator<Item = (String, String)>,
) -> Option<String> {
    let env: BTreeMap<String, String> = env.into_iter().collect();
    for (name, value) in &env {
        if !is_build_var(name) || BENIGN_BUILD_ENV.contains(&name.as_str()) {
            continue;
        }
        let recorded = descriptor.non_path_inputs.get(name);
        let toolchain =
            name == "RUSTUP_TOOLCHAIN" && descriptor.tools.get("toolchain") == Some(value);
        if recorded != Some(value) && !toolchain {
            return Some(format!(
                "{name} is set in the build environment and the method does not record it with \
                 that value; build under the recorded environment (the mesh build scrubs it)"
            ));
        }
    }
    for (name, value) in &descriptor.non_path_inputs {
        if name == CRATE_INPUT || !is_build_var(name) {
            continue;
        }
        let unset_is_same = name == "RUSTFLAGS" && value.is_empty();
        if !env.contains_key(name) && !unset_is_same {
            return Some(format!(
                "the method records {name}={value}, and it is not set in the build environment"
            ));
        }
    }
    None
}

/// One view path at HEAD: `path\t<mode> <type> <oid>` for a tree, blob or symlink (the mode is
/// part of what was read), the gitlink commit for a path at or under a submodule (git cannot see
/// inside it from here), `-` when absent. Every gitlink met is added to `gitlinks` for the dirty
/// check.
fn closure_line(repo: &Path, path: &str, gitlinks: &mut Vec<String>) -> ArtifactResult<String> {
    let parts: Vec<&str> = path.split('/').collect();
    for end in (1..=parts.len()).rev() {
        let prefix = parts[..end].join("/");
        let listed = git_ok(repo, &["ls-tree", "HEAD", "--", &prefix])?;
        let Some(entry) = listed
            .lines()
            .find_map(|line| line.strip_suffix(&format!("\t{prefix}")))
        else {
            continue;
        };
        if let Some(sha) = entry.strip_prefix("160000 commit ") {
            gitlinks.push(prefix.clone());
            return Ok(format!("{path}\tgitlink:{prefix}@{sha}\n"));
        }
        if end == parts.len() {
            return Ok(format!("{path}\t{entry}\n"));
        }
        // An ancestor exists and is no gitlink, so the path itself is absent.
        break;
    }
    Ok(format!("{path}\t-\n"))
}

/// `path` with `.` and `..` resolved lexically, or `None` when it climbs above the checkout.
fn lexical(path: &str) -> Option<String> {
    let mut kept: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                kept.pop()?;
            }
            other => kept.push(other),
        }
    }
    Some(kept.join("/"))
}

fn under_view(path: &str, view: &[String]) -> bool {
    view.iter()
        .any(|v| path == v || path.starts_with(&format!("{v}/")))
}

/// Refuse a tracked symlink inside the view whose target leaves it (review H4): cargo would read
/// bytes no closure line names.
fn refuse_escaping_symlinks(repo: &Path, view: &[String]) -> ArtifactResult<()> {
    let mut args = vec!["ls-tree", "-r", "HEAD", "--"];
    args.extend(view.iter().map(String::as_str));
    let listed = git_ok(repo, &args)?;
    for line in listed.lines() {
        let Some(rest) = line.strip_prefix("120000 blob ") else {
            continue;
        };
        let Some((oid, link)) = rest.split_once('\t') else {
            continue;
        };
        let target = git_ok(repo, &["cat-file", "blob", oid])?;
        let dir = link.rsplit_once('/').map_or("", |(dir, _)| dir);
        let resolved = if target.starts_with('/') {
            None
        } else {
            lexical(&format!("{dir}/{target}"))
        };
        if !resolved.is_some_and(|resolved| under_view(&resolved, view)) {
            return Err(ArtifactError::Gap(format!(
                "the tracked symlink {link} points outside the method's view ({target}), so the \
                 closure does not name what the build reads"
            )));
        }
    }
    Ok(())
}

/// Refuse index flags that hide working-tree changes from `git status` inside the view:
/// assume-unchanged (a lowercase `ls-files -v` tag) and skip-worktree (`S`).
fn refuse_index_flags(repo: &Path, view: &[String]) -> ArtifactResult<()> {
    let mut args = vec!["ls-files", "-v", "--"];
    args.extend(view.iter().map(String::as_str));
    let listed = git_ok(repo, &args)?;
    let flagged: Vec<&str> = listed
        .lines()
        .filter(|line| {
            line.chars()
                .next()
                .is_some_and(|tag| tag.is_ascii_lowercase() || tag == 'S')
        })
        .take(3)
        .collect();
    if flagged.is_empty() {
        Ok(())
    } else {
        Err(ArtifactError::Gap(format!(
            "index flags (assume-unchanged or skip-worktree) hide changes inside the view: {}",
            flagged.join("; ")
        )))
    }
}

/// Refuse a submodule inside the view whose `ignore` setting would hide its changes.
fn refuse_ignored_submodules(repo: &Path, view: &[String]) -> ArtifactResult<()> {
    let gitmodules = repo.join(".gitmodules");
    let config =
        |args: &[&str]| -> String { git(repo, args).map(|out| out.stdout).unwrap_or_default() };
    let mut ignoring = Vec::new();
    for listed in [
        config(&["config", "--get-regexp", r"^submodule\..*\.ignore$"]),
        if gitmodules.is_file() {
            config(&[
                "config",
                "-f",
                ".gitmodules",
                "--get-regexp",
                r"^submodule\..*\.ignore$",
            ])
        } else {
            String::new()
        },
    ] {
        for line in listed.lines() {
            let Some((key, value)) = line.split_once(' ') else {
                continue;
            };
            if value.trim() == "none" {
                continue;
            }
            if let Some(name) = key
                .strip_prefix("submodule.")
                .and_then(|k| k.strip_suffix(".ignore"))
            {
                ignoring.push(name.to_string());
            }
        }
    }
    for name in ignoring {
        let mut path = config(&["config", "--get", &format!("submodule.{name}.path")]);
        if path.is_empty() && gitmodules.is_file() {
            path = config(&[
                "config",
                "-f",
                ".gitmodules",
                "--get",
                &format!("submodule.{name}.path"),
            ]);
        }
        let path = path.trim();
        let overlaps =
            under_view(path, view) || view.iter().any(|v| v.starts_with(&format!("{path}/")));
        if !path.is_empty() && overlaps {
            return Err(ArtifactError::Gap(format!(
                "the submodule {path} inside the view is configured to ignore changes"
            )));
        }
    }
    Ok(())
}

/// The Cargo configuration cargo reads for the crate, by content and never by path (so two
/// checkouts at different places key alike): every `.cargo/config(.toml)` from the crate's
/// directory up to the filesystem root, nearest first, and `$CARGO_HOME/config(.toml)` (`-` when
/// absent).
fn cargo_config_lines(repo: &Path, descriptor: &MethodDescriptor) -> ArtifactResult<String> {
    let mut lines = String::new();
    let Some(crate_dir) = descriptor.non_path_inputs.get(CRATE_INPUT) else {
        return Ok(lines);
    };
    let content = |path: &Path| -> ArtifactResult<Option<String>> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(BlobCid::compute_bytes(&bytes).to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ArtifactError::Gap(format!(
                "the cargo configuration {} does not read: {e}",
                path.display()
            ))),
        }
    };
    let start = repo.join(crate_dir);
    let mut found = 0usize;
    for dir in start.ancestors() {
        for name in ["config", "config.toml"] {
            if let Some(cid) = content(&dir.join(".cargo").join(name))? {
                lines.push_str(&format!("cargo-config:{found}:{name}\t{cid}\n"));
                found += 1;
            }
        }
    }
    let home = std::env::var_os("CARGO_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")));
    for name in ["config", "config.toml"] {
        let cid = match &home {
            Some(home) => content(&home.join(name))?,
            None => None,
        };
        lines.push_str(&format!(
            "cargo-home:{name}\t{}\n",
            cid.as_deref().unwrap_or("-")
        ));
    }
    Ok(lines)
}

/// The `non_path_inputs` entry naming the fixed `PATH` a recorded mesh build runs cargo under.
pub const PATH_INPUT: &str = "PATH";

/// The tools cargo and the build scripts reach through `PATH` whose identity enters the key: the
/// rustc default linker driver (`cc` on the gnu targets) and the linker it drives (`ld`), plus
/// whatever `CC` names when the method records it.
const KEYED_TOOLS: [&str; 2] = ["cc", "ld"];

/// `name` resolved the way a process with `PATH=path` resolves it: as given when it names a
/// path, else the first executable regular file of that name on `path`; canonicalized.
fn resolve_tool(name: &str, path: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let runnable = |candidate: &Path| {
        std::fs::metadata(candidate)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    };
    let found = if name.contains('/') {
        Some(PathBuf::from(name)).filter(|candidate| runnable(candidate))
    } else {
        std::env::split_paths(path)
            .map(|dir| dir.join(name))
            .find(|candidate| runnable(candidate))
    }?;
    std::fs::canonicalize(&found).ok()
}

/// One line per keyed tool under the method's recorded `PATH` (review H3): the canonical path it
/// resolves to and the CID of its `--version` output (`-` when it does not resolve), so another
/// compiler or linker selected through `PATH` moves the key. A method that records no `PATH`
/// (built in an inherited environment) keys no tool and is refused by nothing here.
fn tool_lines(descriptor: &MethodDescriptor) -> ArtifactResult<String> {
    let Some(path) = descriptor.non_path_inputs.get(PATH_INPUT) else {
        return Ok(String::new());
    };
    let mut names: Vec<(String, String)> = KEYED_TOOLS
        .iter()
        .map(|tool| ((*tool).to_string(), (*tool).to_string()))
        .collect();
    if let Some(cc) = descriptor.non_path_inputs.get("CC") {
        names.push(("CC".to_string(), cc.clone()));
    }
    let mut lines = String::new();
    for (label, name) in names {
        let Some(resolved) = resolve_tool(&name, path) else {
            lines.push_str(&format!("tool:{label}\t-\n"));
            continue;
        };
        let output = std::process::Command::new(&resolved)
            .arg("--version")
            .env_clear()
            .env("PATH", path)
            .env("LANG", "C")
            .output()
            .map_err(|e| {
                ArtifactError::Gap(format!(
                    "{} --version does not run: {e}",
                    resolved.display()
                ))
            })?;
        let version = BlobCid::compute_bytes(&[output.stdout, output.stderr].concat());
        lines.push_str(&format!(
            "tool:{label}\t{}\t{version}\n",
            resolved.display()
        ));
    }
    Ok(lines)
}

/// Untracked files `git status --ignored` lists inside the view are inputs no closure line names;
/// `node_modules` directories are excluded by declaration (cargo never reads them; a residual the
/// report names).
fn is_excluded_ignored(path: &str) -> bool {
    path.split('/').any(|part| part == "node_modules")
}

/// The derivation key of `descriptor` at `repo`'s HEAD (the interim `KeyInput::Raw` form; see the
/// module docs). The one key derivation for this closure: `put` and `fetch` both call it.
///
/// # Errors
///
/// [`ArtifactError::Gap`] when git cannot answer, the descriptor does not encode, or the closure
/// cannot name what a build would read: a tracked symlink leaving the view, an index flag or a
/// submodule `ignore` hiding changes, or ignored untracked files inside the view.
pub fn closure_key(repo: &Path, descriptor: &MethodDescriptor) -> ArtifactResult<ClosureKey> {
    let method = descriptor
        .cid()
        .map_err(|e| ArtifactError::Gap(format!("the method descriptor does not encode: {e}")))?;
    let head = git_ok(repo, &["rev-parse", "HEAD"])
        .map_err(|e| ArtifactError::Gap(format!("{} has no HEAD commit: {e}", repo.display())))?;
    let view: View = descriptor.view();
    let mut paths = Vec::new();
    for path in view.paths() {
        let text = path.to_slash_string().map_err(|e| {
            ArtifactError::Gap(format!(
                "a view path is not UTF-8 and cannot name a git path: {e}"
            ))
        })?;
        paths.push(text);
    }
    if paths.is_empty() || paths.iter().any(String::is_empty) {
        return Err(ArtifactError::Gap(
            "the method declares no view (or the whole tree), so no closure is keyed".into(),
        ));
    }
    refuse_escaping_symlinks(repo, &paths)?;
    refuse_index_flags(repo, &paths)?;
    refuse_ignored_submodules(repo, &paths)?;
    let mut lines = String::new();
    let mut gitlinks = Vec::new();
    for path in &paths {
        lines.push_str(&closure_line(repo, path, &mut gitlinks)?);
    }
    lines.push_str(&cargo_config_lines(repo, descriptor)?);
    lines.push_str(&tool_lines(descriptor)?);
    let mut status_args = vec![
        "status",
        "--porcelain",
        "--untracked-files=normal",
        "--ignored=matching",
        "--ignore-submodules=none",
        "--",
    ];
    status_args.extend(paths.iter().map(String::as_str));
    status_args.extend(gitlinks.iter().map(String::as_str));
    let status = git_ok(repo, &status_args)?;
    let mut dirty = Vec::new();
    let mut ignored = Vec::new();
    for line in status.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        match line.strip_prefix("!! ") {
            Some(path) if !is_excluded_ignored(path) => ignored.push(path.to_string()),
            Some(_) => {}
            None => dirty.push(line.trim().to_string()),
        }
    }
    if !ignored.is_empty() {
        return Err(ArtifactError::Gap(format!(
            "ignored files exist inside the method's view ({}); a build could read them and no \
             closure line names them",
            ignored
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    let input = BlobCid::compute_bytes(lines.as_bytes());
    let key = DerivationKey {
        method: method.clone(),
        input: KeyInput::Raw(input.clone()),
    };
    let key_cid = key_cid(&key).map_err(|e| ArtifactError::Gap(e.to_string()))?;
    Ok(ClosureKey {
        method,
        input,
        key_cid,
        basis: BASIS_KEY_SLOT,
        head,
        dirty,
        key,
    })
}

fn read_descriptor(options: &Options) -> ArtifactResult<MethodDescriptor> {
    let path = options.method.as_ref().ok_or_else(|| {
        ArtifactError::InvalidArguments(format!("--method needs a value\n{}", usage()))
    })?;
    let text = std::fs::read_to_string(path).map_err(|e| {
        ArtifactError::Gap(format!(
            "the method descriptor {} does not read: {e}",
            path.display()
        ))
    })?;
    serde_json::from_str(&text).map_err(|e| {
        ArtifactError::Gap(format!(
            "the method descriptor {} is not one: {e}",
            path.display()
        ))
    })
}

fn clean_key(repo: &Path, descriptor: &MethodDescriptor) -> ArtifactResult<ClosureKey> {
    let key = closure_key(repo, descriptor)?;
    if !key.dirty.is_empty() {
        return Err(ArtifactError::Gap(format!(
            "the closure has uncommitted changes inside the method's view ({}), so the bytes \
             are not HEAD's and no key names them",
            key.dirty.join("; ")
        )));
    }
    Ok(key)
}

/// [`build_env_refusal`] against this process's environment.
fn check_build_env(descriptor: &MethodDescriptor) -> ArtifactResult<()> {
    match build_env_refusal(descriptor, std::env::vars()) {
        Some(reason) => Err(ArtifactError::Gap(reason)),
        None => Ok(()),
    }
}

fn key(options: &Options) -> ArtifactResult<ExitCode> {
    let descriptor = read_descriptor(options)?;
    check_build_env(&descriptor)?;
    let key = closure_key(&options.repo, &descriptor)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&key)?);
    } else {
        match options.print.as_deref() {
            Some("input") => println!("{}", key.input),
            Some("key") => println!("{}", key.key_cid),
            Some(other) => {
                return Err(ArtifactError::InvalidArguments(format!(
                    "--print {other}: expected input or key"
                )))
            }
            None => println!(
                "method {}  input {} ({})  key {}  head {}",
                key.method, key.input, key.basis, key.key_cid, key.head
            ),
        }
    }
    if key.dirty.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!(
            "epr artifact: the closure has uncommitted changes inside the view: {}",
            key.dirty.join("; ")
        );
        Ok(ExitCode::from(EXIT_GAP))
    }
}

// ---------------------------------------------------------------------------
// describe
// ---------------------------------------------------------------------------

/// A declared source glob as the view path covering it: its components up to the first glob
/// character. Wider is safe (a change anywhere under it re-keys); narrower is never produced.
fn glob_prefix(glob: &str) -> Option<String> {
    let kept: Vec<&str> = glob
        .split('/')
        .take_while(|part| !part.contains(['*', '?', '[', '{']))
        .filter(|part| !part.is_empty())
        .collect();
    (!kept.is_empty()).then(|| kept.join("/"))
}

fn tool_line(program: &str, args: &[&str], cwd: &Path) -> ArtifactResult<String> {
    let out = crate::process::run(program, args, cwd)
        .map_err(|e| ArtifactError::Gap(format!("{program} {}: {e}", args.join(" "))))?;
    if !out.success {
        return Err(ArtifactError::Gap(format!(
            "{program} {} failed: {}",
            args.join(" "),
            out.stderr
        )));
    }
    Ok(out.stdout)
}

fn describe(options: &Options) -> ArtifactResult<ExitCode> {
    let project = Options::required(&options.project, "--project")?;
    let bin = Options::required(&options.bin, "--bin")?;
    let rustflags = Options::required(&options.rustflags, "--rustflags")?;
    let sources = Options::required(&options.sources, "--sources")?;
    let crate_dir = Options::required(&options.crate_dir, "--crate")?;
    let repo = &options.repo;
    let crate_path = repo.join(crate_dir);

    let rustc = tool_line("rustc", &["-vV"], &crate_path)?;
    let field = |name: &str| {
        rustc
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}: ")))
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    let host = field("host");
    if host.is_empty() {
        return Err(ArtifactError::Gap("rustc -vV names no host triple".into()));
    }
    let cargo = tool_line("cargo", &["-V"], &crate_path)?;
    let mut tools = BTreeMap::new();
    tools.insert(
        "rustc".to_string(),
        format!(
            "{} commit-hash {} host {host}",
            field("release"),
            field("commit-hash")
        ),
    );
    tools.insert(
        "cargo".to_string(),
        cargo.lines().next().unwrap_or_default().to_string(),
    );
    tools.insert(
        "toolchain".to_string(),
        std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_| "unpinned".into()),
    );

    let features = options.features.as_deref().filter(|f| !f.trim().is_empty());
    let mut flags = vec![format!("--bin={bin}")];
    if let Some(features) = features {
        flags.push(format!("--features={features}"));
    }
    flags.push("--profile=dev".into());
    flags.push(format!("--target={host}"));

    // The view: the build step's declared sources, the lock and cargo config beside the crate, any
    // toolchain file, and every path package cargo resolves for this build.
    let (manifest, step) = sources
        .split_once('#')
        .ok_or_else(|| ArtifactError::InvalidArguments("--sources is MANIFEST#STEP".into()))?;
    let manifest_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo.join(manifest))
            .map_err(|e| ArtifactError::Gap(format!("{manifest} does not read: {e}")))?,
    )?;
    let globs = manifest_json["steps"][step]["inputs"]["sources"]
        .as_array()
        .ok_or_else(|| {
            ArtifactError::Gap(format!(
                "{manifest} declares no steps.{step}.inputs.sources"
            ))
        })?;
    let mut view: Vec<String> = Vec::new();
    for glob in globs {
        let glob = glob.as_str().ok_or_else(|| {
            ArtifactError::Gap(format!("{manifest} {step}: a source is not a string"))
        })?;
        let prefix = glob_prefix(glob).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "{manifest} {step}: the source `{glob}` covers the whole tree; no closure is keyed"
            ))
        })?;
        view.push(prefix);
    }
    for extra in [
        "Cargo.lock",
        ".cargo/config.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
    ] {
        if crate_path.join(extra).is_file() {
            view.push(format!("{crate_dir}/{extra}"));
        }
    }
    view.push(format!("{crate_dir}/Cargo.toml"));
    let mut metadata_args = vec![
        "metadata".to_string(),
        "--format-version".into(),
        "1".into(),
        "--offline".into(),
        "--filter-platform".into(),
        host.clone(),
        "--manifest-path".into(),
        crate_path.join("Cargo.toml").to_string_lossy().into_owned(),
    ];
    if let Some(features) = features {
        metadata_args.push("--features".into());
        metadata_args.push(features.into());
    }
    let metadata_args: Vec<&str> = metadata_args.iter().map(String::as_str).collect();
    let metadata: serde_json::Value =
        serde_json::from_str(&tool_line("cargo", &metadata_args, &crate_path)?)?;
    let root = repo.canonicalize().unwrap_or_else(|_| repo.clone());
    for package in metadata["packages"].as_array().into_iter().flatten() {
        if !package["source"].is_null() {
            continue;
        }
        let Some(dir) = package["manifest_path"]
            .as_str()
            .and_then(|p| Path::new(p).parent())
        else {
            continue;
        };
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let relative = dir.strip_prefix(&root).map_err(|_| {
            ArtifactError::Gap(format!(
                "the path package {} is outside the checkout, so no closure is keyed",
                dir.display()
            ))
        })?;
        view.push(relative.to_string_lossy().into_owned());
    }
    let view = View::from_tree_paths(
        view.iter()
            .map(|path| TreePath::from_slash_bytes(path.as_bytes()).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ArtifactError::Gap(format!("a view path is invalid: {e}")))?,
    );

    let mut non_path_inputs = BTreeMap::new();
    non_path_inputs.insert("RUSTFLAGS".to_string(), rustflags.to_string());
    // The crate, so the closure reads every Cargo configuration cargo reads for it.
    non_path_inputs.insert(CRATE_INPUT.to_string(), crate_dir.to_string());
    // Every native toolchain variable set here reaches cargo in the mesh build: recorded.
    for name in NATIVE_BUILD_ENV {
        if let Ok(value) = std::env::var(name) {
            non_path_inputs.insert(name.to_string(), value);
        }
    }
    for pair in &options.env {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            ArtifactError::InvalidArguments(format!("--env {pair}: expected K=V"))
        })?;
        if RESOURCE_CAPS.contains(&key) {
            continue;
        }
        non_path_inputs.insert(key.to_string(), value.to_string());
    }
    let descriptor = MethodDescriptor {
        name: MESH_BUILD.into(),
        tools,
        flags,
        params: None,
        exec: None,
        step: Some(format!("{MESH_BUILD}:{project}")),
        embedder: None,
        view: view.paths().cloned().collect(),
        non_path_inputs,
        deterministic: false,
    };
    let text = serde_json::to_string_pretty(&descriptor)?;
    match &options.out {
        Some(out) => {
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(out, format!("{text}\n"))?;
            let method = descriptor
                .cid()
                .map_err(|e| ArtifactError::Gap(e.to_string()))?;
            println!("{method}");
        }
        None => println!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// put
// ---------------------------------------------------------------------------

/// What a `put` recorded.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Recorded {
    project: String,
    method: String,
    input: String,
    key: String,
    output: String,
    manifest: String,
    bytes: u64,
    commit: String,
    agent_id: String,
    crosses: bool,
}

fn unrecorded(reason: impl std::fmt::Display) -> ArtifactError {
    ArtifactError::Gap(format!("unrecorded: {reason}"))
}

/// `BRIT_BUILD_REF_BIN`, else `brit-build-ref` on PATH.
fn brit_bin() -> ArtifactResult<PathBuf> {
    if let Some(path) = std::env::var_os(crate::covenant::BRIT_BIN_ENV).filter(|v| !v.is_empty()) {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(unrecorded(format!(
                "{}={} is not a file",
                crate::covenant::BRIT_BIN_ENV,
                path.display()
            )))
        };
    }
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join("brit-build-ref"))
                .find(|candidate| candidate.is_file())
        })
        .ok_or_else(|| unrecorded("brit-build-ref not found on PATH"))
}

fn brit(repo: &Path, args: &[&str]) -> ArtifactResult<String> {
    let bin = brit_bin()?;
    let mut command = std::process::Command::new(&bin);
    command.arg("--repo").arg(repo).args(args).current_dir(repo);
    for key in crate::process::GIT_LOCATION_ENV {
        command.env_remove(key);
    }
    let output = command
        .output()
        .map_err(|e| unrecorded(format!("cannot run {}: {e}", bin.display())))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(unrecorded(format!(
            "brit-build-ref {} failed ({}): {}",
            args.first().copied().unwrap_or(""),
            output.status,
            stderr.lines().next().unwrap_or("").trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Refuse when this device's serving daemon serves another payload store than `payload_dir`
/// (review M9): the custody record would reach its flow store and siblings while its bytes sit
/// where it never serves from.
fn served_store_matches(payload_dir: &Path) -> ArtifactResult<()> {
    let peer_dir = peer::peer_dir()?;
    if ServeLock::held(&peer_dir) != Some(true) {
        return Ok(());
    }
    // Serving, and no readable record of what it serves: fail closed (review M9, ruling item 9).
    // `serve` writes the record right after taking the lock, so this also covers its first moment.
    let marker = peer_dir.join(peer::PAYLOAD_STORE_RECORD);
    let recorded = match std::fs::read_to_string(&marker) {
        Ok(text) if !text.trim().is_empty() => PathBuf::from(text.trim()),
        Ok(_) => {
            return Err(unrecorded(format!(
                "this device's epr peer serve is running and its payload-store record {} is \
                 empty, so this put cannot tell it writes the store the daemon serves; restart \
                 epr peer serve",
                marker.display()
            )))
        }
        Err(e) => {
            return Err(unrecorded(format!(
                "this device's epr peer serve is running and its payload-store record {} does \
                 not read ({e}), so this put cannot tell it writes the store the daemon serves; \
                 restart epr peer serve",
                marker.display()
            )))
        }
    };
    let ours = std::fs::canonicalize(payload_dir).unwrap_or_else(|_| payload_dir.to_path_buf());
    if ours == recorded {
        Ok(())
    } else {
        Err(unrecorded(format!(
            "this device's epr peer serve serves the payload store {}, and this put writes {}; \
             pass --payload-store {} (or the same EPR_PAYLOAD_STORE)",
            recorded.display(),
            ours.display(),
            recorded.display()
        )))
    }
}

fn put(options: &Options) -> ArtifactResult<ExitCode> {
    let project = Options::required(&options.project, "--project")?;
    let input = Options::required(&options.input, "--input")?;
    let bytes_path = options.bytes.as_ref().ok_or_else(|| {
        ArtifactError::InvalidArguments(format!("--bytes needs a value\n{}", usage()))
    })?;
    let repo = &options.repo;
    let descriptor = read_descriptor(options).map_err(unrecorded)?;
    let step = format!("{}:{project}", descriptor.name);
    if descriptor.step.as_deref() != Some(step.as_str()) {
        return Err(unrecorded(format!(
            "the descriptor's step is {:?}, not {step}",
            descriptor.step
        )));
    }
    // The environment recording runs in must be the one the method records (the mesh build runs
    // cargo in exactly it), and the closure must be the one the build started from.
    check_build_env(&descriptor).map_err(unrecorded)?;
    let key = clean_key(repo, &descriptor).map_err(unrecorded)?;
    if key.input.to_string() != input {
        return Err(unrecorded(format!(
            "the closure moved during the build: it was {input}, it is {} now",
            key.input
        )));
    }
    if let Some(commit) = &options.commit {
        if *commit != key.head {
            return Err(unrecorded(format!(
                "HEAD moved during the build: it was {commit}, it is {} now",
                key.head
            )));
        }
    }
    let bytes = Bytes::from(
        std::fs::read(bytes_path)
            .map_err(|e| unrecorded(format!("{} does not read: {e}", bytes_path.display())))?,
    );
    let output = BlobCid::compute_bytes(&bytes);

    let signer: serde_json::Value = serde_json::from_str(&brit(repo, &["signer", "--json"])?)
        .map_err(|e| unrecorded(format!("brit-build-ref signer --json is not JSON: {e}")))?;
    let agent_id = signer["agentId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| unrecorded("this repository has no brit signer (agentId is null)"))?
        .to_string();
    let device = DeviceKey::load(&device_key::resolve_path()?)
        .map_err(|e| unrecorded(format!("this workspace has no device key: {e}")))?
        .did_key();

    let duration = options.duration_ms.to_string();
    brit(
        repo,
        &[
            "build",
            "put",
            "--step",
            &step,
            "--manifest",
            &key.method.to_string(),
            "--output",
            &output.to_string(),
            "--inputs-hash",
            &key.key_cid.to_string(),
            "--commit",
            &key.head,
            "--success",
            "true",
            "--duration-ms",
            &duration,
        ],
    )?;

    let peer_options = peer::Options {
        root: repo.clone(),
        payload_store: options.payload_store.clone(),
        ..peer::Options::default()
    };
    let payload_dir = peer::payload_store_dir(&peer_options)?;
    // The runtime the put runs on: a blob-store payload backend is opened inside it (shared: the
    // serving daemon, when up, is the store's owner).
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let store = runtime
        .block_on(crate::payload::open_payload_store(
            &payload_dir,
            crate::payload::Access::Shared,
        ))
        .map_err(|e| unrecorded(format!("the payload store does not open: {e}")))?
        .store;
    served_store_matches(&payload_dir)?;
    let flow_root = peer::served_root(repo).map_err(unrecorded)?;
    let scope = crate::flow::repository::repository_scope(repo)
        .map_err(|e| unrecorded(format!("this checkout has no repository scope: {e}")))?;
    let record = DerivationRecord {
        key: key.key.clone(),
        output: output.clone(),
        computed_by: agent_id.clone(),
        basis: DerivationBasis::Computed,
        deterministic: descriptor.deterministic,
        witness: Witness::Attestation {
            epr: EprRef::new(format!("brit-build-ref:{step}@{}", key.head)),
        },
    };
    let context = LocalBlobContext {
        custodian: AgentRef(device),
        in_scope_of: scope,
        occurred_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        crossing: options.cross.then(|| {
            (
                ARTIFACT_PRODUCE_MEASURE.to_string(),
                ARTIFACT_PRODUCE_KIND.to_string(),
            )
        }),
    };
    let byte_len = bytes.len() as u64;
    let extra = vec![
        BASIS_KEY_SLOT.to_string(),
        format!("env:project={project}"),
        format!("env:method={}", key.method),
        format!("env:input={}", key.input),
        format!("env:key={}", key.key_cid),
        format!("env:output={output}"),
        format!("env:bytes={byte_len}"),
        format!("env:commit={}", key.head),
        format!("env:agentId={agent_id}"),
    ];
    let put = runtime.block_on(async {
        put_method_record(store.as_ref(), &descriptor)
            .await
            .map_err(|e| unrecorded(format!("the method record is not stored: {e}")))?;
        let mut flow = SidecarFlowStore::open(&flow_root)
            .map_err(|e| unrecorded(format!("the flow store does not open: {e}")))?;
        put_local_blob_with(store.as_ref(), &mut flow, record, bytes, &context, extra)
            .await
            .map_err(unrecorded)
    })?;
    let recorded = Recorded {
        project: project.to_string(),
        method: key.method.to_string(),
        input: key.input.to_string(),
        key: key.key_cid.to_string(),
        output: output.to_string(),
        manifest: put.manifest_cid.to_string(),
        bytes: byte_len,
        commit: key.head.clone(),
        agent_id,
        crosses: options.cross,
    };
    if options.json {
        println!("{}", serde_json::to_string_pretty(&recorded)?);
    } else {
        println!(
            "recorded {} output {} manifest {} key {}{}",
            recorded.project,
            recorded.output,
            recorded.manifest,
            recorded.key,
            if recorded.crosses { " (crosses)" } else { "" }
        );
    }
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// fetch
// ---------------------------------------------------------------------------

fn gap(gap: &FetchGap) -> ArtifactError {
    ArtifactError::Gap(format!("fetch gap: {}: {gap}", gap.kind()))
}

/// Wait up to `wait` for the answer at `done_path` that is for `expected`: its nonce, key, output
/// and destination all equal. Anything else there (another request's answer, a stale or forged
/// one, a file that is not a regular file within the cap) is never taken for it, and is left
/// unread or removed; the matching answer is removed once read.
pub fn wait_for_answer(done_path: &Path, expected: &FetchFor, wait: Duration) -> Option<FetchDone> {
    let started = Instant::now();
    loop {
        if let Ok(bytes) = read_queue_file(done_path) {
            match serde_json::from_slice::<FetchDone>(&bytes) {
                Ok(done) if done.for_request() == expected => {
                    let _ = std::fs::remove_file(done_path);
                    return Some(done);
                }
                // Not this request's: removed so the daemon's own answer can take its place.
                Ok(_) => {
                    let _ = std::fs::remove_file(done_path);
                }
                // Half-written or not an answer: looked at again on the next turn.
                Err(_) => {}
            }
        }
        if started.elapsed() >= wait {
            return None;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn fetch(options: &Options) -> ArtifactResult<ExitCode> {
    let dest = options.dest.as_ref().ok_or_else(|| {
        ArtifactError::InvalidArguments(format!("--dest needs a value\n{}", usage()))
    })?;
    let dest = if dest.is_absolute() {
        dest.clone()
    } else {
        std::env::current_dir()?.join(dest)
    };
    let descriptor = read_descriptor(options)
        .map_err(|e| ArtifactError::Gap(format!("fetch gap: no-key: {e}")))?;
    let key = clean_key(&options.repo, &descriptor)
        .map_err(|e| ArtifactError::Gap(format!("fetch gap: no-key: {e}")))?;
    let peer_dir = peer::peer_dir()?;
    if ServeLock::held(&peer_dir) != Some(true) {
        return Err(gap(&FetchGap::DaemonDown));
    }
    let found = peer::sibling_artifacts(
        &options.repo,
        options.handle.as_deref(),
        &key.key_cid.to_string(),
    )
    .map_err(|e| ArtifactError::Gap(format!("fetch gap: no-record: {e}")))?;
    let records: Vec<SiblingRecord> = found
        .rows
        .iter()
        .filter_map(|row| {
            Some(SiblingRecord {
                device: row.device.clone(),
                output: BlobCid::parse(&row.output).ok()?,
                manifest: BlobCid::parse(&row.manifest).ok()?,
            })
        })
        .collect();
    let (output, holders) = resolve_holders(&records).map_err(|why| gap(&why))?;
    let scope = crate::flow::repository::repository_scope(&options.repo)
        .map_err(|e| ArtifactError::Gap(format!("fetch gap: local-store: {e}")))?;
    let (stall, total) = (
        options.stall.unwrap_or(FETCH_STALL.as_secs()),
        options.total.unwrap_or(FETCH_TOTAL.as_secs()),
    );
    let request = FetchRequest {
        nonce: FetchRequest::fresh_nonce(),
        key: key.key.clone(),
        key_cid: key.key_cid.clone(),
        output,
        holders,
        dest: dest.clone(),
        scope: scope.to_string(),
        transport: options.transport,
        stall_s: stall,
        total_s: total,
    };
    let queue = fetch_queue_dir(&peer_dir);
    ensure_queue_dir(&queue)?;
    let stem = request.stem();
    let request_path = fetch_request_path(&queue, &stem);
    let done_path = fetch_done_path(&queue, &stem);
    let expected = FetchFor::of(&request);
    write_queue_file(&request_path, &request)?;
    let wait = Duration::from_secs(total.saturating_add(stall).saturating_add(30));
    let Some(done) = wait_for_answer(&done_path, &expected, wait) else {
        // Unclaimed: withdrawn here. Claimed: the daemon is told to stop, and whatever it
        // answers later names this request's nonce, which nobody waits on any more.
        if std::fs::remove_file(&request_path).is_err() {
            let _ = write_queue_file(&fetch_cancel_path(&queue, &stem), &expected);
        }
        return Err(ArtifactError::Gap(format!(
            "fetch gap: deadline-exceeded: this device's epr peer serve did not answer the \
             queued fetch within {} s; it was withdrawn",
            wait.as_secs()
        )));
    };
    match done {
        FetchDone::Installed {
            fetched_from,
            chunks_fetched,
            chunks_resumed,
            wasted,
            seconds,
            manifest,
            ..
        } => {
            if options.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "installed": dest,
                        "keyCid": key.key_cid,
                        "manifest": manifest,
                        "fetchedFrom": fetched_from,
                        "chunksFetched": chunks_fetched,
                        "chunksResumed": chunks_resumed,
                        "wasted": wasted,
                        "seconds": seconds,
                    })
                );
            } else {
                println!(
                    "fetched {} from {} ({chunks_fetched} chunks, {chunks_resumed} resumed, \
                     {wasted} bytes wasted, {seconds:.1} s)",
                    dest.display(),
                    fetched_from.join(", ")
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        FetchDone::Gap { gap: why, .. } => Err(gap(&why)),
    }
}
