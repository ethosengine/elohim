//! `epr session stasis {fold,decide,status,classify}`: the native dialogue-stasis fold and the
//! shadow regulator (spec: Native CLI contract). The library does the work in
//! `elohim_device_runtime::session_stasis`; this module parses flags, finds the journal root
//! and the active permission rules, and maps outcomes to the contract's exit codes.
//!
//! Exit codes: 0 read, 1 parse error, 2 no journal for `fold`/`status` (unmeasured; `--emit-count`
//! prints nothing). Shadow `decide` exits 0 even with no journal, marking its answer `unmeasured`.
//! Exit 3 (a live-only policy mismatch) is reserved: this slice always runs shadow.

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use chrono::{DateTime, Utc};
use elohim_device_runtime::session_stasis::{
    self,
    classify::{CallInput, Classifier, EffectContext, RuleSet, StallClass},
    journal::StasisError,
    ClockReading, DecideRequest, FoldOptions, Selection, StasisPolicy, POLICY_ABSENT,
};
use serde_json::{json, Value};

const USAGE: &str = "epr session stasis fold [--journal-root DIR] [--session ID] [--since RFC3339 | --shift-current] [--json] [--emit-count MEASURE@V]\n\
epr session stasis decide --session ID --event FILE|- [--journal-root DIR]\n\
epr session stasis status [--session ID] [--shift-current] [--json] [--journal-root DIR]\n\
epr session stasis classify --corpus FILE [--settings FILE]... [--main-checkout DIR] [--cwd DIR]\n\
(fold/decide/status also take [--settings FILE]...; the default is the project's .claude/settings{,.local}.json)";

/// What a run needs from its environment; injectable so tests run on a fake clock.
pub struct Ctx {
    /// Working directory the root and settings are found from.
    pub cwd: PathBuf,
    /// The clock the fold reads open intervals against.
    pub clock: ClockReading,
    /// Journal segment rotation size.
    pub segment_limit: u64,
    /// `STASIS_JOURNAL_ROOT`, when set.
    pub journal_root_env: Option<PathBuf>,
    /// `CLAUDE_CONFIG_DIR` (or `~/.claude`), for user-level settings.
    pub config_dir: Option<PathBuf>,
    /// Standard input, read lazily for `--event -`.
    pub stdin: Option<String>,
}

impl Ctx {
    /// The real environment.
    pub fn system() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            clock: ClockReading::system(),
            segment_limit: std::env::var("STASIS_SEGMENT_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5 * 1024 * 1024),
            journal_root_env: std::env::var_os("STASIS_JOURNAL_ROOT")
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty()),
            config_dir: std::env::var_os("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude"))),
            stdin: None,
        }
    }
}

#[derive(Default)]
struct Flags {
    journal_root: Option<PathBuf>,
    session: Option<String>,
    since: Option<DateTime<Utc>>,
    shift_current: bool,
    json: bool,
    emit_count: Option<String>,
    event: Option<String>,
    corpus: Option<PathBuf>,
    settings: Vec<PathBuf>,
    main_checkout: Option<PathBuf>,
    cwd: Option<PathBuf>,
}

fn parse_flags(args: &[String]) -> Result<Flags, String> {
    let mut f = Flags::default();
    let mut i = 0;
    let value = |i: &mut usize, name: &str| -> Result<String, String> {
        *i += 1;
        args.get(*i)
            .cloned()
            .ok_or_else(|| format!("{name} needs a value"))
    };
    while i < args.len() {
        match args[i].as_str() {
            "--journal-root" => f.journal_root = Some(value(&mut i, "--journal-root")?.into()),
            "--session" => f.session = Some(value(&mut i, "--session")?),
            "--since" => {
                let raw = value(&mut i, "--since")?;
                f.since = Some(
                    DateTime::parse_from_rfc3339(&raw)
                        .map_err(|e| format!("--since: {e}"))?
                        .with_timezone(&Utc),
                );
            }
            "--shift-current" => f.shift_current = true,
            "--json" => f.json = true,
            "--emit-count" => f.emit_count = Some(value(&mut i, "--emit-count")?),
            "--event" => f.event = Some(value(&mut i, "--event")?),
            "--corpus" | "--classify-corpus" => f.corpus = Some(value(&mut i, "--corpus")?.into()),
            "--settings" => f.settings.push(value(&mut i, "--settings")?.into()),
            "--main-checkout" => f.main_checkout = Some(value(&mut i, "--main-checkout")?.into()),
            "--cwd" => f.cwd = Some(value(&mut i, "--cwd")?.into()),
            other => return Err(format!("unexpected stasis argument {other}")),
        }
        i += 1;
    }
    if f.since.is_some() && f.shift_current {
        return Err("--since and --shift-current are exclusive".into());
    }
    Ok(f)
}

fn project_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|d| {
            d.join(".claude/settings.json").is_file() || d.join(".eprfs/status/stasis").is_dir()
        })
        .map(Path::to_path_buf)
}

fn journal_root(f: &Flags, ctx: &Ctx) -> PathBuf {
    f.journal_root
        .clone()
        .or_else(|| ctx.journal_root_env.clone())
        .unwrap_or_else(|| {
            project_root(&ctx.cwd)
                .unwrap_or_else(|| ctx.cwd.clone())
                .join(".eprfs/status/stasis")
        })
}

/// The main checkout: the root itself, or for a linked worktree the root its `.git` file names.
fn main_checkout(root: &Path) -> PathBuf {
    let git = root.join(".git");
    if git.is_file() {
        if let Some(gitdir) = std::fs::read_to_string(&git).ok().and_then(|s| {
            s.trim()
                .strip_prefix("gitdir:")
                .map(|p| PathBuf::from(p.trim()))
        }) {
            // <main>/.git/worktrees/<name>
            if let Some(main) = gitdir.ancestors().nth(3) {
                return main.to_path_buf();
            }
        }
    }
    root.to_path_buf()
}

/// The active ask/deny rules; `None` when no settings could be read (everything is then
/// `uncertain`, since absence of a rule is not evidence of shape).
fn load_rules(f: &Flags, ctx: &Ctx) -> Option<RuleSet> {
    let mut paths: Vec<PathBuf> = f.settings.clone();
    if paths.is_empty() {
        if let Some(root) = project_root(&ctx.cwd) {
            paths.push(root.join(".claude/settings.json"));
            paths.push(root.join(".claude/settings.local.json"));
        }
        if let Some(dir) = &ctx.config_dir {
            paths.push(dir.join("settings.json"));
        }
    }
    let mut found = false;
    let mut values = Vec::new();
    for p in paths {
        let Ok(bytes) = std::fs::read(&p) else {
            continue;
        };
        found = true;
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) => values.push(v),
            // A settings file that will not parse leaves the rules unknown.
            Err(_) => return None,
        }
    }
    found.then(|| RuleSet::from_settings(&values))
}

fn classifier(f: &Flags, ctx: &Ctx) -> Classifier {
    let root = project_root(&ctx.cwd);
    Classifier::new(
        load_rules(f, ctx),
        EffectContext {
            main_checkout: f
                .main_checkout
                .clone()
                .or_else(|| root.as_deref().map(main_checkout)),
            cwd: Some(f.cwd.clone().unwrap_or_else(|| ctx.cwd.clone())),
        },
    )
}

fn line(out: &mut dyn Write, s: &str) {
    // A closed pipe is the reader's choice, not a fold failure.
    let _ = writeln!(out, "{s}");
}

fn fail(err: &mut dyn Write, e: &StasisError) -> ExitCode {
    let _ = writeln!(err, "epr session stasis: {e}");
    match e {
        StasisError::NoJournal(_) => ExitCode::from(2),
        StasisError::Parse(_) => ExitCode::from(1),
        StasisError::Io(_) => ExitCode::from(1),
    }
}

/// Run one `stasis` command against injected streams and environment; `args` follow `stasis`.
pub fn run_with(
    args: &[String],
    ctx: &mut Ctx,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    let verb = args.first().map(String::as_str).unwrap_or("help");
    if verb == "help" || args.iter().any(|a| a == "--help") {
        line(out, USAGE);
        return ExitCode::SUCCESS;
    }
    let flags = match parse_flags(&args[1..]) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(err, "epr session stasis: {e}\n{USAGE}");
            return ExitCode::from(1);
        }
    };
    let policy = StasisPolicy::builtin();
    let root = journal_root(&flags, ctx);
    let selection = match (flags.since, flags.shift_current) {
        (Some(t), _) => Selection::Since(t),
        (None, true) => Selection::ShiftCurrent,
        _ => Selection::All,
    };
    match verb {
        "fold" => {
            let classifier = classifier(&flags, ctx);
            let o = FoldOptions::new(
                ctx.clock.wall,
                selection,
                flags.session.is_some(),
                &policy,
                &classifier,
            )
            .with_clock(&ctx.clock.boot_id, ctx.clock.uptime_seconds);
            let (report, _) = match session_stasis::fold_root(&root, flags.session.as_deref(), &o) {
                Ok(r) => r,
                Err(e) => return fail(err, &e),
            };
            if let Some(measure) = &flags.emit_count {
                // Policy status goes to stderr so stdout stays the probe's declared output.
                let _ = writeln!(err, "policy: {POLICY_ABSENT}");
                return match session_stasis::emit_count(&report, measure) {
                    Ok(Some(v)) => {
                        line(out, &v.to_string());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        let _ = writeln!(
                            err,
                            "epr session stasis: {measure} is unmeasured for this selection"
                        );
                        ExitCode::from(2)
                    }
                    Err(e) => fail(err, &e),
                };
            }
            if flags.json {
                line(out, &serde_json::to_string(&report).unwrap_or_default());
            } else {
                let m = &report.measures["dialogue-blocked-seconds"];
                line(out, &format!("policy: {}", report.policy));
                line(
                    out,
                    &format!(
                        "sessions: {}   incidents: {}   gaps: {}",
                        report.sessions.len(),
                        report.incidents.len(),
                        report.gaps.len()
                    ),
                );
                line(
                    out,
                    &format!(
                        "dialogue-blocked-seconds@1 shift={}: {}",
                        m["shift"].as_str().unwrap_or(""),
                        if m["value"].is_null() {
                            "unmeasured".to_owned()
                        } else {
                            m["value"].to_string()
                        }
                    ),
                );
            }
            ExitCode::SUCCESS
        }
        "decide" => {
            let (Some(session), Some(event)) = (flags.session.as_deref(), flags.event.as_deref())
            else {
                let _ = writeln!(
                    err,
                    "epr session stasis: decide needs --session and --event\n{USAGE}"
                );
                return ExitCode::from(1);
            };
            let raw = if event == "-" {
                match ctx.stdin.take() {
                    Some(s) => s,
                    None => {
                        let mut s = String::new();
                        if std::io::stdin().read_to_string(&mut s).is_err() {
                            let _ = writeln!(err, "epr session stasis: could not read stdin");
                            return ExitCode::from(1);
                        }
                        s
                    }
                }
            } else {
                match std::fs::read_to_string(event) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = writeln!(err, "epr session stasis: {event}: {e}");
                        return ExitCode::from(1);
                    }
                }
            };
            let request = serde_json::from_str::<Value>(&raw)
                .map_err(|e| StasisError::Parse(format!("event: {e}")))
                .and_then(|v| DecideRequest::from_json(&v));
            let request = match request {
                Ok(r) => r,
                Err(e) => return fail(err, &e),
            };
            let classifier = classifier(&flags, ctx);
            match session_stasis::decide_shadow(
                &root,
                session,
                &request,
                &classifier,
                &policy,
                &ctx.clock,
                ctx.segment_limit,
            ) {
                Ok(decision) => {
                    line(out, &serde_json::to_string(&decision).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => fail(err, &e),
            }
        }
        "status" => {
            let classifier = classifier(&flags, ctx);
            let o = FoldOptions::new(
                ctx.clock.wall,
                selection,
                flags.session.is_some(),
                &policy,
                &classifier,
            )
            .with_clock(&ctx.clock.boot_id, ctx.clock.uptime_seconds);
            let (_, folds) = match session_stasis::fold_root(&root, flags.session.as_deref(), &o) {
                Ok(r) => r,
                Err(e) => return fail(err, &e),
            };
            let sessions: Vec<Value> = folds
                .iter()
                .map(|f| session_stasis::status(f, ctx.clock.wall))
                .collect();
            let body = json!({"policy": POLICY_ABSENT, "sessions": sessions});
            if flags.json {
                line(out, &body.to_string());
            } else {
                line(out, &format!("policy: {POLICY_ABSENT}"));
                for s in &sessions {
                    line(
                        out,
                        &format!(
                            "{}: {} open incident(s), {} pending park(s), coordinator {}",
                            s["session"].as_str().unwrap_or(""),
                            s["openIncidents"].as_array().map_or(0, Vec::len),
                            s["pendingParks"].as_array().map_or(0, Vec::len),
                            s["coordinator"]["state"].as_str().unwrap_or("unknown"),
                        ),
                    );
                }
            }
            ExitCode::SUCCESS
        }
        "classify" => classify_corpus(&flags, ctx, out, err),
        other => {
            let _ = writeln!(err, "epr session stasis: unknown command {other}\n{USAGE}");
            ExitCode::from(1)
        }
    }
}

/// The native side of the differential check against `stasis-probe`'s permission harness: one
/// JSON line per corpus entry. A corpus line is `{"tool":"Bash","command":"..."}`,
/// `{"tool":"Edit","path":"..."}`, or a bare command. A later run diffs `ruleMatch` against
/// whether the live harness fired a PermissionRequest; a disagreement is a failure unless the
/// native side said `risk` or `uncertain`.
fn classify_corpus(flags: &Flags, ctx: &Ctx, out: &mut dyn Write, err: &mut dyn Write) -> ExitCode {
    let Some(path) = &flags.corpus else {
        let _ = writeln!(
            err,
            "epr session stasis: classify needs --corpus FILE\n{USAGE}"
        );
        return ExitCode::from(1);
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(err, "epr session stasis: {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    let classifier = classifier(flags, ctx);
    for (n, raw) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let entry: Value = if raw.trim_start().starts_with('{') {
            match serde_json::from_str(raw) {
                Ok(v) => v,
                Err(e) => {
                    let _ = writeln!(err, "epr session stasis: {}:{}: {e}", path.display(), n + 1);
                    return ExitCode::from(1);
                }
            }
        } else {
            json!({"tool": "Bash", "command": raw})
        };
        let tool = entry["tool"].as_str().unwrap_or("Bash");
        let input = match (entry["command"].as_str(), entry["path"].as_str()) {
            (Some(c), _) => CallInput::Command(c),
            (None, Some(p)) => CallInput::Path(p),
            _ => CallInput::Opaque,
        };
        let c = classifier.classify(StallClass::Authorization, tool, input);
        line(
            out,
            &json!({"line": n + 1, "tool": tool, "input": entry, "classification": c}).to_string(),
        );
    }
    ExitCode::SUCCESS
}

/// Entry point for `epr session stasis ...`; `args` follow `stasis`.
pub fn run(args: &[String]) -> ExitCode {
    let mut ctx = Ctx::system();
    run_with(
        args,
        &mut ctx,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::TimeZone;

    use super::*;

    fn ctx(root: &Path) -> Ctx {
        Ctx {
            cwd: root.to_path_buf(),
            clock: ClockReading {
                wall: Utc.with_ymd_and_hms(2026, 10, 31, 10, 5, 0).unwrap(),
                boot_id: "b".into(),
                uptime_seconds: 5.0,
            },
            segment_limit: 5 << 20,
            journal_root_env: None,
            config_dir: None,
            stdin: None,
        }
    }

    fn event(seq: u64, secs: i64, kind: &str, extra: Value) -> Value {
        let wall = (Utc.with_ymd_and_hms(2026, 10, 31, 10, 0, 0).unwrap()
            + chrono::Duration::seconds(secs))
        .to_rfc3339();
        let mut v = json!({
            "schemaVersion": 1, "kind": kind, "session": {"runtime": "claude-code", "session": "s1"},
            "agentId": "main", "seq": seq,
            "clock": {"wall": wall, "bootId": "b", "uptimeSeconds": 1.0, "provenance": "t"},
            "presence": {"disposition": "unknown", "basis": {"lastWitnessedAgeSeconds": null, "rulingRef": null}},
        });
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }

    fn journal(dir: &Path) -> PathBuf {
        let root = dir.join("stasis");
        let s = root.join("s1");
        fs::create_dir_all(&s).unwrap();
        let corr =
            |id: &str| json!({"incidentId": id, "confidence": "high", "method": "tool-use-id"});
        let lines = [
            event(
                1,
                0,
                "shift-marker",
                json!({"shift": {"id": "shift-7", "edge": "open"}}),
            ),
            event(
                2,
                10,
                "permission-requested",
                json!({"tool": "Bash", "shape": "cd <path> && ls", "source": {"hookEvent": "PermissionRequest"}, "correlation": corr("s1:2")}),
            ),
            event(
                3,
                130,
                "prompt-closed",
                json!({"closedBy": "approved", "durationMs": 0, "correlation": corr("s1:2")}),
            ),
        ];
        fs::write(
            s.join("events.jsonl"),
            lines.iter().map(|l| format!("{l}\n")).collect::<String>(),
        )
        .unwrap();
        root
    }

    fn call(args: &[&str], ctx: &mut Ctx) -> (ExitCode, String, String) {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_with(&args, ctx, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn code(c: ExitCode) -> String {
        format!("{c:?}")
    }

    #[test]
    fn fold_json_reports_incidents_measures_gaps_and_the_policy_state() {
        let dir = tempfile::tempdir().unwrap();
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let (exit, out, _) = call(
            &["fold", "--json", "--journal-root", root.to_str().unwrap()],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["policy"], "absent (defaults, shadow)");
        assert_eq!(v["measures"]["dialogue-blocked-seconds"]["value"], 120.0);
        assert!(v["incidents"][0]["id"] == "s1:2" && v["gaps"].is_array());
        assert!(v["measures"]["ladder-resolution"].is_array());
    }

    #[test]
    fn emit_count_prints_the_probe_object_and_policy_goes_to_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let args = [
            "fold",
            "--shift-current",
            "--emit-count",
            "dialogue-blocked-seconds@1",
            "--journal-root",
            root.to_str().unwrap(),
        ];
        let (exit, out, err) = call(&args, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(
            (v["value"].clone(), v["env"]["shift"].clone()),
            (json!(120.0), json!("shift-7"))
        );
        assert!(err.contains("absent (defaults, shadow)"));
        // Unmeasured selection: nothing on stdout, exit 2.
        let (exit, out, _) = call(
            &[
                "fold",
                "--since",
                "2026-11-30T00:00:00Z",
                "--emit-count",
                "dialogue-blocked-seconds@1",
                "--journal-root",
                root.to_str().unwrap(),
                "--session",
                "s1",
            ],
            &mut c,
        );
        assert!(out.is_empty());
        let _ = exit;
    }

    #[test]
    fn exit_codes_follow_the_contract() {
        let dir = tempfile::tempdir().unwrap();
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let missing = dir.path().join("none");
        let (exit, out, _) = call(
            &[
                "fold",
                "--emit-count",
                "dialogue-blocked-seconds@1",
                "--journal-root",
                missing.to_str().unwrap(),
            ],
            &mut c,
        );
        assert_eq!((code(exit), out.as_str()), (code(ExitCode::from(2)), ""));
        let (exit, ..) = call(&["fold", "--since", "nonsense"], &mut c);
        assert_eq!(code(exit), code(ExitCode::from(1)));
        let (exit, ..) = call(
            &["fold", "--since", "2026-10-31T00:00:00Z", "--shift-current"],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::from(1)));
        fs::write(root.join("s1/events.jsonl"), "garbage\n{}\n").unwrap();
        let (exit, ..) = call(
            &["fold", "--json", "--journal-root", root.to_str().unwrap()],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::from(1)));
    }

    #[test]
    fn decide_is_shadow_journals_would_have_and_names_the_absent_policy() {
        let dir = tempfile::tempdir().unwrap();
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        c.stdin = Some(json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "shape": "git rebase main"}).to_string());
        let (exit, out, _) = call(
            &[
                "decide",
                "--session",
                "s1",
                "--event",
                "-",
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            (
                v["effect"].clone(),
                v["shadow"].clone(),
                v["policy"].clone()
            ),
            (
                json!("none"),
                json!(true),
                json!("absent (defaults, shadow)")
            )
        );
        let last = fs::read_to_string(root.join("s1/events.jsonl"))
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .to_owned();
        assert!(last.contains("\"kind\":\"would-have\""));
        // Unknown session: shadow still exits 0, with effect none and an unmeasured marker.
        c.stdin = Some("{\"hook_event_name\":\"Stop\"}".into());
        let (exit, out, _) = call(
            &[
                "decide",
                "--session",
                "ghost",
                "--event",
                "-",
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            (
                v["effect"].clone(),
                v["shadow"].clone(),
                v["unmeasured"].clone()
            ),
            (json!("none"), json!(true), json!(true))
        );
        assert!(!root.join("ghost").exists());
    }

    #[test]
    fn status_and_classify_corpus_run_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let (exit, out, _) = call(
            &[
                "status",
                "--json",
                "--session",
                "s1",
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["sessions"][0]["coordinator"]["state"], "unknown");
        let settings = dir.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"permissions":{"ask":["Bash(git *rebase*)"]}}"#,
        )
        .unwrap();
        let corpus = dir.path().join("corpus.jsonl");
        fs::write(
            &corpus,
            "git rebase main\ncd a && ls\n{\"tool\":\"Bash\",\"command\":\"cargo test\"}\n",
        )
        .unwrap();
        let (exit, out, _) = call(
            &[
                "classify",
                "--corpus",
                corpus.to_str().unwrap(),
                "--settings",
                settings.to_str().unwrap(),
            ],
            &mut c,
        );
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let kinds: Vec<String> = out
            .lines()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["classification"]["concern"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(kinds, ["risk", "shape", "uncertain"]);
    }
}
