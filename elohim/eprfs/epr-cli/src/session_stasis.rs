//! `epr session stasis {fold,decide,status,classify}`: the native dialogue-stasis fold and the
//! shadow regulator (spec: Native CLI contract). The library does the work in
//! `elohim_device_runtime::session_stasis`; this module parses flags, finds the journal root
//! and the active permission rules, and maps outcomes to the contract's exit codes.
//!
//! Exit codes: 0 read, 1 parse error, 2 no journal for `fold`/`status` (unmeasured; `--emit-count`
//! prints nothing). Shadow `decide` exits 0 even with no journal, marking its answer `unmeasured`.
//! Exit 3 is live-only (`decide --live`, or `STASIS_LADDER=live`): the policy file is absent,
//! unparsable, or its raw-byte CID is not the one the steward's latest `--kind ruling` names. The
//! regulator then prints `{"effect":"none","inert":true,"reason":...}` and journals an
//! `observation-gap`, so it is visibly inert rather than silently live. A matching pin still
//! applies no effect in this slice (slice 9 turns effects on).

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
    live::LiveDecision,
    policy::{
        read_policy_file, Pin, PolicyAssessment, PolicyState, AWAY_PURPOSE, POLICY_PURPOSE,
        POLICY_REL_PATH,
    },
    AwayRuling, ClockReading, DecideRequest, FoldOptions, Selection,
};
use serde_json::{json, Value};

const USAGE: &str = "epr session stasis fold [--journal-root DIR] [--session ID] [--since RFC3339 | --shift-current] [--json] [--emit-count MEASURE@V]\n\
epr session stasis decide --session ID --event FILE|- [--live] [--journal-root DIR]\n\
epr session stasis status [--session ID] [--shift-current] [--json] [--journal-root DIR]\n\
epr session stasis classify --corpus FILE [--settings FILE]... [--main-checkout DIR] [--cwd DIR]\n\
(fold/decide/status also take [--settings FILE]...; the default is the project's .claude/settings{,.local}.json)";

/// What a run needs from its environment; injectable so tests run on a fake clock.
pub struct Ctx {
    /// Working directory the root and settings are found from.
    pub cwd: PathBuf,
    /// The clock the fold reads open intervals against.
    pub clock: ClockReading,
    /// Re-reads the clock after the journal lock is held (the system clock in production);
    /// `None` reuses `clock`, which fixtures that run on a fake clock want.
    pub reclock: Option<fn() -> ClockReading>,
    /// Journal segment rotation size.
    pub segment_limit: u64,
    /// `STASIS_JOURNAL_ROOT`, when set.
    pub journal_root_env: Option<PathBuf>,
    /// `CLAUDE_CONFIG_DIR` (or `~/.claude`), for user-level settings.
    pub config_dir: Option<PathBuf>,
    /// Standard input, read lazily for `--event -`.
    pub stdin: Option<String>,
    /// `STASIS_LADDER`, when set; `live` selects live `decide` as `--live` does.
    pub ladder_env: Option<String>,
    /// Test seam: the only steward the pin check accepts. `None` asks the collective natively.
    pub steward_of_record: Option<String>,
}

impl Ctx {
    /// The real environment.
    pub fn system() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            clock: ClockReading::system(),
            reclock: Some(ClockReading::system),
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
            ladder_env: std::env::var("STASIS_LADDER").ok(),
            steward_of_record: None,
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
    live: bool,
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
            "--live" => f.live = true,
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

/// The steward's latest ruling for the policy: the CID its reason names.
///
/// Only the steward's word counts: a `--kind ruling` note on the policy path whose `steward:`
/// slot names a steward of the repository's collective (the predicate `epr covenant` applies),
/// authored by that same human (`actor` equals the slot, so an agent identity never qualifies).
/// Order is `occurred_at` parsed as RFC 3339, then append order; any candidate ruling on the
/// path with an unparsable `occurred_at` makes the pin unreadable. The NEWEST steward ruling
/// decides: if its reason is not the pin JSON (`purpose`, a string `policyCid`) the pin is
/// unreadable, never silently the older approval.
fn pinned_cid(root: &Path, is_steward: &dyn Fn(&str) -> bool) -> Pin {
    use chrono::DateTime;
    use elohim_epr_rea::{FlowStore, SidecarFlowStore};

    use crate::flow::read::{is_note, note_view};

    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Pin::None;
    }
    let records = match SidecarFlowStore::open(root).and_then(|s| s.records()) {
        Ok(r) => r,
        Err(e) => return Pin::Unreadable(e.to_string()),
    };
    type Newest = Option<((DateTime<chrono::FixedOffset>, usize), Option<String>)>;
    let mut newest: Newest = None;
    for (seq, (cid, record)) in records.iter().enumerate() {
        let elohim_epr_rea::FlowRecord::Event(event) = record else {
            continue;
        };
        if !is_note(event) {
            continue;
        }
        let view = note_view(cid, event);
        if view.kind != "run:ruling" || view.subject != POLICY_REL_PATH {
            continue;
        }
        let Ok(at) = DateTime::parse_from_rfc3339(&view.occurred_at) else {
            return Pin::Unreadable(format!(
                "a ruling on the policy has an unparsable occurred_at `{}`",
                view.occurred_at
            ));
        };
        let steward_authored = view
            .steward
            .as_deref()
            .is_some_and(|s| view.actor == s && is_steward(s));
        if !steward_authored {
            continue;
        }
        let named = view
            .reason
            .as_deref()
            .and_then(|r| serde_json::from_str::<Value>(r).ok())
            .filter(|r| r["purpose"] == POLICY_PURPOSE)
            .and_then(|r| r["policyCid"].as_str().map(str::to_owned));
        let key = (at, seq);
        if newest.as_ref().is_none_or(|(held, _)| *held < key) {
            newest = Some((key, named));
        }
    }
    match newest {
        None => Pin::None,
        Some((_, Some(cid))) => Pin::Cid(cid),
        Some((_, None)) => {
            Pin::Unreadable("the steward's newest ruling on the policy is malformed".into())
        }
    }
}

/// The away rulings the flow sidecar can resolve: steward-authored `--kind ruling` notes, named
/// by their note CID. The reason JSON must carry `purpose: "dialogue-stasis-away"` and a parsable
/// RFC 3339 `expiresAt`; a ruling missing either is not away. A reference not in this list is not away: unverifiable absence is never granted.
fn away_rulings(root: &Path, is_steward: &dyn Fn(&str) -> bool) -> Vec<AwayRuling> {
    use elohim_epr_rea::{FlowStore, SidecarFlowStore};

    use crate::flow::read::{is_note, note_view};

    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Vec::new();
    }
    let Ok(records) = SidecarFlowStore::open(root).and_then(|s| s.records()) else {
        return Vec::new();
    };
    records
        .iter()
        .filter_map(|(cid, record)| {
            let elohim_epr_rea::FlowRecord::Event(event) = record else {
                return None;
            };
            if !is_note(event) {
                return None;
            }
            let view = note_view(cid, event);
            let steward = view.steward.as_deref()?;
            if view.kind != "run:ruling" || view.actor != steward || !is_steward(steward) {
                return None;
            }
            // An away ruling names its purpose AND carries a parsable expiry; anything else is not
            // away (a ruling that never expires is not something this reader invents).
            let reason = view
                .reason
                .as_deref()
                .and_then(|r| serde_json::from_str::<Value>(r).ok())?;
            if reason["purpose"] != AWAY_PURPOSE {
                return None;
            }
            let expires_at = DateTime::parse_from_rfc3339(reason["expiresAt"].as_str()?)
                .ok()
                .map(|t| t.with_timezone(&Utc))?;
            Some(AwayRuling {
                id: view.cid,
                expires_at: Some(expires_at),
            })
        })
        .collect()
}

/// The away rulings in force for this checkout.
fn checkout_away_rulings(ctx: &Ctx) -> Vec<AwayRuling> {
    let root = project_root(&ctx.cwd).unwrap_or_else(|| ctx.cwd.clone());
    match ctx.steward_of_record.as_deref() {
        Some(only) => away_rulings(&root, &|s| s == only),
        None => away_rulings(&root, &|s| is_collective_steward(&root, s)),
    }
}

/// Whether `human` stands as a PERSON steward of this checkout's collective, natively: an active,
/// non-fixture Steward whose affiliation says `Person` (an agent holding the role is not one).
fn is_collective_steward(root: &Path, human: &str) -> bool {
    matches!(
        crate::flow::memory::person_steward_refusal(root, human),
        Ok(None)
    )
}

/// Read the policy file and the pin under the checkout root and assess them.
fn assess_policy(ctx: &Ctx) -> Result<PolicyAssessment, StasisError> {
    let root = project_root(&ctx.cwd).unwrap_or_else(|| ctx.cwd.clone());
    let file = read_policy_file(&root)?;
    let pin = if file.is_some() {
        match ctx.steward_of_record.as_deref() {
            Some(only) => pinned_cid(&root, &|s| s == only),
            None => pinned_cid(&root, &|s| is_collective_steward(&root, s)),
        }
    } else {
        Pin::None
    };
    Ok(PolicyAssessment::assess(file, pin))
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
    let assessment = match assess_policy(ctx) {
        Ok(a) => a,
        Err(e) => return fail(err, &e),
    };
    let policy = assessment.policy.clone();
    let shadow_label = assessment.label(false);
    let rulings = checkout_away_rulings(ctx);
    // Live records count only while the policy they were decided under is still the pinned one.
    let pinned_now = match &assessment.state {
        PolicyState::PinnedMatch { cid } => cid.clone(),
        _ => String::new(),
    };
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
            .with_clock(&ctx.clock.boot_id, ctx.clock.uptime_seconds)
            .with_rulings(&rulings)
            .with_pinned_cid(&pinned_now);
            let (mut report, _) =
                match session_stasis::fold_root(&root, flags.session.as_deref(), &o) {
                    Ok(r) => r,
                    Err(e) => return fail(err, &e),
                };
            report.policy.clone_from(&shadow_label);
            if let Some(measure) = &flags.emit_count {
                // Policy status goes to stderr so stdout stays the probe's declared output.
                let _ = writeln!(err, "policy: {shadow_label}");
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
                let st = &report.stops;
                line(
                    out,
                    &format!(
                        "stops: accomplished {} exception {} question {} stalled {} unclassified {}",
                        st.accomplished, st.exception, st.question, st.stalled, st.unclassified
                    ),
                );
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
            let live = flags.live || ctx.ladder_env.as_deref() == Some("live");
            if live && ctx.ladder_env.as_deref() == Some("off") {
                // The kill switch wins over --live: no decision, and nothing new is journaled.
                line(
                    out,
                    &json!({"effect": "none", "shadow": false, "live": true, "ladder": "off"})
                        .to_string(),
                );
                return ExitCode::SUCCESS;
            }
            if live {
                // The event was read after the first assessment; judge the policy as it stands now.
                let assessment = match assess_policy(ctx) {
                    Ok(a) => a,
                    Err(e) => return fail(err, &e),
                };
                let classifier = classifier(&flags, ctx);
                return decide_live(
                    &assessment,
                    session,
                    &request,
                    &root,
                    &classifier,
                    ctx,
                    out,
                    err,
                );
            }
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
                Ok(mut decision) => {
                    decision.policy.clone_from(&shadow_label);
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
            .with_clock(&ctx.clock.boot_id, ctx.clock.uptime_seconds)
            .with_rulings(&rulings)
            .with_pinned_cid(&pinned_now);
            let (_, folds) = match session_stasis::fold_root(&root, flags.session.as_deref(), &o) {
                Ok(r) => r,
                Err(e) => return fail(err, &e),
            };
            let sessions: Vec<Value> = folds
                .iter()
                .map(|f| session_stasis::status(f, ctx.clock.wall))
                .collect();
            let mut sessions = sessions;
            for s in &mut sessions {
                s["policy"] = json!(shadow_label);
            }
            let mut body = json!({"policy": shadow_label, "sessions": sessions});
            body["policyState"] = json!(assessment.state_word());
            if let Some(cid) = assessment.cid() {
                body["policyCid"] = json!(cid);
            }
            if flags.json {
                line(out, &body.to_string());
            } else {
                line(
                    out,
                    &format!("policy: {shadow_label} [{}]", assessment.state_word()),
                );
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

/// Live `decide`: act only under a pinned policy. A matching pin lets the regulator deny (park
/// in automated presence, or offer an active learned legible form at rung 0) or do nothing; it
/// never approves. Anything else is inert: exit 3, the reason printed and journaled as an
/// `observation-gap`.
#[allow(
    clippy::too_many_arguments,
    reason = "Live decide threads the verified policy, request, journal root, classifier and injected context unchanged from the dispatcher"
)]
fn decide_live(
    assessment: &PolicyAssessment,
    session: &str,
    request: &DecideRequest,
    root: &Path,
    classifier: &Classifier,
    ctx: &Ctx,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    let Some(reason) = assessment.inert_reason() else {
        let answer = session_stasis::live::decide_live(
            root,
            session,
            request,
            classifier,
            &|| assess_policy(ctx),
            &checkout_away_rulings(ctx),
            &|| ctx.reclock.map_or_else(|| ctx.clock.clone(), |f| f()),
            ctx.segment_limit,
        );
        match answer {
            Ok(LiveDecision::Acted(answer)) => {
                line(out, &serde_json::to_string(&answer).unwrap_or_default());
                return ExitCode::SUCCESS;
            }
            Ok(LiveDecision::Inert(reason)) => {
                return inert(root, session, request, &reason, ctx, out, err)
            }
            Err(e) => return fail(err, &e),
        }
    };
    inert(root, session, request, &reason, ctx, out, err)
}

/// The inert answer: journal why, print it, exit 3.
fn inert(
    root: &Path,
    session: &str,
    request: &DecideRequest,
    reason: &str,
    ctx: &Ctx,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    if let Err(e) = session_stasis::record_inert(
        root,
        session,
        &request.agent_id,
        reason,
        &ctx.clock,
        ctx.segment_limit,
    ) {
        // The answer is still inert; only the record failed, and that is worth saying.
        let _ = writeln!(err, "epr session stasis: could not journal the gap: {e}");
    }
    line(
        out,
        &json!({"effect": "none", "inert": true, "reason": reason}).to_string(),
    );
    ExitCode::from(3)
}

/// The native side of the differential check against `stasis-probe`'s permission harness: one
/// JSON line per corpus entry. A corpus line is `{"tool":"Bash","command":"..."}`,
/// `{"tool":"Edit","path":"..."}`, `{"tool":"Bash","shape":"..."}` (an already redacted shape, as `decide` classifies it), or a bare command. A later run diffs `ruleMatch` against
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
        let input = match (
            entry["command"].as_str(),
            entry["path"].as_str(),
            entry["shape"].as_str(),
        ) {
            (Some(c), _, _) => CallInput::Command(c),
            (None, Some(p), _) => CallInput::Path(p),
            (None, None, Some(shape)) => CallInput::Shape(shape),
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

    use elohim_device_runtime::session_stasis::policy::{POLICY_PURPOSE, POLICY_REL_PATH};

    fn ctx(root: &Path) -> Ctx {
        Ctx {
            cwd: root.to_path_buf(),
            clock: ClockReading {
                wall: Utc.with_ymd_and_hms(2026, 10, 31, 10, 5, 0).unwrap(),
                boot_id: "b".into(),
                uptime_seconds: 5.0,
            },
            reclock: None,
            segment_limit: 5 << 20,
            journal_root_env: None,
            config_dir: None,
            stdin: None,
            ladder_env: None,
            steward_of_record: Some("human:matthew".into()),
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

    // ---- slice 6: policy loader, pin check and live refusal ---------------------------------

    use elohim_epr_rea::{
        atom_cid, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
    };

    /// The default policy, as the committed file spells it.
    fn policy_value() -> Value {
        json!({
            "version": 1,
            "never": [
                {"id": "never-approve-prompt", "invariant": "a"},
                {"id": "never-widen-permissions", "invariant": "b"},
                {"id": "never-bypass-ask", "invariant": "c"},
                {"id": "never-circumvent-denial", "invariant": "d"},
                {"id": "never-impersonate-steward", "invariant": "e"},
                {"id": "never-undo-work", "invariant": "f"},
                {"id": "never-edit-policy", "invariant": "g"}
            ],
            "budgets": {"incidentKeeperCalls": 4, "incidentElapsedS": 1800, "incidentTokens": 200000,
                        "shiftKeeperS": 7200, "shiftTokens": 2000000},
            "collaborativeWindowS": 600,
            "stall": {"limit": 300, "unit": "seconds", "thresholdPct": 60},
            "shiftCeiling": {"limit": 1800, "unit": "seconds", "thresholdPct": 100},
            "dwellS": 300,
            "ruleSet": {"shapePatterns": ["cd-compound", "heredoc", "bash-c", "multi-line"],
                        "riskRules": ["deny", "ask"], "precedence": ["deny", "ask", "allow"],
                        "uncertainAs": "risk", "nonAuthorizationConcern": "uncertain",
                        "effectNormalization": "native-session-stasis-classifier-v1"},
            "promotionCriteria": {"cleanProceduralReshapeResolutions": 2, "sameEffectClass": true,
                                  "trigger": "propose",
                                  "activationRequires": ["completed-passing-other-family-audit",
                                                         "steward-standing-ruling"],
                                  "standingRuling": "2026-10-30: auto-promote, audited"},
            "rulingValidation": {"kind": "ruling", "authority": "steward", "unexpired": true,
                                 "scopeCoversCase": true,
                                 "policyPin": "latest-steward-ruling-exact-cid",
                                 "missingOrMismatchedPolicy": "inert-live-exit-3"}
        })
    }

    /// A checkout root for the policy and its sidecar, marked so `project_root` stops here.
    fn checkout(dir: &Path) {
        fs::create_dir_all(dir.join(".claude")).unwrap();
        fs::write(dir.join(".claude/settings.json"), "{}").unwrap();
    }

    fn write_policy(dir: &Path, v: &Value) -> String {
        let path = dir.join(POLICY_REL_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = serde_json::to_vec_pretty(v).unwrap();
        fs::write(&path, &bytes).unwrap();
        elohim_device_runtime::session_stasis::policy::policy_cid(&bytes)
    }

    /// Record a `--kind ruling` note on the policy path, as `epr flow note` would.
    fn rule(dir: &Path, provider: &str, at: &str, named: &str) {
        let reason = json!({"purpose": POLICY_PURPOSE, "policyPath": POLICY_REL_PATH,
                            "policyCid": named, "version": 1})
        .to_string();
        rule_raw(dir, provider, "human:matthew", at, &reason);
    }

    fn rule_raw(dir: &Path, provider: &str, steward: &str, at: &str, reason: &str) -> String {
        let event = FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef(provider.into()),
            receiver: AgentRef("repo:test".into()),
            resource: atom_cid(&POLICY_REL_PATH.to_string()).unwrap(),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "run-note".into(),
            },
            process: None,
            in_scope_of: atom_cid(&"scope".to_string()).unwrap(),
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            classified_as: vec![
                "run:ruling".into(),
                POLICY_REL_PATH.into(),
                format!("reason:{reason}"),
                format!("steward:{steward}"),
            ],
            occurred_at: at.into(),
        };
        SidecarFlowStore::open(dir)
            .unwrap()
            .append(FlowRecord::Event(event))
            .unwrap()
            .to_string()
    }

    fn decide(root: &Path, live: bool, c: &mut Ctx) -> (ExitCode, String) {
        c.stdin = Some(json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "shape": "git rebase main"}).to_string());
        let mut args = vec![
            "decide",
            "--session",
            "s1",
            "--event",
            "-",
            "--journal-root",
        ];
        args.push(root.to_str().unwrap());
        if live {
            args.push("--live");
        }
        let (exit, out, _) = call(&args, c);
        (exit, out)
    }

    fn last_gap(root: &Path) -> Option<String> {
        let last: Value = serde_json::from_str(
            fs::read_to_string(root.join("s1/events.jsonl"))
                .unwrap()
                .lines()
                .last()
                .unwrap(),
        )
        .unwrap();
        (last["kind"] == "observation-gap").then(|| last["gap"]["reason"].as_str().unwrap().into())
    }

    #[test]
    fn shadow_without_a_policy_file_is_unchanged_in_every_way() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let (exit, out) = decide(&root, false, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        assert_eq!(
            out.trim(),
            r#"{"effect":"none","message":null,"incidentId":null,"rung":0,"shadow":true,"policy":"absent (defaults, shadow)","unmeasured":false}"#
        );
        // A stale ruling with no file changes nothing either.
        rule(
            dir.path(),
            "human:matthew",
            "2026-10-31T09:00:00Z",
            "bafkreiX",
        );
        let (_, again) = decide(&root, false, &mut c);
        assert_eq!(again, out);
    }

    #[test]
    fn live_without_a_policy_is_inert_exit_3_and_the_reason_is_journaled() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!((&v["effect"], &v["inert"]), (&json!("none"), &json!(true)));
        let reason = v["reason"].as_str().unwrap().to_owned();
        assert!(reason.contains("absent"), "{reason}");
        assert_eq!(last_gap(&root), Some(reason.clone()));
        // The same invocation again does not pile up identical gaps.
        let before = fs::read_to_string(root.join("s1/events.jsonl")).unwrap();
        let (exit, _) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert_eq!(
            fs::read_to_string(root.join("s1/events.jsonl")).unwrap(),
            before
        );
        // STASIS_LADDER=live selects the same mode.
        c.ladder_env = Some("live".into());
        let (exit, _) = decide(&root, false, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
    }

    #[test]
    fn editing_the_policy_without_a_matching_ruling_makes_live_inert() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let pinned = write_policy(dir.path(), &policy_value());

        // No ruling yet: unpinned.
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert!(out.contains("no steward ruling pins policy"), "{out}");

        // The steward pins it: live passes, still with no effect in this slice.
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &pinned);
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            (&v["effect"], &v["live"], &v["policyCid"]),
            (&json!("none"), &json!(true), &json!(pinned))
        );
        assert_eq!(v["policy"], format!("file {pinned} (pinned), live"));

        // The policy is edited; the ruling still names the old bytes.
        let mut edited = policy_value();
        edited["dwellS"] = json!(60);
        let new_cid = write_policy(dir.path(), &edited);
        assert_ne!(new_cid, pinned);
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        let v: Value = serde_json::from_str(&out).unwrap();
        let reason = v["reason"].as_str().unwrap();
        assert!(
            reason.contains(&new_cid) && reason.contains(&pinned),
            "{reason}"
        );
        assert_eq!(last_gap(&root).as_deref(), Some(reason));

        // Shadow keeps answering, names the file as unpinned, and exits 0.
        let (exit, out) = decide(&root, false, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["policy"], format!("file {new_cid} (unpinned), shadow"));
        assert_eq!(v["shadow"], true);

        // A later ruling for the new bytes supersedes the earlier one.
        rule(
            dir.path(),
            "human:matthew",
            "2026-10-31T09:30:00Z",
            &new_cid,
        );
        let (exit, _) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
    }

    #[test]
    fn a_ruling_from_an_agent_or_for_other_bytes_never_pins() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let cid = write_policy(dir.path(), &policy_value());
        rule(
            dir.path(),
            "agent:keeper@opus-5",
            "2026-10-31T09:00:00Z",
            &cid,
        );
        let (exit, _) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        // A steward ruling that names other bytes is a mismatch, even next to an agent's match.
        rule(
            dir.path(),
            "human:matthew",
            "2026-10-31T09:10:00Z",
            "bafkreiOTHER",
        );
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert!(out.contains("bafkreiOTHER"), "{out}");
    }

    #[test]
    fn an_unparsable_policy_is_inert_even_when_a_ruling_names_its_bytes() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let mut bad = policy_value();
        bad["surprise"] = json!(1);
        let cid = write_policy(dir.path(), &bad);
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert!(out.contains("does not parse"), "{out}");
    }

    #[test]
    fn status_reports_absent_unpinned_matched_and_mismatched() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let state = |c: &mut Ctx| -> (String, Option<String>) {
            let (_, out, _) = call(
                &[
                    "status",
                    "--json",
                    "--session",
                    "s1",
                    "--journal-root",
                    root.to_str().unwrap(),
                ],
                c,
            );
            let v: Value = serde_json::from_str(&out).unwrap();
            (
                v["policyState"].as_str().unwrap().to_owned(),
                v["policyCid"].as_str().map(str::to_owned),
            )
        };
        assert_eq!(state(&mut c), ("absent".into(), None));
        let cid = write_policy(dir.path(), &policy_value());
        assert_eq!(state(&mut c), ("unpinned".into(), Some(cid.clone())));
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        assert_eq!(state(&mut c), ("pinned-match".into(), Some(cid.clone())));
        let mut edited = policy_value();
        edited["dwellS"] = json!(60);
        let new_cid = write_policy(dir.path(), &edited);
        assert_eq!(state(&mut c), ("pinned-mismatch".into(), Some(new_cid)));
    }

    // ---- live effects under a pinned policy -------------------------------------------------

    const SHAPE: &str = "ls -la\nls";

    /// One session with an OPEN authorization incident `s1:2` (a shape-class call), the human
    /// last witnessed at t=0.
    fn open_journal(dir: &Path, extra: &[Value]) -> PathBuf {
        open_journal_shaped(
            dir,
            extra,
            SHAPE,
            json!({"disposition": "collaborative", "basis": {"lastWitnessedAgeSeconds": 0.0, "rulingRef": null}}),
        )
    }

    fn open_journal_shaped(dir: &Path, extra: &[Value], shape: &str, presence: Value) -> PathBuf {
        let root = dir.join("stasis");
        let s = root.join("s1");
        fs::create_dir_all(&s).unwrap();
        let corr = json!({"incidentId": "s1:2", "confidence": "high", "method": "tool-use-id"});
        let mut lines = vec![
            event(
                1,
                0,
                "shift-marker",
                json!({"shift": {"id": "shift-7", "edge": "open"}, "presence": presence}),
            ),
            event(
                2,
                10,
                "permission-requested",
                json!({"tool": "Bash", "shape": shape, "source": {"hookEvent": "PermissionRequest"},
                       "correlation": corr, "presence": presence}),
            ),
        ];
        lines.extend(extra.iter().cloned());
        fs::write(
            s.join("events.jsonl"),
            lines.iter().map(|l| format!("{l}\n")).collect::<String>(),
        )
        .unwrap();
        root
    }

    fn settings(dir: &Path) -> String {
        settings_with(dir, &["Bash(git *rebase*)", "Bash(ls *secret*)"])
    }

    fn settings_with(dir: &Path, asks: &[&str]) -> String {
        let p = dir.join("settings.json");
        fs::write(&p, json!({"permissions": {"ask": asks}}).to_string()).unwrap();
        p.to_str().unwrap().to_owned()
    }

    fn live(
        root: &Path,
        settings: &str,
        hook: &str,
        shape: &str,
        c: &mut Ctx,
    ) -> (ExitCode, Value) {
        c.stdin = Some(
            json!({"hook_event_name": hook, "tool_name": "Bash", "shape": shape,
                   "correlation": {"incidentId": "s1:2"}})
            .to_string(),
        );
        let (exit, out, _) = call(
            &[
                "decide",
                "--live",
                "--session",
                "s1",
                "--event",
                "-",
                "--settings",
                settings,
                "--journal-root",
                root.to_str().unwrap(),
            ],
            c,
        );
        (exit, serde_json::from_str(&out).unwrap_or(Value::Null))
    }

    fn pinned_checkout(dir: &Path, policy: &Value) -> String {
        checkout(dir);
        let cid = write_policy(dir, policy);
        rule(dir, "human:matthew", "2026-10-31T09:00:00Z", &cid);
        cid
    }

    fn events_of(root: &Path) -> Vec<Value> {
        fs::read_to_string(root.join("s1/events.jsonl"))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn at_automated(c: &mut Ctx) {
        // 30 minutes after the last witness: past T_c, so automated presence.
        c.clock.wall = Utc.with_ymd_and_hms(2026, 10, 31, 10, 30, 0).unwrap();
    }

    #[test]
    fn automated_presence_parks_the_permission_request_and_journals_the_park() {
        let dir = tempfile::tempdir().unwrap();
        let cid = pinned_checkout(dir.path(), &policy_value());
        let root = open_journal(dir.path(), &[]);
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        at_automated(&mut c);
        let (exit, v) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        assert_eq!(v["effect"], "deny");
        assert_eq!(v["incidentId"], "s1:2");
        assert_eq!(
            (v["rung"].clone(), v["shadow"].clone(), v["live"].clone()),
            (json!(0), json!(false), json!(true))
        );
        assert_eq!(v["policyCid"], json!(cid));
        assert_eq!(
            v["message"],
            "parked as stasis `s1:2`. Do not retry this effect by any tool or spelling. Continue independent authorized work. Guidance will arrive."
        );
        let last = events_of(&root).pop().unwrap();
        assert_eq!(last["kind"], "consultation");
        assert_eq!(
            (
                last["consultation"]["pivot"].clone(),
                last["consultation"]["decision"].clone()
            ),
            (json!("park"), json!("Permit"))
        );
        // The park shows in status as a pending park; a repeat denies again, writing nothing.
        let n = events_of(&root).len();
        let (_, again) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(again["effect"], "deny");
        assert_eq!(events_of(&root).len(), n);
        let (_, out, _) = call(
            &[
                "status",
                "--json",
                "--session",
                "s1",
                "--settings",
                &st,
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        let status: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(status["sessions"][0]["pendingParks"], json!(["s1:2"]));
    }

    #[test]
    fn collaborative_presence_does_nothing_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        pinned_checkout(dir.path(), &policy_value());
        let root = open_journal(dir.path(), &[]);
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        let before = events_of(&root).len();
        let (exit, v) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        assert_eq!(
            (v["effect"].clone(), v["message"].clone()),
            (json!("none"), Value::Null)
        );
        assert_eq!(events_of(&root).len(), before);
    }

    #[test]
    fn an_unpinned_policy_stays_inert_even_with_an_open_automated_incident() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        write_policy(dir.path(), &policy_value());
        let root = open_journal(dir.path(), &[]);
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        at_automated(&mut c);
        let (exit, v) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert_eq!(v["inert"], true);
        assert!(events_of(&root).iter().all(|e| e["kind"] != "consultation"));
    }

    fn learned_policy(c: &mut Ctx, dir: &Path, st: &str) -> Value {
        learned_policy_for(c, dir, st, "ls -la\nls", SHAPE)
    }

    /// A policy whose one active form is for `shape`, with the effect `classify` derives for
    /// `command`.
    fn learned_policy_for(c: &mut Ctx, dir: &Path, st: &str, command: &str, shape: &str) -> Value {
        // The checkout marker decides the main checkout, hence whether targets resolve.
        checkout(dir);
        let corpus = dir.join("corpus.jsonl");
        fs::write(
            &corpus,
            json!({"tool": "Bash", "shape": command}).to_string() + "\n",
        )
        .unwrap();
        let (_, out, _) = call(
            &[
                "classify",
                "--corpus",
                corpus.to_str().unwrap(),
                "--settings",
                st,
            ],
            c,
        );
        let classified = serde_json::from_str::<Value>(out.lines().next().unwrap()).unwrap();
        let effect = classified["classification"]["effect"].clone();
        let mut p = policy_value();
        p["learnedForms"] = json!([{
            "name": "ls-in-dir", "tool": "Bash", "shape": shape, "effectClass": effect,
            "legibleForm": "ls <path>", "status": "active",
            "auditVerdict": "approved", "auditRef": "audit-1", "rulingRef": "ruling-1"
        }]);
        p
    }

    fn concern_of(c: &mut Ctx, dir: &Path, st: &str, command: &str) -> String {
        checkout(dir);
        let corpus = dir.join("concern.jsonl");
        fs::write(
            &corpus,
            json!({"tool": "Bash", "shape": command}).to_string() + "\n",
        )
        .unwrap();
        let (_, out, _) = call(
            &[
                "classify",
                "--corpus",
                corpus.to_str().unwrap(),
                "--settings",
                st,
            ],
            c,
        );
        serde_json::from_str::<Value>(out.lines().next().unwrap()).unwrap()["classification"]
            ["concern"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a_learned_form_reshapes_once_and_a_second_reshape_refers() {
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        // A shape call: no settings rule matches `cd ... && ls`.
        let policy = learned_policy(&mut c, dir.path(), &st);
        pinned_checkout(dir.path(), &policy);
        let root = open_journal(dir.path(), &[]);
        let (exit, v) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        assert_eq!(v["effect"], "deny");
        assert_eq!(
            v["message"],
            "use the legible form: ls <path> (same effect, passes the same permission checks)"
        );
        let last = events_of(&root).pop().unwrap();
        assert_eq!(last["consultation"]["pivot"], "reshape");
        // The allowance is spent: the next one Refers, journaled once.
        let (_, second) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(
            (second["effect"].clone(), second["refer"].clone()),
            (json!("none"), json!(true))
        );
        let refers = |r: &Path| {
            events_of(r)
                .iter()
                .filter(|e| e["consultation"]["decision"] == "Refer")
                .count()
        };
        assert_eq!(refers(&root), 1);
        let (_, third) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(third["refer"], true);
        assert_eq!(refers(&root), 1);
    }

    #[test]
    fn a_risk_call_with_a_learned_form_gets_no_reshape() {
        // L8: the incident IS a risk call, the learned form's shape AND effect are the risk
        // call's own, so nothing but the risk guard in live.rs stands between it and a reshape.
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        assert_eq!(
            concern_of(&mut c, dir.path(), &st, "git rebase main"),
            "risk"
        );
        let policy = learned_policy_for(
            &mut c,
            dir.path(),
            &st,
            "git rebase main",
            "git rebase main",
        );
        pinned_checkout(dir.path(), &policy);
        let root = open_journal_shaped(dir.path(), &[], "git rebase main", collaborative());
        let before = events_of(&root).len();
        let (_, v) = live(&root, &st, "PreToolUse", "git rebase main", &mut c);
        assert_eq!(v["effect"], "none");
        assert_eq!(events_of(&root).len(), before);
    }

    fn collaborative() -> Value {
        json!({"disposition": "collaborative", "basis": {"lastWitnessedAgeSeconds": 0.0, "rulingRef": null}})
    }

    #[test]
    fn a_risk_candidate_on_a_shape_incident_gets_no_reshape() {
        // The incident is shape class; the CANDIDATE call matches an ask rule yet has the same
        // effect and a form of its own shape.
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        let risky = "ls secret\nls";
        assert_eq!(concern_of(&mut c, dir.path(), &st, risky), "risk");
        let policy = learned_policy_for(&mut c, dir.path(), &st, "ls -la\nls", risky);
        pinned_checkout(dir.path(), &policy);
        let root = open_journal(dir.path(), &[]);
        let before = events_of(&root).len();
        let (_, v) = live(&root, &st, "PreToolUse", risky, &mut c);
        assert_eq!(v["effect"], "none");
        assert_eq!(events_of(&root).len(), before);
    }

    #[test]
    fn an_incident_that_is_not_shape_class_gets_no_reshape_for_a_shape_looking_candidate() {
        // L3: the incident was recorded as risk; a later multi-line candidate with the same
        // effect looks like a shape call, but the incident's own concern decides.
        let dir = tempfile::tempdir().unwrap();
        let st = settings_with(dir.path(), &["Bash(git rebase main)"]);
        let mut c = ctx(dir.path());
        let candidate = "git rebase other\nls";
        assert_eq!(
            concern_of(&mut c, dir.path(), &st, "git rebase main"),
            "risk"
        );
        assert_eq!(concern_of(&mut c, dir.path(), &st, candidate), "shape");
        let policy = learned_policy_for(&mut c, dir.path(), &st, candidate, candidate);
        // Same effect as the incident's, so only the incident's concern can stop the reshape.
        let incident_effect = learned_policy_for(&mut c, dir.path(), &st, "git rebase main", "x");
        assert_eq!(
            policy["learnedForms"][0]["effectClass"],
            incident_effect["learnedForms"][0]["effectClass"]
        );
        pinned_checkout(dir.path(), &policy);
        let root = open_journal_shaped(dir.path(), &[], "git rebase main", collaborative());
        let before = events_of(&root).len();
        let (_, v) = live(&root, &st, "PreToolUse", candidate, &mut c);
        assert_eq!(v["effect"], "none", "{v}");
        assert_eq!(events_of(&root).len(), before);
    }

    #[test]
    fn an_unresolved_effect_never_gets_a_reshape() {
        // L4: `git -C <path>` redacts to an unresolved target; structural equality is not
        // sameness.
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        let shape = "cd <path> && ls";
        assert_eq!(concern_of(&mut c, dir.path(), &st, shape), "shape");
        let mut policy = learned_policy_for(&mut c, dir.path(), &st, "ls -la\nls", shape);
        // Give the form the very effect the classifier derives for this shape.
        let corpus = dir.path().join("u.jsonl");
        fs::write(
            &corpus,
            json!({"tool": "Bash", "shape": "cd <path> && ls"}).to_string() + "\n",
        )
        .unwrap();
        let (_, out, _) = call(
            &[
                "classify",
                "--corpus",
                corpus.to_str().unwrap(),
                "--settings",
                &st,
            ],
            &mut c,
        );
        let classified: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
        assert_eq!(
            classified["classification"]["effect"]["target"],
            "unresolved"
        );
        policy["learnedForms"][0]["effectClass"] = classified["classification"]["effect"].clone();
        pinned_checkout(dir.path(), &policy);
        let root = open_journal_shaped(dir.path(), &[], shape, collaborative());
        let before = events_of(&root).len();
        let (_, v) = live(&root, &st, "PreToolUse", shape, &mut c);
        assert_eq!(v["effect"], "none", "{v}");
        assert_eq!(events_of(&root).len(), before);
    }

    #[test]
    fn a_form_that_is_inactive_or_invalid_never_reshapes() {
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        // Proposed, not active: valid policy, no reshape.
        let mut p = learned_policy(&mut c, dir.path(), &st);
        p["learnedForms"][0]["status"] = json!("proposed");
        pinned_checkout(dir.path(), &p);
        let root = open_journal(dir.path(), &[]);
        let (exit, v) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(
            (code(exit), v["effect"].clone()),
            (code(ExitCode::SUCCESS), json!("none"))
        );
        // L7: an active form missing its audit makes the whole policy invalid: inert, exit 3.
        let mut bad = learned_policy(&mut c, dir.path(), &st);
        bad["learnedForms"][0]["auditVerdict"] = json!("rejected");
        let cid = write_policy(dir.path(), &bad);
        rule(dir.path(), "human:matthew", "2026-10-31T09:40:00Z", &cid);
        let (exit, v) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert_eq!(v["inert"], true);
    }

    /// A keeper `proceed` record: the coordinator's live shape, or a shadow one.
    fn proceed_record(live: bool, cid: &str) -> Value {
        let mut c = json!({"phase": "complete", "rung": 2, "level": "C2", "binding": "b",
                           "decision": "Permit", "pivot": "proceed", "reason": "r"});
        let mut e = event(
            3,
            60,
            "consultation",
            json!({"correlation": {"incidentId": "s1:2", "confidence": "high", "method": "t"}}),
        );
        if live {
            c["live"] = json!(true);
            e["agentId"] = json!("stasis-coordinator");
            e["regulator"] = json!({"kind": "coordinator"});
            e["policyCid"] = json!(cid);
        }
        e["consultation"] = c;
        // Keep the away basis: the last event decides presence.
        e["presence"] = collaborative();
        e
    }

    #[test]
    fn a_live_proceed_is_downgraded_to_park_only_in_automated_presence() {
        // M2: a shadow proceed never reaches the downgrade; a live one needs automated presence.
        for (live_proceed, automated, writes) in [
            (false, false, false),
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let cid = pinned_checkout(dir.path(), &policy_value());
            let root = open_journal(dir.path(), &[proceed_record(live_proceed, &cid)]);
            let st = settings(dir.path());
            let mut c = ctx(dir.path());
            if automated {
                at_automated(&mut c);
            }
            let before = events_of(&root).len();
            let _ = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
            let added = events_of(&root).len() - before;
            // Automated + live proceed: exactly one park (the downgrade), then the deny rides it.
            let expect = usize::from(writes);
            let parks = events_of(&root)
                .iter()
                .filter(|e| e["consultation"]["pivot"] == "park")
                .count();
            if live_proceed && automated {
                assert_eq!((added, parks), (1, 1), "live proceed, automated");
                let last = events_of(&root).pop().unwrap();
                assert!(last["consultation"]["reason"]
                    .as_str()
                    .unwrap()
                    .contains("proceed downgraded to park"));
            } else if automated {
                // A shadow proceed in automated presence: the ordinary park, not a downgrade.
                assert_eq!(added, 1, "shadow proceed, automated");
                let last = events_of(&root).pop().unwrap();
                assert!(!last["consultation"]["reason"]
                    .as_str()
                    .unwrap()
                    .contains("proceed downgraded"));
            } else {
                assert_eq!((added, expect), (0, 0), "{live_proceed} collaborative");
            }
        }
    }

    // ---- pin-check hardening (review of the slice-6 leg) -------------------------------------

    fn state_of(dir: &Path, root: &Path, c: &mut Ctx) -> String {
        let (_, out, _) = call(
            &[
                "status",
                "--json",
                "--session",
                "s1",
                "--journal-root",
                root.to_str().unwrap(),
            ],
            c,
        );
        let _ = dir;
        serde_json::from_str::<Value>(&out).unwrap()["policyState"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a_forged_or_mismatched_steward_never_pins() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let cid = write_policy(dir.path(), &policy_value());
        let reason = json!({"purpose": POLICY_PURPOSE, "policyCid": cid}).to_string();
        // Well-formed, but it names a different steward than the steward of record.
        rule_raw(
            dir.path(),
            "human:mallory",
            "human:mallory",
            "2026-10-31T09:00:00Z",
            &reason,
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "unpinned");
        // The steward slot is the steward's, but the acting identity is not.
        rule_raw(
            dir.path(),
            "human:mallory",
            "human:matthew",
            "2026-10-31T09:01:00Z",
            &reason,
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "unpinned");
        // No steward slot at all is no steward.
        rule_raw(
            dir.path(),
            "human:matthew",
            "",
            "2026-10-31T09:02:00Z",
            &reason,
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "unpinned");
        // The real steward, acting as themselves, pins.
        rule_raw(
            dir.path(),
            "human:matthew",
            "human:matthew",
            "2026-10-31T09:03:00Z",
            &reason,
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "pinned-match");
    }

    #[test]
    fn a_malformed_newest_ruling_makes_the_pin_unreadable_not_the_older_approval() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let cid = write_policy(dir.path(), &policy_value());
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        assert_eq!(state_of(dir.path(), &root, &mut c), "pinned-match");
        rule_raw(
            dir.path(),
            "human:matthew",
            "human:matthew",
            "2026-10-31T09:30:00Z",
            "not json",
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "pin-unreadable");
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert!(out.contains("unreadable"), "{out}");
        // A well-formed newer ruling restores the pin.
        rule(dir.path(), "human:matthew", "2026-10-31T09:45:00Z", &cid);
        assert_eq!(state_of(dir.path(), &root, &mut c), "pinned-match");
    }

    #[test]
    fn timestamps_parse_as_rfc3339_and_ties_go_to_append_order() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let cid = write_policy(dir.path(), &policy_value());
        // Same instant: the later append wins, whichever way the offsets are spelled.
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        rule(
            dir.path(),
            "human:matthew",
            "2026-10-31T11:00:00+02:00",
            "bafkreiOTHER",
        );
        assert_eq!(state_of(dir.path(), &root, &mut c), "pinned-mismatch");
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        assert_eq!(state_of(dir.path(), &root, &mut c), "pinned-match");
        // A timestamp that is not RFC 3339 cannot win lexically: it makes the pin unreadable.
        rule(dir.path(), "human:matthew", "zzz", &cid);
        assert_eq!(state_of(dir.path(), &root, &mut c), "pin-unreadable");
    }

    #[test]
    fn an_unreadable_flow_sidecar_is_inert_in_live() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        write_policy(dir.path(), &policy_value());
        fs::create_dir_all(dir.path().join(".eprfs/status")).unwrap();
        fs::write(dir.path().join(".eprfs/status/flows.jsonl"), "{ torn\n").unwrap();
        assert_eq!(state_of(dir.path(), &root, &mut c), "pin-unreadable");
        let (exit, _) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::from(3)));
    }

    #[test]
    fn the_ladder_off_kill_switch_beats_live_and_journals_nothing() {
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let root = journal(dir.path());
        let mut c = ctx(dir.path());
        let cid = write_policy(dir.path(), &policy_value());
        rule(dir.path(), "human:matthew", "2026-10-31T09:00:00Z", &cid);
        c.ladder_env = Some("off".into());
        let before = fs::read_to_string(root.join("s1/events.jsonl")).unwrap();
        let (exit, out) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            (&v["effect"], &v["shadow"], &v["ladder"]),
            (&json!("none"), &json!(false), &json!("off"))
        );
        assert_eq!(
            fs::read_to_string(root.join("s1/events.jsonl")).unwrap(),
            before
        );
        // Even with no policy at all, off answers none rather than inert.
        fs::remove_file(dir.path().join(POLICY_REL_PATH)).unwrap();
        let (exit, _) = decide(&root, true, &mut c);
        assert_eq!(code(exit), code(ExitCode::SUCCESS));
    }

    // ---- ladder-review regressions (L1, L2, L5) -----------------------------------------------

    #[test]
    fn only_a_person_can_stand_as_steward_of_record() {
        // L1: an agent holding the Steward role, or a missing affiliation, is not a person.
        use crate::flow::memory::person_refusal;
        use eprfs_agent::memory::MemberKind;
        assert!(person_refusal("human:matthew", Some(MemberKind::Person)).is_none());
        assert!(person_refusal("agent:keeper@opus-5", Some(MemberKind::ElohimAgent)).is_some());
        assert!(person_refusal("human:matthew", Some(MemberKind::ElohimAgent)).is_some());
        assert!(person_refusal("human:matthew", Some(MemberKind::Collective)).is_some());
        assert!(person_refusal("agent:keeper@opus-5", Some(MemberKind::Person)).is_some());
        assert!(person_refusal("human:matthew", None).is_some());
    }

    fn away(presence_ruling: &str) -> Value {
        json!({"disposition": "automated",
               "basis": {"lastWitnessedAgeSeconds": 0.0, "rulingRef": presence_ruling}})
    }

    fn away_park(dir: &Path, ruling: &str) -> Value {
        pinned_checkout(dir, &policy_value());
        let st = settings(dir);
        let root = open_journal_shaped(dir, &[], SHAPE, away(ruling));
        let mut c = ctx(dir);
        live(&root, &st, "PermissionRequest", SHAPE, &mut c).1
    }

    fn away_ruling(dir: &Path, provider: &str, reason: &str) -> String {
        checkout(dir);
        rule_raw(dir, provider, provider, "2026-10-31T08:00:00Z", reason)
    }

    #[test]
    fn an_unresolvable_or_expired_away_ruling_is_not_away() {
        // L5: a ruling reference the sidecar cannot resolve parks nothing...
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(away_park(dir.path(), "bogus-ruling")["effect"], "none");
        // ...an expired steward ruling parks nothing...
        let dir = tempfile::tempdir().unwrap();
        let expired = away_ruling(
            dir.path(),
            "human:matthew",
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T09:00:00Z"}"#,
        );
        assert_eq!(away_park(dir.path(), &expired)["effect"], "none");
        // ...a ruling from a non-steward is not a ruling...
        let dir = tempfile::tempdir().unwrap();
        let forged = away_ruling(
            dir.path(),
            "human:mallory",
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T23:00:00Z"}"#,
        );
        assert_eq!(away_park(dir.path(), &forged)["effect"], "none");
        // ...and an unexpired steward ruling is away: the park happens.
        let dir = tempfile::tempdir().unwrap();
        let held = away_ruling(
            dir.path(),
            "human:matthew",
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T23:00:00Z"}"#,
        );
        assert_eq!(away_park(dir.path(), &held)["effect"], "deny");
        // R1: no expiry, an unparsable expiry, or another purpose is not an away ruling.
        for reason in [
            r#"{"purpose":"dialogue-stasis-away"}"#,
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"someday"}"#,
            r#"{"purpose":"away","expiresAt":"2026-10-31T23:00:00Z"}"#,
            r#"not json"#,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let cid = away_ruling(dir.path(), "human:matthew", reason);
            assert_eq!(away_park(dir.path(), &cid)["effect"], "none", "{reason}");
        }
    }

    #[test]
    fn an_away_ruling_survives_a_regulator_record_and_live_records_carry_provenance() {
        // N1 + N2: the park the regulator writes keeps the away ruling and disposition on the
        // LAST event (so the next fold is still automated), names the regulator and the pinned
        // policy CID, and is written under the regulator's fixed agent id.
        let dir = tempfile::tempdir().unwrap();
        checkout(dir.path());
        let ruling = away_ruling(
            dir.path(),
            "human:matthew",
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T23:00:00Z"}"#,
        );
        let cid = pinned_checkout(dir.path(), &policy_value());
        let st = settings(dir.path());
        let root = open_journal_shaped(dir.path(), &[], SHAPE, away(&ruling));
        let mut c = ctx(dir.path());
        let (_, v) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(v["effect"], "deny");
        let last = events_of(&root).pop().unwrap();
        assert_eq!(last["presence"]["basis"]["rulingRef"], json!(ruling));
        assert_eq!(last["presence"]["disposition"], "automated");
        assert_eq!(last["agentId"], "stasis-regulator");
        assert_eq!(last["regulator"]["kind"], "native-decide");
        assert_eq!(last["policyCid"], json!(cid));
        // The next fold still reads the ruling: presence stays automated.
        let (_, out, _) = call(
            &[
                "status",
                "--json",
                "--session",
                "s1",
                "--settings",
                &st,
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        let status: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            status["sessions"][0]["presence"]["disposition"],
            "automated"
        );
        // And the live park counts: it closes the stall.
        let (_, out, _) = call(
            &[
                "fold",
                "--json",
                "--session",
                "s1",
                "--settings",
                &st,
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        let fold: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(fold["incidents"][0]["closedBy"], "parked");
        // Edit the policy: the same record no longer verifies and folds as shadow.
        let mut edited = policy_value();
        edited["dwellS"] = json!(60);
        write_policy(dir.path(), &edited);
        let (_, out, _) = call(
            &[
                "fold",
                "--json",
                "--session",
                "s1",
                "--settings",
                &st,
                "--journal-root",
                root.to_str().unwrap(),
            ],
            &mut c,
        );
        let fold: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(fold["incidents"][0]["closedBy"], Value::Null);
    }

    #[test]
    fn shadow_records_never_count_as_live_park_or_reshape_allowance() {
        // N3: a SHADOW park does not suppress the live park...
        let dir = tempfile::tempdir().unwrap();
        pinned_checkout(dir.path(), &policy_value());
        let shadow_park = event(
            3,
            20,
            "consultation",
            json!({"correlation": {"incidentId": "s1:2", "confidence": "high", "method": "t"},
                   "consultation": {"rung": 0, "level": "C1", "binding": "b", "decision": "Permit",
                                    "pivot": "park", "reason": "shadow"}}),
        );
        let root = open_journal_shaped(dir.path(), &[shadow_park], SHAPE, away_basis());
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        at_automated(&mut c);
        let (_, v) = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
        assert_eq!(v["effect"], "deny");
        let last = events_of(&root).pop().unwrap();
        assert_eq!(last["consultation"]["live"], true);
        // ...and a SHADOW reshape does not consume the live allowance.
        let dir = tempfile::tempdir().unwrap();
        let st = settings(dir.path());
        let mut c = ctx(dir.path());
        let policy = learned_policy(&mut c, dir.path(), &st);
        pinned_checkout(dir.path(), &policy);
        let shadow_reshape = event(
            3,
            20,
            "consultation",
            json!({"correlation": {"incidentId": "s1:2", "confidence": "high", "method": "t"},
                   "consultation": {"rung": 0, "level": "C1", "binding": "b", "decision": "Permit",
                                    "pivot": "reshape", "reason": "shadow"}}),
        );
        let root = open_journal(dir.path(), &[shadow_reshape]);
        let (_, v) = live(&root, &st, "PreToolUse", SHAPE, &mut c);
        assert_eq!(v["effect"], "deny", "{v}");
    }

    fn away_basis() -> Value {
        collaborative()
    }

    #[test]
    fn a_policy_edited_while_decide_waits_for_the_lock_is_inert_with_the_post_edit_cid() {
        // M5: the test holds journal.lock, starts `decide --live`, edits the policy, releases
        // the lock; the decision re-verifies under the lock and is inert on the NEW bytes.
        use elohim_device_runtime::session_stasis::journal::Lock;
        let dir = tempfile::tempdir().unwrap();
        let old_cid = pinned_checkout(dir.path(), &policy_value());
        let root = open_journal(dir.path(), &[]);
        let st = settings(dir.path());
        let held = Lock::exclusive(&root.join("s1")).unwrap();
        let handle = {
            let (root, st, cwd) = (root.clone(), st.clone(), dir.path().to_path_buf());
            std::thread::spawn(move || {
                let mut c = ctx(&cwd);
                let r = live(&root, &st, "PermissionRequest", SHAPE, &mut c);
                (r.0, r.1)
            })
        };
        // Give the decision time to pass its first read and block on the lock.
        std::thread::sleep(std::time::Duration::from_millis(700));
        let mut edited = policy_value();
        edited["dwellS"] = json!(60);
        let new_cid = write_policy(dir.path(), &edited);
        assert_ne!(new_cid, old_cid);
        let before = events_of(&root).len();
        drop(held);
        let (exit, v) = handle.join().unwrap();
        assert_eq!(code(exit), code(ExitCode::from(3)));
        assert_eq!(v["inert"], true);
        assert!(v["reason"].as_str().unwrap().contains(&new_cid), "{v}");
        assert!(events_of(&root).iter().all(|e| e["kind"] != "consultation"));
        // Only the inert gap was added.
        assert!(events_of(&root).len() <= before + 1);
    }

    fn later() -> ClockReading {
        ClockReading {
            wall: Utc.with_ymd_and_hms(2026, 10, 31, 10, 8, 0).unwrap(),
            boot_id: "b".into(),
            uptime_seconds: 8.0,
        }
    }

    #[test]
    fn a_ruling_that_expires_before_the_lock_is_acquired_is_not_away() {
        // M4: the clock is re-read after the lock; the ruling is valid at process start (10:05)
        // but expired at the decision (10:08), when the human is still within T_c.
        let dir = tempfile::tempdir().unwrap();
        let ruling = away_ruling(
            dir.path(),
            "human:matthew",
            r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T10:06:00Z"}"#,
        );
        let run = |reclock: Option<fn() -> ClockReading>| {
            let dir_ = tempfile::tempdir().unwrap();
            let r = away_ruling(
                dir_.path(),
                "human:matthew",
                r#"{"purpose":"dialogue-stasis-away","expiresAt":"2026-10-31T10:06:00Z"}"#,
            );
            pinned_checkout(dir_.path(), &policy_value());
            let st = settings(dir_.path());
            let root = open_journal_shaped(dir_.path(), &[], SHAPE, away(&r));
            let mut c = ctx(dir_.path());
            c.reclock = reclock;
            live(&root, &st, "PermissionRequest", SHAPE, &mut c).1
        };
        let _ = ruling;
        // Same clock throughout: the ruling holds, so the call is parked.
        assert_eq!(run(None)["effect"], "deny");
        // Re-read after the lock finds it expired: not away, nothing parked.
        assert_eq!(run(Some(later))["effect"], "none");
    }
}
