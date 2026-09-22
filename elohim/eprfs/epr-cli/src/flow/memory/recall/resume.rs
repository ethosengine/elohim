//! S2 (2026-09-22 recall-Codex-trail sprint) — `open --purpose resume --habit <id>`: a read-only
//! view for an agent resuming interrupted work on ONE named concern, composed only from sources
//! this executor already knows how to read (the habit register, the habit's own atom, and bounded
//! git queries) — never a new ledger, never a write beyond the ordinary private session state
//! `open` already produces.
//!
//! The register path and the habit-atom walk are discovery's own (`HABITS_REL`,
//! `find_habit_atom`); this seam adds only what resuming needs beyond them — the last delta WITH
//! its parsed date, and the bounded git reads.
use std::path::Component;

use super::discovery::{find_habit_atom, HABITS_REL};
use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Bounded reads
// ───────────────────────────────────────────────────────────────────────────────────────────────

enum Bounded {
    Missing,
    TooLarge,
    Ok(Vec<u8>),
}

fn read_bounded(root: &Path, rel: &str, budget: usize) -> FlowResult<Bounded> {
    let path = match confine_under(root, &root.join(rel)) {
        Ok(path) => path,
        Err(_) => return Ok(Bounded::Missing),
    };
    if !path.is_file() {
        return Ok(Bounded::Missing);
    }
    let mut data = Vec::new();
    File::open(&path)
        .and_then(|file| {
            file.take(budget as u64 + 1)
                .read_to_end(&mut data)
                .map(|_| ())
        })
        .map_err(|source| FlowError::Read { path, source })?;
    if data.len() > budget {
        return Ok(Bounded::TooLarge);
    }
    Ok(Bounded::Ok(data))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The habit register row
// ───────────────────────────────────────────────────────────────────────────────────────────────

struct HabitRegisterRow {
    status: String,
    active: bool,
    checks: Vec<String>,
    declared: Option<String>,
}

/// One habit by exact id, read fail-closed from the register (mirrors `discovery.rs`'s bootstrap
/// register read: an absent, unreadable or malformed register REFUSES this whole view, since
/// resuming a NAMED habit without being able to read the register at all would orient the reader
/// from nothing). An id the register genuinely does not carry is a DIFFERENT refusal — the
/// register read fine, the id named is simply not one it declares.
fn read_habit_row(
    root: &Path,
    contract: &Contract,
    id: &str,
    usage: &mut Value,
) -> FlowResult<HabitRegisterRow> {
    let budget = contract.limit_usize("habit_register_bytes");
    let data = match read_bounded(root, HABITS_REL, budget)? {
        Bounded::Ok(data) => data,
        Bounded::Missing => {
            return Err(refused(format!(
                "resume cannot read the habit register: {HABITS_REL}: absent"
            )))
        }
        Bounded::TooLarge => {
            return Err(refused(format!(
                "resume cannot read the habit register: {HABITS_REL}: exceeds its declared budget"
            )))
        }
    };
    add_usage(usage, &json!({"habit_register_bytes": data.len()}));
    let doc: serde_yaml::Value = serde_yaml::from_slice(&data).map_err(|error| {
        refused(format!(
            "resume cannot read the habit register: {HABITS_REL}: {error}"
        ))
    })?;
    let habits = doc
        .get("habits")
        .and_then(serde_yaml::Value::as_sequence)
        .ok_or_else(|| {
            refused(format!(
                "resume cannot read the habit register: {HABITS_REL}: no `habits:` sequence"
            ))
        })?;
    let row = habits
        .iter()
        .find(|h| h.get("id").and_then(serde_yaml::Value::as_str) == Some(id))
        .ok_or_else(|| {
            refused(format!(
                "no habit named `{id}` in the register ({HABITS_REL})"
            ))
        })?;
    let status = row
        .get("status")
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let active = row
        .get("active")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false);
    let checks = row
        .get("checks")
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(serde_yaml::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let declared = row
        .get("declared")
        .or_else(|| row.get("atom"))
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_string);
    Ok(HabitRegisterRow {
        status,
        active,
        checks,
        declared,
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The habit atom — located, its newest delta and its `refs:`
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The newest `DELTA`/`GREEN`/`RED[ WRITTEN]` paragraph's first line (clipped), its 1-based
/// inclusive line range, and the date parsed from its own leading `KEYWORD YYYY-MM-DD` token —
/// `discovery.rs`'s `atom_last_delta` extended with date parsing (see the module doc for why this
/// is a duplicate, not an import).
fn last_delta(text: &str) -> Option<(String, String, Option<String>)> {
    let mut lines = text.lines().enumerate();
    if lines.next().map(|(_, line)| line.trim()) != Some("---") {
        return None;
    }
    let body_start = lines
        .find(|(_, line)| line.trim() == "---")
        .map(|(index, _)| index + 2)?;
    let all_lines: Vec<&str> = text.lines().collect();
    let (mut start, mut end) = (None, body_start);
    for (offset, line) in all_lines.iter().enumerate().skip(body_start - 1) {
        let line_no = offset + 1;
        if line.trim().is_empty() {
            if start.is_some() {
                break;
            }
            continue;
        }
        start.get_or_insert(line_no);
        end = line_no;
    }
    let start = start?;
    let first_line = all_lines[start - 1];
    Some((
        clip(first_line, 160),
        format!("{start}:{end}"),
        parse_delta_date(first_line),
    ))
}

/// `DELTA 2026-09-11 (…)` / `GREEN 2026-09-11 (…)` / `RED WRITTEN 2026-09-11 (…)` / `RED
/// 2026-09-11 (…)` → `2026-09-11`. `None` for a first line that carries no such leading token —
/// rendered as an honest omission rather than a fabricated date.
fn parse_delta_date(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("DELTA ")
        .or_else(|| line.strip_prefix("GREEN "))
        .or_else(|| line.strip_prefix("RED WRITTEN "))
        .or_else(|| line.strip_prefix("RED "))?;
    let token = rest.split_whitespace().next()?;
    let bytes = token.as_bytes();
    let digit = |b: u8| b.is_ascii_digit();
    let valid = bytes.len() == 10
        && bytes[..4].iter().copied().all(digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().copied().all(digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().copied().all(digit);
    valid.then(|| token.to_string())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Path helpers — plans (`refs:`) and concern paths (checks:/refs: tokens that exist)
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn first_token(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or_default()
}

/// `true` when `token` is a repository-relative path (no leading `/`, no `..` component) that
/// exists on disk under `root`. A token that escapes the repository is reported absent rather
/// than resolved — this is an informational existence check, not a read.
fn repo_path_exists(root: &Path, token: &str) -> bool {
    if token.is_empty()
        || Path::new(token).is_absolute()
        || Path::new(token)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return false;
    }
    root.join(token).exists()
}

/// Every `/`-bearing token in `text`, loosely tokenized (this is a convenience index into
/// checks/refs prose, never an authoritative parse) — used only to grow `concern_paths` with
/// paths the register/atom prose itself names and that actually exist.
fn path_like_tokens(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ',' | ';' | '"'))
        .map(|s| s.trim_matches(|c| c == '.' || c == ':'))
        .filter(|s| s.contains('/') && !s.is_empty())
        .map(str::to_string)
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Bounded git — since-delta activity, local-branch standing
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Run one bounded git query, tolerant by construction: any failure to spawn or a nonzero exit
/// becomes an `Err(message)` the caller folds into `omissions`, never a crash and never a panic.
fn git_lines(root: &Path, args: &[String]) -> Result<Vec<String>, String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = crate::process::build_command("git", &refs, root, &[])
        .output()
        .map_err(|error| format!("git {} failed to spawn: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} exited {}: {}",
            args.join(" "),
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "by signal".into()),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

/// `sha\tdate\tsubject` (the `%H%x09%ad%x09%s` format every query below uses) → its three fields.
fn parse_commit_line(line: &str) -> Option<(String, String, String)> {
    let mut parts = line.splitn(3, '\t');
    let sha = parts.next()?.to_string();
    if sha.is_empty() {
        return None;
    }
    let date = parts.next().unwrap_or_default().to_string();
    let subject = parts.next().unwrap_or_default().to_string();
    Some((sha, date, subject))
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(12)]
}

/// Commits since the habit's own last-evidence date that either NAME the habit (`--grep`) or
/// TOUCH one of its concern paths (`-- <paths>`) — run as two separate bounded `git log` calls and
/// unioned by sha, each row tagged `matched_by: subject|path|both`. Bounded by
/// `limits.resume_commits` on each call and again after the union.
fn since_delta(
    root: &Path,
    contract: &Contract,
    habit_id: &str,
    delta_date: &str,
    concern_paths: &[String],
) -> Value {
    let limit = contract.limit_usize("resume_commits").max(1);
    let mut omissions: Vec<String> = Vec::new();
    let mut by_sha: BTreeMap<String, (String, String, BTreeSet<&'static str>)> = BTreeMap::new();

    let subject_args: Vec<String> = vec![
        "log".into(),
        "--since".into(),
        delta_date.into(),
        format!("--grep={habit_id}"),
        "--format=%H%x09%ad%x09%s".into(),
        "--date=iso-strict".into(),
        "-n".into(),
        limit.to_string(),
    ];
    match git_lines(root, &subject_args) {
        Ok(lines) => {
            for line in lines {
                if let Some((sha, date, subject)) = parse_commit_line(&line) {
                    by_sha
                        .entry(sha)
                        .and_modify(|(d, s, matched)| {
                            *d = date.clone();
                            *s = subject.clone();
                            matched.insert("subject");
                        })
                        .or_insert_with(|| {
                            let mut matched = BTreeSet::new();
                            matched.insert("subject");
                            (date, subject, matched)
                        });
                }
            }
        }
        Err(message) => omissions.push(format!("commits-by-subject query failed: {message}")),
    }

    if concern_paths.is_empty() {
        omissions
            .push("no concern paths resolved from checks/refs; path-based match skipped".into());
    } else {
        let mut path_args: Vec<String> = vec![
            "log".into(),
            "--since".into(),
            delta_date.into(),
            "--format=%H%x09%ad%x09%s".into(),
            "--date=iso-strict".into(),
            "-n".into(),
            limit.to_string(),
            "--".into(),
        ];
        path_args.extend(concern_paths.iter().cloned());
        match git_lines(root, &path_args) {
            Ok(lines) => {
                for line in lines {
                    if let Some((sha, date, subject)) = parse_commit_line(&line) {
                        by_sha
                            .entry(sha)
                            .and_modify(|(d, s, matched)| {
                                *d = date.clone();
                                *s = subject.clone();
                                matched.insert("path");
                            })
                            .or_insert_with(|| {
                                let mut matched = BTreeSet::new();
                                matched.insert("path");
                                (date, subject, matched)
                            });
                    }
                }
            }
            Err(message) => omissions.push(format!("commits-by-path query failed: {message}")),
        }
    }

    let mut rows: Vec<(String, String, String, String)> = by_sha
        .into_iter()
        .map(|(sha, (date, subject, matched))| {
            let matched_by = if matched.len() == 2 {
                "both".to_string()
            } else {
                matched.into_iter().next().unwrap_or("unknown").to_string()
            };
            (sha, date, subject, matched_by)
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    rows.truncate(limit);
    let commits: Vec<Value> = rows
        .iter()
        .map(|(sha, date, subject, matched_by)| {
            json!({"sha": short_sha(sha), "date": date, "subject": subject, "matched_by": matched_by})
        })
        .collect();
    json!({
        "commits": commits,
        "count": commits.len(),
        "bound": limit,
        "omissions": omissions,
    })
}

/// Commits on `HEAD` not yet on its upstream, and local branches whose recent commits (bounded to
/// 3 per branch, per the sprint plan) name the habit since its own delta date. Branch listing
/// itself is capped at `limits.resume_commits` too — the same declared bound reused for a second,
/// unrelated item count, documented here rather than inventing a second ungoverned constant.
fn local_work(root: &Path, contract: &Contract, habit_id: &str, delta_date: Option<&str>) -> Value {
    let mut omissions: Vec<String> = Vec::new();

    let ahead_count = match git_lines(
        root,
        &["rev-list".into(), "--count".into(), "@{u}..HEAD".into()],
    ) {
        Ok(lines) => lines.first().and_then(|s| s.trim().parse::<u64>().ok()),
        Err(message) => {
            omissions.push(format!(
                "no upstream configured, or rev-list failed: {message}"
            ));
            None
        }
    };
    let mut ahead_commits: Vec<Value> = Vec::new();
    if ahead_count.is_some_and(|n| n > 0) {
        let bound = contract.limit_usize("resume_commits").max(1);
        match git_lines(
            root,
            &[
                "log".into(),
                "@{u}..HEAD".into(),
                "--format=%H%x09%s".into(),
                "-n".into(),
                bound.to_string(),
            ],
        ) {
            Ok(lines) => {
                for line in lines {
                    if let Some((sha, subject)) = line.split_once('\t') {
                        ahead_commits.push(json!({"sha": short_sha(sha), "subject": subject}));
                    }
                }
            }
            Err(message) => {
                omissions.push(format!("ahead-of-upstream subject query failed: {message}"))
            }
        }
    }

    let mut branches: Vec<Value> = Vec::new();
    match git_lines(
        root,
        &[
            "for-each-ref".into(),
            "refs/heads".into(),
            "--format=%(refname:short)".into(),
        ],
    ) {
        Ok(names) => match delta_date {
            Some(date) => {
                let cap = contract.limit_usize("resume_commits").max(1);
                for name in names.into_iter().take(cap) {
                    let args: Vec<String> = vec![
                        "log".into(),
                        name.clone(),
                        "--since".into(),
                        date.to_string(),
                        format!("--grep={habit_id}"),
                        "--format=%H%x09%ad%x09%s".into(),
                        "--date=iso-strict".into(),
                        "-n".into(),
                        "3".into(),
                    ];
                    match git_lines(root, &args) {
                        Ok(lines) => {
                            let commits: Vec<Value> = lines
                                .iter()
                                .filter_map(|line| parse_commit_line(line))
                                .map(|(sha, date, subject)| {
                                    json!({"sha": short_sha(&sha), "date": date, "subject": subject})
                                })
                                .collect();
                            if !commits.is_empty() {
                                branches.push(json!({"name": name, "commits": commits}));
                            }
                        }
                        Err(message) => {
                            omissions.push(format!("branch {name} scan failed: {message}"))
                        }
                    }
                }
            }
            None => omissions.push("no delta date available; local-branch scan skipped".into()),
        },
        Err(message) => omissions.push(format!("local branch listing failed: {message}")),
    }

    json!({
        "ahead_of_upstream": {
            "count": ahead_count,
            "commits": ahead_commits,
        },
        "branches": branches,
        "omissions": omissions,
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The view
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `open --purpose resume --habit <id>`'s whole contribution: composes `view["resume"]` and its
/// Linked-choice actions, and folds its own usage into `view["usage"]`. Refuses (via `?`) only
/// when the register itself cannot be read, or when `habit_id` names no row in it — everything
/// else this function discovers (a missing atom, a git query that fails, no upstream) is a named
/// omission, never a fault.
pub(super) fn resume_view(
    args: &Args,
    contract: &Contract,
    habit_id: &str,
    view: &mut Value,
) -> FlowResult<()> {
    let root = &args.root;
    let mut usage = view["usage"].take();

    let register = read_habit_row(root, contract, habit_id, &mut usage)?;
    let atom_budget = contract.limit_usize("habit_register_bytes");
    let atom_path: Option<String> = match &register.declared {
        Some(declared) => confine_under(root, &root.join(declared))
            .ok()
            .filter(|p| p.is_file())
            .map(|p| rel_to_root(root, &p)),
        None => {
            find_habit_atom(root, contract, habit_id, atom_budget).map(|p| rel_to_root(root, &p))
        }
    };

    let mut dynamic_omissions: Vec<String> = Vec::new();
    let mut delta_first_line: Option<String> = None;
    let mut delta_lines: Option<String> = None;
    let mut delta_date: Option<String> = None;
    let mut plans: Vec<Value> = Vec::new();
    let mut concern_texts: Vec<String> = register.checks.clone();

    match &atom_path {
        Some(atom_rel) => match read_bounded(root, atom_rel, atom_budget)? {
            Bounded::Ok(data) => {
                add_usage(&mut usage, &json!({"habit_atom_bytes": data.len()}));
                let text = String::from_utf8_lossy(&data).into_owned();
                match last_delta(&text) {
                    Some((line, lines, date)) => {
                        if date.is_none() {
                            dynamic_omissions.push(format!(
                                "{atom_rel}'s newest evidence line carries no parseable leading date"
                            ));
                        }
                        delta_first_line = Some(line);
                        delta_lines = Some(lines);
                        delta_date = date;
                    }
                    None => dynamic_omissions
                        .push(format!("{atom_rel} carries no evidence-ledger entry")),
                }
                let frontmatter = super::super::super::parse_frontmatter(&text);
                for entry in frontmatter.list("refs") {
                    let token = first_token(entry);
                    plans.push(json!({
                        "ref": clip(entry, 200),
                        "path": token,
                        "exists": repo_path_exists(root, token),
                    }));
                    concern_texts.push(entry.clone());
                }
            }
            Bounded::Missing => dynamic_omissions.push(format!("{atom_rel} could not be read")),
            Bounded::TooLarge => {
                dynamic_omissions.push(format!("{atom_rel} exceeds its declared budget"))
            }
        },
        None => dynamic_omissions.push(format!(
            "habit atom for {habit_id} was not located within budget"
        )),
    }

    let mut concern_path_set: BTreeSet<String> = BTreeSet::new();
    if let Some(atom_rel) = &atom_path {
        if let Some(dir) = Path::new(atom_rel).parent() {
            let dir_text = dir.to_string_lossy().to_string();
            if !dir_text.is_empty() {
                concern_path_set.insert(dir_text);
            }
        }
    }
    for text in &concern_texts {
        for token in path_like_tokens(text) {
            if repo_path_exists(root, &token) {
                concern_path_set.insert(token);
            }
        }
    }
    let concern_paths: Vec<String> = concern_path_set.into_iter().collect();

    let since = match &delta_date {
        Some(date) => since_delta(root, contract, habit_id, date, &concern_paths),
        None => json!({
            "commits": [],
            "count": 0,
            "bound": contract.limit_usize("resume_commits"),
            "omissions": ["no delta date available; commits-since cannot be bounded"],
        }),
    };
    let local = local_work(root, contract, habit_id, delta_date.as_deref());

    let commit_count = since["count"].as_u64().unwrap_or(0);
    let verdict = if commit_count > 0 {
        let first_check = register
            .checks
            .first()
            .cloned()
            .unwrap_or_else(|| "no check declared".to_string());
        format!(
            "implemented-but-unverified: {commit_count} commit(s) postdate the last evidence \
             ({}); the habit's status ({}) was recorded before them — rerun: {}",
            delta_date.as_deref().unwrap_or("unknown"),
            if register.status.is_empty() {
                "unstated"
            } else {
                register.status.as_str()
            },
            clip(&first_check, 200),
        )
    } else if let Some(date) = &delta_date {
        format!("evidence current: no commit on the concern's paths since {date}")
    } else {
        "evidence currency unknown: this habit's atom carries no parseable delta date".to_string()
    };

    let mut omissions = vec![
        "CI/Jenkins state is not read by this view.".to_string(),
        "A commit subject or a recent binary is not evidence of coverage.".to_string(),
        "Worktrees of other sessions are listed only by branch name.".to_string(),
    ];
    omissions.extend(dynamic_omissions);
    for source in [&since, &local] {
        for message in source["omissions"].as_array().cloned().unwrap_or_default() {
            if let Some(text) = message.as_str() {
                omissions.push(text.to_string());
            }
        }
    }

    // Linked choices, before `plans`/`atom_path` are moved into the final JSON below: the atom's
    // newest delta first (or the atom's own head when it carries no delta), then `source` each
    // EXISTING plan — bounded to a handful so a habit with a long `refs:` list does not turn one
    // resume view into a wall of outline commands. The first check renders verbatim inside
    // `habit.checks` already (S2's own "as text, not executed" requirement), so it needs no
    // separate action here.
    match (&atom_path, &delta_lines) {
        (Some(atom_rel), Some(lines)) => push_action(
            view,
            action(
                args,
                &format!("Read {habit_id}'s last delta — {atom_rel} ({lines})"),
                "read",
                &[("path", json!(atom_rel)), ("lines", json!(lines))],
            ),
        ),
        (Some(atom_rel), None) => push_action(
            view,
            action(
                args,
                &format!("Read {habit_id}'s atom — {atom_rel}"),
                "read",
                &[("path", json!(atom_rel)), ("lines", json!("1:20"))],
            ),
        ),
        (None, _) => {}
    }
    for plan in plans.iter().filter(|p| p["exists"] == json!(true)).take(5) {
        if let Some(path) = plan["path"].as_str() {
            push_action(
                view,
                action(
                    args,
                    &format!("Outline {path}"),
                    "source",
                    &[("path", json!(path))],
                ),
            );
        }
    }

    view["resume"] = json!({
        "habit": {
            "id": habit_id,
            "status": register.status,
            "active": register.active,
            "checks": register.checks,
            "atom": atom_path,
            "last_delta": {"text": delta_first_line, "lines": delta_lines, "date": delta_date},
        },
        "plans": plans,
        "concern_paths": concern_paths,
        "since_delta": since,
        "local_work": local,
        "verdict": verdict,
        "omissions": omissions,
    });
    view["usage"] = usage;
    Ok(())
}
