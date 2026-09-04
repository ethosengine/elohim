//! `epr flow` — derive REA/ValueFlows records from the repository filesystem and walk
//! the developer value chain (spec §3–§5,
//! `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`).
//!
//! The projection is deterministic + idempotent: resource identity is the canonical body
//! CID (frontmatter excluded, matching the Python cite oracle), all timestamps come from
//! git, and records are deduped by CID against the existing sidecar before append.

pub mod claim;
pub mod edges;
pub mod fulfill;
pub mod governor;
pub mod note;
pub mod project;
pub mod read;
pub mod registers;
pub mod registry;
pub mod seal;
pub mod stocks;
pub mod walk;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::{atom_cid, AgentRef, PinnedRef};
use eprfs_core::BlobCid;

/// The synthetic agent standing in for the repository as a whole.
pub const REPO_AGENT: &str = "repo:ethosengine/elohim";

/// Errors surfaced by the `flow` command family.
#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("recipe registry parse ({path}): {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("fabric: {0}")]
    Fabric(#[from] elohim_epr_rea::FabricError),

    #[error("resource not found in sidecar labels or the tree: {0}")]
    UnknownResource(String),
}

pub type FlowResult<T> = std::result::Result<T, FlowError>;

/// A label index: `cid string -> repo-relative path or descriptive label`.
pub type Labels = BTreeMap<String, String>;

/// Options shared by the flow subcommands.
struct GlobalOpts {
    root: PathBuf,
    json: bool,
}

/// Entry point invoked by `main` for `epr flow …`.
pub fn run(args: &[String]) -> FlowResult<ExitCode> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err(FlowError::InvalidArguments(usage()));
    };

    match sub {
        "project" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let mut recipes = default_recipes(&opts.root);
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--recipes" => {
                        let value = rest.get(i + 1).ok_or_else(|| {
                            FlowError::InvalidArguments("--recipes needs a path".into())
                        })?;
                        recipes = resolve_under(&opts.root, value);
                        i += 2;
                    }
                    other => {
                        return Err(FlowError::InvalidArguments(format!(
                            "unknown project argument `{other}`"
                        )))
                    }
                }
            }
            let summary = project::project(&opts.root, &recipes)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                summary.render(&opts.root);
            }
            Ok(ExitCode::SUCCESS)
        }
        "walk" => {
            let Some(target) = args.get(1).filter(|a| !a.starts_with("--")) else {
                return Err(FlowError::InvalidArguments(
                    "usage: epr flow walk <path> [--json] [--root DIR]".into(),
                ));
            };
            let (opts, rest) = parse_global(&args[2..])?;
            if let Some(other) = rest.first() {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown walk argument `{other}`"
                )));
            }
            let result = walk::walk(&opts.root, target)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                result.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        "status" => {
            let (opts, rest) = parse_global(&args[1..])?;
            if let Some(other) = rest.first() {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown status argument `{other}`"
                )));
            }
            let result = walk::status(&opts.root)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                result.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        "seal" => run_seal(&args[1..]),
        "reseal" => run_reseal(&args[1..]),
        "hold" => run_hold(&args[1..]),
        "claim" => run_claim(&args[1..]),
        "fulfill" => run_fulfill(&args[1..]),
        "note" => run_note(&args[1..]),
        "stocks" => run_stocks(&args[1..]),
        other => Err(FlowError::InvalidArguments(format!(
            "unknown flow subcommand `{other}`\n{}",
            usage()
        ))),
    }
}

/// The positional `<file>` (first non-flag argument) plus the remaining args untouched.
fn positional_file(args: &[String]) -> FlowResult<(&str, &[String])> {
    match args.first() {
        Some(first) if !first.starts_with("--") => Ok((first.as_str(), &args[1..])),
        _ => Err(FlowError::InvalidArguments(
            "expected a <file> argument".into(),
        )),
    }
}

/// Pull a `--key <value>` option out of `rest`, erroring on a missing value.
fn take_opt(rest: &[String], key: &str) -> FlowResult<Option<String>> {
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == key {
            let value = rest
                .get(i + 1)
                .ok_or_else(|| FlowError::InvalidArguments(format!("{key} needs a value")))?;
            return Ok(Some(value.clone()));
        }
        i += 1;
    }
    Ok(None)
}

fn run_seal(args: &[String]) -> FlowResult<ExitCode> {
    let (file, tail) = positional_file(args)?;
    let (opts, rest) = parse_global(tail)?;
    let on = take_opt(&rest, "--on")?
        .ok_or_else(|| FlowError::InvalidArguments("seal needs --on <upstream>".into()))?;
    // `--governor` absent → auto-derive from `.claude/epr-meta/governors.yaml` (spec §3).
    // An explicit `--governor` is a binding override that bypasses derivation entirely.
    let governor = take_opt(&rest, "--governor")?;
    let desc = take_opt(&rest, "--desc")?;
    let outcome = seal::seal(&opts.root, file, &on, governor.as_deref(), desc)?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

fn run_reseal(args: &[String]) -> FlowResult<ExitCode> {
    let (file, tail) = positional_file(args)?;
    let (opts, rest) = parse_global(tail)?;
    let on = take_opt(&rest, "--on")?;
    let all_stale = rest.iter().any(|a| a == "--all-stale");
    let outcomes = seal::reseal(&opts.root, file, on.as_deref(), all_stale)?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcomes)?);
    } else {
        for outcome in &outcomes {
            outcome.render();
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_hold(args: &[String]) -> FlowResult<ExitCode> {
    let (file, tail) = positional_file(args)?;
    let (opts, rest) = parse_global(tail)?;
    let on = take_opt(&rest, "--on")?
        .ok_or_else(|| FlowError::InvalidArguments("hold needs --on <upstream>".into()))?;
    let reason = take_opt(&rest, "--reason")?
        .ok_or_else(|| FlowError::InvalidArguments("hold needs --reason <text>".into()))?;
    let valid_from = take_opt(&rest, "--valid-from")?;
    let outcome = seal::hold(&opts.root, file, &on, reason, valid_from.as_deref())?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

/// `epr flow claim` — the verb by which an actor TAKES one intent.
///
/// The whole tail is walked once before any value is read, the way `stocks` does it: `take_opt`
/// scans for a `--key value` pair and ignores what it does not recognise, so a mistyped
/// `--supercede` would otherwise read as "no supersede intended" and turn a deliberate takeover
/// into a duplicate refusal the caller never asked for.
fn run_claim(args: &[String]) -> FlowResult<ExitCode> {
    let (opts, rest) = parse_global(args)?;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--supersede" => i += 1,
            "--on" | "--as" | "--brief" | "--serves" | "--session" => i += 2,
            other => {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown claim argument `{other}`"
                )))
            }
        }
    }
    let on = take_opt(&rest, "--on")?.ok_or_else(|| {
        FlowError::InvalidArguments("claim needs --on <intent-cid|gap-id|path>".into())
    })?;
    let brief = take_opt(&rest, "--brief")?;
    let serves = take_opt(&rest, "--serves")?;
    let actor = note::NoteActor {
        as_ref: take_opt(&rest, "--as")?,
        session: resolve_session(take_opt(&rest, "--session")?),
    };
    let outcome = claim::claim(
        &opts.root,
        &claim::ClaimRequest {
            on: &on,
            brief: brief.as_deref(),
            serves: serves.as_deref(),
            supersede: rest.iter().any(|a| a == "--supersede"),
            actor: &actor,
        },
    )?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

/// `epr flow fulfill` — ONE verb, two arms, branched on whether the first argument is a flag.
///
/// The positional arm (`fulfill <report.json>`) reads an a2o sprint report and is byte-compatible
/// with what it has always done. The flag arm (`fulfill --on …`) discharges one claimed gap-item
/// commitment from an authored task report. Branching on the argument SHAPE rather than adding a
/// second subcommand keeps "discharge a promise" one word in the surface, which is what it is in
/// the ledger.
fn run_fulfill(args: &[String]) -> FlowResult<ExitCode> {
    if args.first().is_some_and(|first| first.starts_with("--")) {
        return run_fulfill_on(args);
    }
    let (file, tail) = positional_file(args)?;
    let (opts, rest) = parse_global(tail)?;
    let mut fulfill_opts = fulfill::FulfillOptions::default();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--dry-run" => {
                fulfill_opts.dry_run = true;
                i += 1;
            }
            "--surface-prefix" => {
                let value = rest.get(i + 1).ok_or_else(|| {
                    FlowError::InvalidArguments("--surface-prefix needs a value".into())
                })?;
                fulfill_opts.surface_prefix = value.clone();
                i += 2;
            }
            other => {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown fulfill argument `{other}`"
                )))
            }
        }
    }
    let report_path = resolve_under(&opts.root, file);
    let summary = fulfill::fulfill(&opts.root, &report_path, &fulfill_opts)?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        summary.render();
    }
    Ok(ExitCode::SUCCESS)
}

/// The task-report arm of `fulfill`. Repeatable `--commit` is walked positionally rather than
/// through `take_opt`, which returns only the FIRST match — a second SHA silently dropped is
/// evidence silently dropped.
fn run_fulfill_on(args: &[String]) -> FlowResult<ExitCode> {
    let (opts, rest) = parse_global(args)?;
    let mut commits = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--commit" => {
                let value = rest
                    .get(i + 1)
                    .ok_or_else(|| FlowError::InvalidArguments("--commit needs a sha".into()))?;
                commits.push(value.clone());
                i += 2;
            }
            "--on" | "--report" | "--status" | "--as" | "--session" => i += 2,
            other => {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown fulfill argument `{other}`"
                )))
            }
        }
    }
    let on = take_opt(&rest, "--on")?.ok_or_else(|| {
        FlowError::InvalidArguments("fulfill needs --on <commitment-cid|gap-id>".into())
    })?;
    let report = take_opt(&rest, "--report")?
        .ok_or_else(|| FlowError::InvalidArguments("fulfill needs --report <path>".into()))?;
    let status = take_opt(&rest, "--status")?.ok_or_else(|| {
        FlowError::InvalidArguments("fulfill needs --status DONE|DONE_WITH_CONCERNS".into())
    })?;
    let actor = note::NoteActor {
        as_ref: take_opt(&rest, "--as")?,
        session: resolve_session(take_opt(&rest, "--session")?),
    };
    let outcome = fulfill::fulfill_on(
        &opts.root,
        &fulfill::FulfillOnRequest {
            on: &on,
            report: &report,
            status: &status,
            commits: &commits,
            actor: &actor,
        },
    )?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

/// `epr flow note` — the only leg that takes an AUTHORED observation rather than deriving one
/// from the tree, so all four of its flags are required-or-optional by name and none is
/// positional. Every one of them is refused when absent or blank rather than defaulted: a note
/// whose kind, target, or body was guessed is a record that reads as evidence and is not.
///
/// The two attribution flags are the exception to "nothing is inferred": `--session` falls back
/// to the environment, because the harness that runs an agent knows the session id and the agent
/// writing a mid-run note should not have to repeat it. The fallback lives HERE, in the shell,
/// and never in [`note::note`] — a projection whose record changed with an ambient variable would
/// be neither testable nor reproducible.
fn run_note(args: &[String]) -> FlowResult<ExitCode> {
    let (opts, rest) = parse_global(args)?;
    let on = take_opt(&rest, "--on")?.ok_or_else(|| {
        FlowError::InvalidArguments("note needs --on <commitment-cid-or-path>".into())
    })?;
    let kind = take_opt(&rest, "--kind")?.ok_or_else(|| {
        FlowError::InvalidArguments(
            "note needs --kind failed-approach|correction|observation|ruling|verdict".into(),
        )
    })?;
    let reason = take_opt(&rest, "--reason")?
        .ok_or_else(|| FlowError::InvalidArguments("note needs --reason <text>".into()))?;
    let switched_to = take_opt(&rest, "--switched-to")?;
    let verdict = take_opt(&rest, "--verdict")?;
    let actor = note::NoteActor {
        as_ref: take_opt(&rest, "--as")?,
        session: resolve_session(take_opt(&rest, "--session")?),
    };
    let outcome = note::note(
        &opts.root,
        &on,
        &kind,
        &reason,
        switched_to.as_deref(),
        verdict.as_deref(),
        &actor,
    )?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

/// The session id a run is acting under: the explicit flag first, then `CLAUDE_SESSION_ID`, then
/// `ELOHIM_SESSION_ID`.
///
/// A BLANK value at any position is absence, never a session named by the empty string. An
/// exported-but-empty variable is exactly what an un-sessioned harness looks like, and taking it
/// literally would send every such run asking the actor sidecar who claimed session `""` — a
/// question that has one answer for everybody.
fn resolve_session(explicit: Option<String>) -> Option<String> {
    explicit
        .into_iter()
        .chain(
            ["CLAUDE_SESSION_ID", "ELOHIM_SESSION_ID"]
                .into_iter()
                .filter_map(|key| std::env::var(key).ok()),
        )
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

/// `epr flow stocks` — the only leg whose EXIT CODE is a verdict rather than a status.
///
/// Two things about the argument handling are deliberate. First, `--window` and `--per` are
/// **refused when absent** rather than defaulted: `Window` declares its denominator on purpose
/// (`stock.rs:70-73`), and any default this leg invented would either be a wall-clock read — on
/// a path whose whole discipline is history-derived time — or a hidden claim about what counts
/// as "now". Second, `--check` is the only flag that can make this leg exit non-zero, and when
/// it does, it does so **fail-closed**: a refused fold, an empty window, or a filling stock all
/// exit non-zero, because "we cannot see the drain" and "the drain is adequate" must never share
/// an exit code.
fn run_stocks(args: &[String]) -> FlowResult<ExitCode> {
    let (opts, rest) = parse_global(args)?;
    let check = rest.iter().any(|a| a == "--check");
    // Walk the whole tail once before reading any value: `take_opt` scans for a `--key value`
    // pair and ignores everything it does not recognise, so a typo'd flag would otherwise be
    // silently dropped — on a leg whose exit code is a verdict, a mis-typed `--windwo` reading
    // as "no window declared" is the honest outcome only because this loop refuses it first.
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--check" => i += 1,
            "--window" | "--per" | "--stock" => i += 2,
            other => {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown stocks argument `{other}`"
                )))
            }
        }
    }
    let window_spec = take_opt(&rest, "--window")?.ok_or_else(|| {
        FlowError::InvalidArguments(
            "stocks needs --window START..END — a rate whose window is implicit is not a claim"
                .into(),
        )
    })?;
    let per_raw = take_opt(&rest, "--per")?.ok_or_else(|| {
        FlowError::InvalidArguments(
            "stocks needs --per <second|minute|hour|day|week> — the denominator every rate here \
             carries is declared, never inherited from a subtraction"
                .into(),
        )
    })?;
    let per = stocks::parse_period(&per_raw)?;
    let window = stocks::parse_window(&window_spec, per)?;
    let names = vec![stocks::StockName::parse(
        take_opt(&rest, "--stock")?
            .as_deref()
            .unwrap_or("commitments"),
    )?];

    let outcome = stocks::stocks(&opts.root, &window, &names)?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    if check && !outcome.equilibrium {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

/// Pull `--root` and `--json` out of `args`, returning the remainder untouched.
fn parse_global(args: &[String]) -> FlowResult<(GlobalOpts, Vec<String>)> {
    let mut root = PathBuf::from(".");
    let mut json = false;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                json = true;
                i += 1;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| FlowError::InvalidArguments("--root needs a path".into()))?;
                root = PathBuf::from(value);
                i += 2;
            }
            _ => {
                rest.push(args[i].clone());
                i += 1;
            }
        }
    }
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    Ok((GlobalOpts { root, json }, rest))
}

pub fn default_recipes(root: &Path) -> PathBuf {
    root.join(".claude/epr-meta/recipes.yaml")
}

fn resolve_under(root: &Path, value: &str) -> PathBuf {
    let p = PathBuf::from(value);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn usage() -> String {
    "usage: epr flow <\n  \
     project [--root DIR] [--recipes PATH]\n  \
     | walk <path> [--json] [--root DIR]\n  \
     | status [--root DIR] [--json]\n  \
     | seal <file> --on <upstream> \
     [--governor compiler:<unit>|codegen:<pipeline>|schema-contract:<test>|test:<id>|cite-seal] \
     [--desc <text>] [--json] [--root DIR] \
     (omit --governor to auto-derive from .claude/epr-meta/governors.yaml)\n  \
     | reseal <file> [--on <upstream>] [--all-stale] [--json] [--root DIR]\n  \
     | hold <file> --on <upstream> --reason <text> [--valid-from <iso8601>] [--json] [--root DIR]\n  \
     | claim --on <intent-cid|gap-id|path> [--as agent:<role>@<model>] [--brief <path>] \
     [--serves <habit-id>] [--session <id>] [--supersede] [--json] [--root DIR] \
     (--serves is checked against genesis/manifests/habits.yaml; a standing claim on the same \
     intent is refused by name unless --supersede)\n  \
     | fulfill <report.json> [--dry-run] [--surface-prefix genesis/a2o] [--json] [--root DIR]\n  \
     | fulfill --on <commitment-cid|gap-id> --report <path> --status DONE|DONE_WITH_CONCERNS \
     [--commit <sha>]... [--as agent:<role>@<model>] [--session <id>] [--json] [--root DIR] \
     (NEEDS_CONTEXT|BLOCKED|HOLD are refused — record those as note --kind observation)\n  \
     | note --on <commitment-cid-or-path> \
     --kind failed-approach|correction|observation|ruling|verdict \
     --reason <text> [--switched-to <text>] \
     [--verdict approved|changes-requested (required by --kind verdict, refused on every other kind)] \
     [--as agent:<role>@<model>] [--session <id>] [--json] [--root DIR] \
     (omit --session to fall back to CLAUDE_SESSION_ID/ELOHIM_SESSION_ID; \
     an agent-attributed note carries steward:<git-author-email> as its last slot)\n  \
     | stocks --window START..END --per <second|minute|hour|day|week> \
     [--stock commitments] [--check] [--json] [--root DIR] \
     (--check exits non-zero when a stock is filling OR when nothing could be measured)\n>"
        .to_string()
}

// ---------------------------------------------------------------------------
// Canonical body → CID (the cite-oracle-parity encoder)
// ---------------------------------------------------------------------------

/// Extract the canonical BODY of a document exactly as the Python cite oracle does
/// (`_lib/cite_graph.body_of` + `_body_digest`): frontmatter excluded when a `---`
/// fenced block opens the file, then `.strip()`-equivalent trimming. A file with no
/// frontmatter (`.feature`, `.json`) canonicalizes to its whole, trimmed contents.
pub fn canonical_body(text: &str) -> String {
    let body = strip_frontmatter(text);
    body.trim().to_string()
}

/// Split the frontmatter from a document body, mirroring `_lib.frontmatter.parse`.
fn strip_frontmatter(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return text.to_string();
    }
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }
    match end {
        Some(e) => lines[e + 1..].join("\n"),
        None => text.to_string(),
    }
}

/// The canonical body CID (`CIDv1 · raw codec 0x55 · sha2-256`) of a document — the same
/// content-address whose short form is the doc's cite fingerprint (2026-07-12 convergence).
pub fn body_cid(text: &str) -> Cid {
    *BlobCid::compute_raw(canonical_body(text).as_bytes()).as_cid()
}

/// Read a file and return its canonical body CID, or `None` if the file cannot be read.
pub fn body_cid_of_file(path: &Path) -> Option<Cid> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(body_cid(&text))
}

/// The repo-scope atom CID — the container of last resort for repo-wide flows.
pub fn repo_scope_atom() -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: REPO_AGENT.to_string(),
        version: 1,
    })?)
}

/// The synthetic repository agent.
pub fn repo_agent() -> AgentRef {
    AgentRef(REPO_AGENT.to_string())
}

// ---------------------------------------------------------------------------
// Frontmatter (Rust reader — scalar + string-list subset, matching the oracle)
// ---------------------------------------------------------------------------

/// A parsed frontmatter block: scalar fields and string-list fields.
#[derive(Debug, Default)]
pub struct Frontmatter {
    scalars: BTreeMap<String, String>,
    lists: BTreeMap<String, Vec<String>>,
}

impl Frontmatter {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.scalars.get(key).map(String::as_str)
    }

    pub fn list(&self, key: &str) -> &[String] {
        self.lists.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Parse the `---`-fenced frontmatter of a document (subset: `key: value` scalars and
/// `key:` followed by `- item` lists — the shape spec/plan frontmatter uses).
pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return fm;
    }
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }
    let Some(end) = end else {
        return fm;
    };

    let mut pending: Option<String> = None;
    for line in &lines[1..end] {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            pending = None;
            continue;
        }
        // List continuation `  - item`
        if let Some(key) = &pending {
            let l = trimmed.trim_start();
            if let Some(item) = l.strip_prefix("- ") {
                fm.lists
                    .entry(key.clone())
                    .or_default()
                    .push(item.trim().to_string());
                continue;
            }
        }
        // `key: value` / `key:`
        if let Some((key, val)) = split_kv(trimmed) {
            if val.is_empty() {
                fm.lists.entry(key.clone()).or_default();
                pending = Some(key);
            } else {
                fm.scalars
                    .insert(key, val.trim_matches('"').trim_matches('\'').to_string());
                pending = None;
            }
        } else {
            pending = None;
        }
    }
    fm
}

fn split_kv(line: &str) -> Option<(String, String)> {
    // Only treat as a top-level key when there is no leading indentation.
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (key, val) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((key.to_string(), val.trim().to_string()))
}

/// Extract the `path:` locator from a single cite envelope line, if present.
/// Envelope: `<slug> | <desc> | <fingerprint> [| status: …] [| path: <locator>]`.
pub fn cite_path(entry: &str) -> Option<String> {
    for segment in entry.split('|') {
        let seg = segment.trim();
        if let Some(rest) = seg.strip_prefix("path:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Extract the `sha256:hex16` fingerprint token from a cite envelope line, if present.
pub fn cite_fingerprint(entry: &str) -> Option<String> {
    for segment in entry.split('|') {
        let seg = segment.trim();
        if seg.starts_with("sha256:") {
            return Some(seg.to_string());
        }
    }
    None
}

/// The `slug` (ref) of a cite envelope — the first `|`-delimited segment. `None` for an
/// empty entry. Mirrors `_lib/cite_graph.parse_cite`'s `parts[0]`.
pub fn cite_slug(entry: &str) -> Option<String> {
    let slug = entry.split('|').next()?.trim();
    if slug.is_empty() {
        None
    } else {
        Some(slug.to_string())
    }
}

/// Extract the tool-managed `status:` hint from a cite envelope line, if present.
pub fn cite_status(entry: &str) -> Option<String> {
    for segment in entry.split('|') {
        let seg = segment.trim();
        if let Some(rest) = seg.strip_prefix("status:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Git provenance
// ---------------------------------------------------------------------------

/// What one commit says about who made it: the signing author, when, and whom the author named
/// alongside themselves.
///
/// The three travel together because they come from one `git log` line and mean one thing —
/// *this act, by these people, at this instant*. Splitting them across helpers would let a caller
/// take the timestamp from one commit and the roster from another, which is a plural-authorship
/// claim nobody made.
///
/// `co_authors` holds trailer values **verbatim** (`Claude Fable 5 <noreply@anthropic.com>`).
/// This layer reads git; it does not interpret it. Turning a raw trailer into projection
/// vocabulary is the caller's decision and lives where that vocabulary is minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub author: String,
    pub occurred_at: String,
    pub co_authors: Vec<String>,
}

/// The provenance of the commit that ADDED `rel_path`, falling back to the most recent commit
/// touching it. `None` when git has no history for the path.
///
/// The add-commit is the one that answers "who produced this", which is why the fallback is a
/// fallback: a doc's most recent toucher is its latest editor, and attributing production to them
/// would credit whoever fixed the last typo.
pub fn producing_commit(root: &Path, rel_path: &str) -> Option<Provenance> {
    // Oldest add is the last line of the reverse-chronological add-filtered log.
    if let Some(found) = git_log_provenance(root, &["--diff-filter=A"], rel_path)
        .into_iter()
        .last()
    {
        return Some(found);
    }
    git_log_provenance(root, &["-1"], rel_path)
        .into_iter()
        .next()
}

/// The committer-date Unix timestamp of `HEAD` — the git-derived clock the flow family
/// uses for seal/hold records (`%ct`), never a wall-clock read. `None` when git has no
/// history (a fresh, uncommitted tree). Reuses the same `git log` provenance source as
/// [`producing_commit`].
pub fn head_commit_epoch(root: &Path) -> Option<i64> {
    let out = crate::process::build_command("git", &["log", "-1", "--format=%ct"], root, &[])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// The `(author-email, RFC3339 author-date)` of `HEAD` — the whole-tree analogue of
/// [`producing_commit`], for a record that is about the tree rather than about one path.
///
/// One `git log -1` yields both halves, so a caller needing provenance AND a timestamp spawns a
/// single child process rather than two. `None` when git has no history (a fresh, uncommitted
/// tree) or when either half comes back empty — an unattributed record is refused by the caller,
/// never filled in with a placeholder.
/// Canonical UTC spelling for a git `%aI` timestamp.
///
/// Git's strict-ISO output is a moving target across versions: 2.47 prints a UTC author date
/// as `+00:00`, 2.52 prints `Z`. These bytes reach content-addressed records, so left raw the
/// SAME commit mints a DIFFERENT CID depending on which git binary ran the projection — caught
/// live by `flow_coauthors::a_solo_produce_event_keeps_the_cid_…` (green on a 2.52 workstation,
/// red on a 2.47 CI pod, one commit). Parseable RFC3339 re-encodes as UTC with `Z` at second
/// precision: the instant is untouched, only the spelling is pinned. Unparseable input passes
/// through verbatim — a strange date is still provenance, and refusing it would drop the record.
pub(crate) fn normalize_git_timestamp(ts: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        Err(_) => ts.to_string(),
    }
}

pub fn head_commit_provenance(root: &Path) -> Option<(String, String)> {
    let out =
        crate::process::build_command("git", &["log", "-1", "--format=%ae%x1f%aI"], root, &[])
            .output()
            .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (email, ts) = text.lines().next()?.split_once('\u{1f}')?;
    if email.is_empty() || ts.is_empty() {
        return None;
    }
    Some((email.to_string(), normalize_git_timestamp(ts)))
}

/// Field separator inside one commit's line; record separator between trailer values.
///
/// Both are control characters no commit message or address carries, so the line is parseable
/// without quoting rules. `%(trailers:…)` does the RFC-822-shaped work — folded continuations,
/// case-insensitive key match, the `Key: value` split — which is why nothing here re-implements
/// trailer grammar.
const FIELD_SEP: char = '\u{1f}';
const TRAILER_SEP: char = '\u{1e}';

/// A git too old to know `%(trailers)` prints the specifier **verbatim** rather than failing, so
/// the third field arrives as literal format text instead of a roster. Detected by its own
/// opening token and read as "this git cannot tell us" — an empty roster, which is honest
/// absence. Inventing a co-author out of a format string would be worse than seeing none.
const UNEXPANDED_TRAILERS: &str = "%(trailers";

fn git_log_provenance(root: &Path, extra: &[&str], rel_path: &str) -> Vec<Provenance> {
    let mut args: Vec<&str> = vec!["log"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&[
        "--format=%ae%x1f%aI%x1f%(trailers:key=Co-Authored-By,valueonly,separator=%x1e)",
        "--",
        rel_path,
    ]);
    let Ok(out) = crate::process::build_command("git", &args, root, &[]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(parse_provenance_line)
        .collect()
}

/// One `git log` line → one [`Provenance`]. Pure, so the shapes that are awkward to provoke from
/// a real repository — an ancient git, a commit with no trailers at all — are testable directly.
fn parse_provenance_line(line: &str) -> Option<Provenance> {
    // `splitn(3)` so a trailer value that itself contained a field separator would land whole in
    // the roster rather than truncating the line into a different commit.
    let mut fields = line.splitn(3, FIELD_SEP);
    let email = fields.next()?;
    let ts = fields.next()?;
    let trailers = fields.next().unwrap_or_default();
    if email.is_empty() {
        return None;
    }
    let co_authors = if trailers.contains(UNEXPANDED_TRAILERS) {
        Vec::new()
    } else {
        trailers
            .split(TRAILER_SEP)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    };
    Some(Provenance {
        author: email.to_string(),
        occurred_at: normalize_git_timestamp(ts),
        co_authors,
    })
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Turn an absolute path under `root` into a `/`-joined repo-relative string.
pub fn rel_to_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Confine `path` under an already-canonical `root` — the gate for CLI-supplied
/// `<file>`/`--on` arguments (`epr flow seal|reseal|hold`). Canonicalizes `path`; when it
/// doesn't exist yet (a `--on` upstream being sealed for the first time, or a `<file>` not
/// yet on disk), canonicalizes its parent directory instead and rejoins the file name, so a
/// legitimately fresh target under the tree is still accepted. Errors — never silently
/// truncates or writes outside the tree — when the canonical result does not fall under
/// `root` (`--on ../outside.md`, an absolute path elsewhere, a `..`-laden argument).
pub fn confine_under(root: &Path, path: &Path) -> FlowResult<PathBuf> {
    let canonical = if path.exists() {
        std::fs::canonicalize(path).map_err(|source| FlowError::Read {
            path: path.to_path_buf(),
            source,
        })?
    } else {
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let canonical_parent = std::fs::canonicalize(parent).map_err(|source| FlowError::Read {
            path: parent.to_path_buf(),
            source,
        })?;
        match path.file_name() {
            Some(name) => canonical_parent.join(name),
            None => canonical_parent,
        }
    };
    if !canonical.starts_with(root) {
        return Err(FlowError::InvalidArguments(format!(
            "path `{}` escapes the repo root `{}`",
            path.display(),
            root.display()
        )));
    }
    Ok(canonical)
}

/// A short, human-glanceable rendering of a CID for fallback labels.
pub fn short_cid(cid: &Cid) -> String {
    let s = cid.to_string();
    if s.len() > 12 {
        format!("{}…{}", &s[..8], &s[s.len() - 4..])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(email: &str, ts: &str, trailers: &str) -> String {
        format!("{email}{FIELD_SEP}{ts}{FIELD_SEP}{trailers}")
    }

    #[test]
    fn a_roster_splits_on_the_record_separator_and_keeps_values_verbatim() {
        let raw = line(
            "author@example.test",
            "2026-08-15T12:00:00Z",
            "Claude Fable 5 <noreply@anthropic.com>\u{1e}ethosengine <noreply@ethosengine.com>",
        );
        let p = parse_provenance_line(&raw).expect("a well-formed line parses");
        assert_eq!(p.author, "author@example.test");
        assert_eq!(p.occurred_at, "2026-08-15T12:00:00Z");
        assert_eq!(
            p.co_authors,
            vec![
                "Claude Fable 5 <noreply@anthropic.com>".to_string(),
                "ethosengine <noreply@ethosengine.com>".to_string(),
            ],
            "trailer values reach the caller exactly as the author wrote them"
        );
    }

    #[test]
    fn a_commit_with_no_trailers_yields_an_empty_roster_not_one_blank_co_author() {
        // The format leaves a trailing separator with nothing after it, and an empty string is
        // absence — a roster of one nameless person would classify every solo commit as plural.
        let p = parse_provenance_line(&line("solo@example.test", "2026-08-15T12:00:00Z", ""))
            .expect("a trailer-less line still carries provenance");
        assert!(p.co_authors.is_empty());
    }

    #[test]
    fn an_old_git_that_prints_the_format_specifier_is_read_as_no_roster() {
        // Older git does not fail on an unknown placeholder; it echoes it. Parsed naively, every
        // commit in such a repository would gain a co-author named after a format string.
        let raw = line(
            "author@example.test",
            "2026-08-15T12:00:00Z",
            "%(trailers:key=Co-Authored-By,valueonly,separator=%x1e)",
        );
        let p = parse_provenance_line(&raw).expect("the first two fields are still usable");
        assert!(p.co_authors.is_empty());
        assert_eq!(p.author, "author@example.test");
    }

    #[test]
    fn an_authorless_line_is_refused_rather_than_attributed_to_nobody() {
        assert!(parse_provenance_line(&line("", "2026-08-15T12:00:00Z", "")).is_none());
    }

    #[test]
    fn the_same_instant_spells_identically_whichever_git_printed_it() {
        // git 2.47 prints a UTC author date as `+00:00`; 2.52 prints `Z`. Both must reach the
        // record as the SAME bytes or the same commit mints two different CIDs (the live
        // local-green/CI-red split behind flow_coauthors' solo-CID golden).
        assert_eq!(
            normalize_git_timestamp("2026-08-15T12:00:00+00:00"),
            "2026-08-15T12:00:00Z"
        );
        assert_eq!(
            normalize_git_timestamp("2026-08-15T12:00:00Z"),
            "2026-08-15T12:00:00Z"
        );
    }

    #[test]
    fn a_non_utc_offset_is_converted_to_the_instant_not_the_spelling() {
        // The author's own offset is provenance about their clock, but the ADDRESS must be
        // offset-independent: same instant, same bytes, same CID.
        assert_eq!(
            normalize_git_timestamp("2026-08-15T14:00:00+02:00"),
            "2026-08-15T12:00:00Z"
        );
    }

    #[test]
    fn an_unparseable_timestamp_passes_through_verbatim() {
        // A strange date is still provenance; refusing it would drop the record entirely.
        assert_eq!(normalize_git_timestamp("not-a-date"), "not-a-date");
        assert_eq!(normalize_git_timestamp(""), "");
    }
}
