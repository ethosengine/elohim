//! The native stall classifier: concern class, effect normalization and the Claude Code
//! permission-rule subset the dialogue-stasis design specifies (Classification).
//!
//! Inputs are the raw facts slice-3 adapters record (`tool`, a redacted `shape`) or, for the
//! differential corpus, a real command. The classifier never sets authority; it only decides
//! which pivots are admissible, and it errs toward risk: a rule it cannot parse, a specifier
//! form outside the subset, or a match that redaction makes undecidable is `uncertain`, which
//! the loop treats as risk.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// Which pivots are admissible for a stalled call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConcernClass {
    /// No ask/deny rule matches and a declared shape pattern does: a more legible form exists.
    Shape,
    /// An explicit ask or deny rule matches.
    Risk,
    /// Everything else; treated as risk.
    Uncertain,
}

/// What kind of stall is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StallClass {
    /// A permission prompt.
    Authorization,
    /// An unanswered question from the agent.
    Question,
    /// The steward is waiting on progress.
    Silence,
}

/// The target kind of a Bash effect, decided by the ordered rules in the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// A network remote.
    Remote,
    /// Under `<main checkout>/.worktrees/`.
    Worktree,
    /// Under the main checkout root, outside `.worktrees/`.
    MainCheckout,
    /// Anywhere else.
    Other,
    /// Redaction or missing context makes the kind undecidable; never equal to anything.
    Unresolved,
}

/// The normalized effect tuple. Two effects are the same class exactly when the tuples are
/// equal and no part is unresolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectClass {
    /// The tool name (`Bash`, `Edit`, ...), or a stall pseudo-tool.
    pub tool: String,
    /// Bash: the first command word.
    pub command: Option<String>,
    /// Bash: the subcommand of a multi-command tool.
    pub subcommand: Option<String>,
    /// Bash: the target kind.
    pub target: Option<Target>,
    /// Edit/Write: the path-class glob.
    pub path_class: Option<String>,
}

impl EffectClass {
    fn bare(tool: &str) -> Self {
        Self {
            tool: tool.to_owned(),
            command: None,
            subcommand: None,
            target: None,
            path_class: None,
        }
    }

    /// The effect of a stall that is not a tool call.
    pub fn for_stall(stall: StallClass) -> Self {
        Self::bare(match stall {
            StallClass::Authorization => "unknown",
            StallClass::Question => "(question)",
            StallClass::Silence => "(silence)",
        })
    }

    /// True when both effects are fully resolved and equal.
    pub fn same_effect(&self, other: &Self) -> bool {
        self == other
            && self.target != Some(Target::Unresolved)
            && self.path_class.as_deref() != Some("<unresolved>")
    }
}

impl std::fmt::Display for EffectClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tool)?;
        if let Some(c) = &self.command {
            write!(f, ":{c}")?;
        }
        if let Some(s) = &self.subcommand {
            write!(f, " {s}")?;
        }
        if let Some(t) = self.target {
            let word = serde_json::to_value(t)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            write!(f, "@{word}")?;
        }
        if let Some(p) = &self.path_class {
            write!(f, "@{p}")?;
        }
        Ok(())
    }
}

/// Where effect normalization may look for paths.
#[derive(Debug, Clone, Default)]
pub struct EffectContext {
    /// The main checkout root; without it path targets are `unresolved`.
    pub main_checkout: Option<PathBuf>,
    /// The session's working directory, for relative paths and commands with no path argument.
    pub cwd: Option<PathBuf>,
}

/// The call input the classifier sees.
#[derive(Debug, Clone, Copy)]
pub enum CallInput<'a> {
    /// A real command (differential corpus).
    Command(&'a str),
    /// A redacted shape, as journalled by the adapters.
    Shape(&'a str),
    /// A file path for Read/Edit/Write.
    Path(&'a str),
    /// No usable input was recorded.
    Opaque,
}

/// The classifier's verdict with its evidence.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Classification {
    /// The concern class.
    pub concern: ConcernClass,
    /// A stable code naming why.
    pub reason: String,
    /// The rule that decided it (or is undecidable), when one did.
    pub matched_rule: Option<String>,
    /// `ask`, `deny`, `possible`, `unparseable` or `none`.
    pub rule_match: String,
    /// The declared shape patterns the input matched.
    pub patterns: Vec<String>,
    /// The normalized effect.
    pub effect: EffectClass,
}

// ---------------------------------------------------------------------------------------------
// Wildcard matching. `*` matches any run of characters (newlines included, which errs toward a
// match, i.e. toward risk). A redacted placeholder in the text may stand for anything, which
// yields a *possible* match rather than a definite one.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Ch(char),
    Placeholder,
}

fn units(text: &str, placeholders: bool) -> Vec<Unit> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if placeholders && chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|c| *c == '>') {
                let name: String = chars[i + 1..i + end].iter().collect();
                if matches!(
                    name.as_str(),
                    "path" | "ref" | "url" | "num" | "glob" | "str"
                ) {
                    out.push(Unit::Placeholder);
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(Unit::Ch(chars[i]));
        i += 1;
    }
    out
}

fn wild_match(pattern: &str, text: &str, placeholders: bool) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt = units(text, placeholders);
    let (m, n) = (pat.len(), txt.len());
    let mut dp = vec![vec![false; n + 1]; m + 1];
    dp[m][n] = true;
    for i in (0..=m).rev() {
        for j in (0..=n).rev() {
            if i == m && j == n {
                continue;
            }
            let mut ok = false;
            if j < n && txt[j] == Unit::Placeholder {
                // A placeholder may be empty, or absorb pattern characters.
                ok |= dp[i][j + 1];
                if i < m {
                    ok |= dp[i + 1][j];
                }
            }
            if i < m && pat[i] == '*' {
                ok |= dp[i + 1][j] || (j < n && dp[i][j + 1]);
            }
            if i < m && j < n && pat[i] != '*' && txt[j] == Unit::Ch(pat[i]) {
                ok |= dp[i + 1][j + 1];
            }
            dp[i][j] = ok;
        }
    }
    dp[0][0]
}

// ---------------------------------------------------------------------------------------------
// A small shell lexer: enough to find compounds, heredocs, `bash -c` and command words. It is
// not a shell parser; heredoc bodies are ignored and anything unterminated degrades toward
// "more structure", never less.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Op(String),
}

fn lex(command: &str) -> (Vec<Tok>, bool) {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut heredoc = false;
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    let flush = |toks: &mut Vec<Tok>, cur: &mut String, in_word: &mut bool| {
        if *in_word {
            toks.push(Tok::Word(std::mem::take(cur)));
            *in_word = false;
        }
    };
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    cur.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    cur.push(chars[i]);
                    i += 1;
                }
            }
            '\\' if i + 1 < chars.len() => {
                in_word = true;
                i += 1;
                cur.push(chars[i]);
            }
            ' ' | '\t' | '\r' => flush(&mut toks, &mut cur, &mut in_word),
            '\n' => {
                flush(&mut toks, &mut cur, &mut in_word);
                if heredoc {
                    // The body of a heredoc is data, not commands.
                    break;
                }
                toks.push(Tok::Op("\n".into()));
            }
            ';' | '&' | '|' => {
                flush(&mut toks, &mut cur, &mut in_word);
                let mut op = String::from(c);
                while i + 1 < chars.len() && matches!(chars[i + 1], ';' | '&' | '|') {
                    i += 1;
                    op.push(chars[i]);
                }
                toks.push(Tok::Op(op));
            }
            '<' if chars.get(i + 1) == Some(&'<') && chars.get(i + 2) != Some(&'<') => {
                heredoc = true;
                flush(&mut toks, &mut cur, &mut in_word);
                toks.push(Tok::Op("<<".into()));
                i += 1;
            }
            '<' if chars.get(i + 1) == Some(&'<') => {
                // A here-string `<<<` is not a heredoc: keep it inside the word.
                in_word = true;
                cur.push_str("<<<");
                i += 2;
            }
            _ => {
                in_word = true;
                cur.push(c);
            }
        }
        i += 1;
    }
    flush(&mut toks, &mut cur, &mut in_word);
    (toks, heredoc)
}

#[derive(Debug, Clone)]
struct Segment {
    words: Vec<String>,
}

/// Split tokens into command segments at every operator; returns the segments and the operators
/// seen between them.
fn segments(toks: &[Tok]) -> (Vec<Segment>, Vec<String>) {
    let mut segs = vec![Segment { words: vec![] }];
    let mut ops = Vec::new();
    for t in toks {
        match t {
            Tok::Word(w) => segs.last_mut().expect("seeded").words.push(w.clone()),
            Tok::Op(op) if op == "<<" => {}
            Tok::Op(op) => {
                ops.push(op.clone());
                segs.push(Segment { words: vec![] });
            }
        }
    }
    segs.retain(|s| !s.words.is_empty());
    (segs, ops)
}

fn strip_prefixes(words: &[String]) -> &[String] {
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let assignment = w.split_once('=').is_some_and(|(k, _)| {
            !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        if assignment || matches!(w.as_str(), "sudo" | "time" | "command" | "exec" | "nohup") {
            i += 1;
        } else {
            break;
        }
    }
    &words[i..]
}

/// The declared shape patterns an input matches: shell compounds around `cd`, heredocs,
/// `bash -c`, and multi-line commands. The list is the initial rule set, not a law.
pub fn shape_patterns(command_or_shape: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let (toks, heredoc) = lex(command_or_shape);
    let (segs, ops) = segments(&toks);
    let compound = ops.iter().any(|o| matches!(o.as_str(), "&&" | ";" | "|"));
    let cd = segs
        .iter()
        .any(|s| strip_prefixes(&s.words).first().is_some_and(|w| w == "cd"));
    if compound && cd {
        found.push("cd-compound");
    }
    if heredoc {
        found.push("heredoc");
    }
    let bash_c = segs.iter().any(|s| {
        let w = strip_prefixes(&s.words);
        let base = w.first().map(|x| x.rsplit('/').next().unwrap_or(x));
        matches!(base, Some("bash" | "sh" | "zsh"))
            && w[1..].iter().any(|a| {
                a == "-c" || (a.starts_with('-') && !a.starts_with("--") && a.contains('c'))
            })
    });
    if bash_c {
        found.push("bash-c");
    }
    if command_or_shape.contains('\n') {
        found.push("multi-line");
    }
    found
}

// ---------------------------------------------------------------------------------------------
// Permission rules.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleKind {
    Ask,
    Deny,
}

#[derive(Debug, Clone)]
enum Spec {
    Any,
    Bash(String),
    Path(String),
    Unparseable,
}

#[derive(Debug, Clone)]
struct Rule {
    kind: RuleKind,
    raw: String,
    tool: String,
    spec: Spec,
}

fn is_edit_family(tool: &str) -> bool {
    matches!(tool, "Edit" | "Write" | "MultiEdit" | "NotebookEdit")
}

impl Rule {
    fn parse(kind: RuleKind, raw: &str) -> Self {
        let trimmed = raw.trim();
        let (tool, spec) = match trimmed.find('(') {
            None => (trimmed.to_owned(), None),
            Some(open) if trimmed.ends_with(')') => (
                trimmed[..open].to_owned(),
                Some(trimmed[open + 1..trimmed.len() - 1].to_owned()),
            ),
            Some(open) => (
                trimmed[..open].to_owned(),
                Some(String::from("\u{0}broken")),
            ),
        };
        let name_ok = !tool.is_empty()
            && tool
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '*');
        let spec = match (name_ok, spec) {
            (false, _) => Spec::Unparseable,
            (true, None) => Spec::Any,
            (true, Some(s)) if s == "\u{0}broken" => Spec::Unparseable,
            (true, Some(s)) if tool == "Bash" => Spec::Bash(s.trim().replace(":*", "*")),
            (true, Some(s)) if tool == "Read" || is_edit_family(&tool) => {
                Spec::Path(s.trim().to_owned())
            }
            (true, Some(_)) => Spec::Unparseable,
        };
        Self {
            kind,
            raw: raw.to_owned(),
            tool,
            spec,
        }
    }

    fn applies_to(&self, tool: &str) -> bool {
        if self.tool.is_empty() {
            return true;
        }
        if self.tool == tool || (is_edit_family(&self.tool) && is_edit_family(tool)) {
            return true;
        }
        if self.tool.starts_with("mcp__") {
            if self.tool.contains('*') {
                return wild_match(&self.tool, tool, false);
            }
            return tool.starts_with(&format!("{}__", self.tool));
        }
        false
    }
}

/// How one rule relates to one call.
enum Relation {
    Definite,
    Possible,
    Unparseable,
    Disjoint,
}

/// The ask and deny rules of the active settings, in the supported subset.
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    /// Read `permissions.ask` and `permissions.deny` from parsed Claude Code settings files.
    pub fn from_settings(files: &[Value]) -> Self {
        let mut rules = Vec::new();
        for file in files {
            for (key, kind) in [("deny", RuleKind::Deny), ("ask", RuleKind::Ask)] {
                if let Some(list) = file
                    .pointer(&format!("/permissions/{key}"))
                    .and_then(Value::as_array)
                {
                    for entry in list {
                        match entry.as_str() {
                            Some(raw) => rules.push(Rule::parse(kind, raw)),
                            // A non-string entry is a rule we cannot parse for any tool.
                            None => rules.push(Rule {
                                kind,
                                raw: entry.to_string(),
                                tool: String::new(),
                                spec: Spec::Unparseable,
                            }),
                        }
                    }
                }
            }
        }
        Self { rules }
    }

    /// Number of ask/deny rules held.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True when no rule is held.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    fn relate(&self, rule: &Rule, tool: &str, input: CallInput<'_>) -> Relation {
        if !rule.applies_to(tool) {
            return Relation::Disjoint;
        }
        match &rule.spec {
            Spec::Any => Relation::Definite,
            Spec::Unparseable => Relation::Unparseable,
            Spec::Bash(pattern) => match input {
                CallInput::Command(command) => {
                    let (toks, _) = lex(command);
                    let (segs, _) = segments(&toks);
                    let mut candidates = vec![command.trim().to_owned()];
                    candidates.extend(segs.iter().map(|s| s.words.join(" ")));
                    if candidates.iter().any(|c| wild_match(pattern, c, false)) {
                        Relation::Definite
                    } else {
                        Relation::Disjoint
                    }
                }
                CallInput::Shape(shape) => {
                    let (toks, _) = lex(shape);
                    let (segs, _) = segments(&toks);
                    let mut candidates = vec![shape.trim().to_owned()];
                    candidates.extend(segs.iter().map(|s| s.words.join(" ")));
                    if candidates.iter().any(|c| wild_match(pattern, c, false)) {
                        Relation::Definite
                    } else if candidates.iter().any(|c| wild_match(pattern, c, true)) {
                        Relation::Possible
                    } else {
                        Relation::Disjoint
                    }
                }
                _ => Relation::Possible,
            },
            Spec::Path(pattern) => match input {
                CallInput::Path(path) => {
                    if path_matches(pattern, path) {
                        Relation::Definite
                    } else {
                        Relation::Disjoint
                    }
                }
                _ => Relation::Possible,
            },
        }
    }
}

/// gitignore-style matching of a Read/Edit/Write specifier against a path: `*` and `?` stay
/// inside a segment, `**` crosses segments, a pattern with no inner `/` matches at any depth, and
/// a leading `/` anchors it.
fn path_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_start_matches("./");
    let anchored = pattern.starts_with('/');
    let pattern = pattern.trim_start_matches('/').trim_end_matches('/');
    let path = path.trim_start_matches("./").trim_start_matches('/');
    let psegs: Vec<&str> = pattern.split('/').collect();
    let tsegs: Vec<&str> = path.split('/').collect();
    let floating = !anchored && psegs.len() == 1;
    let seg_match = |p: &str, t: &str| seg_glob(p, t);
    fn go(p: &[&str], t: &[&str], seg: &dyn Fn(&str, &str) -> bool) -> bool {
        match (p.first(), t.first()) {
            (None, _) => true,
            (Some(&"**"), _) => go(&p[1..], t, seg) || (!t.is_empty() && go(p, &t[1..], seg)),
            (Some(_), None) => false,
            (Some(ps), Some(ts)) => seg(ps, ts) && go(&p[1..], &t[1..], seg),
        }
    }
    if floating {
        (0..tsegs.len()).any(|i| go(&psegs, &tsegs[i..], &seg_match))
    } else {
        go(&psegs, &tsegs, &seg_match)
    }
}

fn seg_glob(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    let (m, n) = (pat.len(), txt.len());
    let mut dp = vec![vec![false; n + 1]; m + 1];
    dp[m][n] = true;
    for i in (0..m).rev() {
        for j in (0..=n).rev() {
            dp[i][j] = match pat[i] {
                '*' => dp[i + 1][j] || (j < n && dp[i][j + 1]),
                '?' => j < n && dp[i + 1][j + 1],
                c => j < n && txt[j] == c && dp[i + 1][j + 1],
            };
        }
    }
    dp[0][0]
}

// ---------------------------------------------------------------------------------------------
// Effect normalization.
// ---------------------------------------------------------------------------------------------

const MULTI_COMMAND: &[&str] = &[
    "git",
    "cargo",
    "pnpm",
    "npm",
    "yarn",
    "just",
    "docker",
    "kubectl",
    "gh",
    "epr",
    "hc",
    "brit",
    "rustup",
    "systemctl",
    "pip",
    "uv",
];

fn looks_like_path(arg: &str) -> bool {
    !arg.starts_with('-')
        && !arg.contains("://")
        && (arg.contains('/')
            || arg.starts_with('.')
            || arg.starts_with('~')
            || arg == "<path>"
            || arg == "<glob>")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn kind_of_path(path: &str, ctx: &EffectContext) -> Target {
    if path.contains('<') || path.starts_with('~') {
        return Target::Unresolved;
    }
    let Some(main) = ctx.main_checkout.as_ref() else {
        return Target::Unresolved;
    };
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(cwd) = ctx.cwd.as_ref() {
        cwd.join(p)
    } else {
        return Target::Unresolved;
    };
    let abs = lexical_normalize(&abs);
    if abs.starts_with(main.join(".worktrees")) {
        Target::Worktree
    } else if abs.starts_with(main) {
        Target::MainCheckout
    } else {
        Target::Other
    }
}

fn effect_for_command(command: &str, ctx: &EffectContext) -> EffectClass {
    let (toks, _) = lex(command);
    let (segs, _) = segments(&toks);
    let mut cd_targets: Vec<String> = Vec::new();
    let mut primary: Option<Vec<String>> = None;
    for seg in &segs {
        let words = strip_prefixes(&seg.words);
        match words.first().map(String::as_str) {
            Some("cd") => {
                if let Some(t) = words[1..].iter().find(|w| !w.starts_with('-')) {
                    cd_targets.push(t.clone());
                }
            }
            Some(_) if primary.is_none() => primary = Some(words.to_vec()),
            _ => {}
        }
    }
    let Some(words) = primary else {
        let mut e = EffectClass::bare("Bash");
        e.command = Some("cd".into());
        e.target = Some(
            cd_targets
                .first()
                .map_or(Target::Unresolved, |t| kind_of_path(t, ctx)),
        );
        return e;
    };
    let cmd = words[0].rsplit('/').next().unwrap_or(&words[0]).to_owned();
    let mut git_c: Vec<String> = Vec::new();
    let mut manifest: Vec<String> = Vec::new();
    let mut sub: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let multi = MULTI_COMMAND.contains(&cmd.as_str());
    let mut i = 1;
    while i < words.len() {
        let w = &words[i];
        if (cmd == "git") && (w == "-C" || w == "-c") {
            if w == "-C" {
                if let Some(v) = words.get(i + 1) {
                    git_c.push(v.clone());
                }
            }
            i += 2;
            continue;
        }
        if w == "--manifest-path" {
            if let Some(v) = words.get(i + 1) {
                manifest.push(v.clone());
            }
            i += 2;
            continue;
        }
        if let Some(v) = w.strip_prefix("--manifest-path=") {
            manifest.push(v.to_owned());
        } else if multi && sub.is_none() && !w.starts_with('-') && !w.starts_with('+') {
            sub = Some(w.clone());
        } else {
            rest.push(w.clone());
        }
        i += 1;
    }
    let positionals: Vec<&String> = rest.iter().filter(|a| !a.starts_with('-')).collect();
    let remote = (cmd == "git"
        && matches!(
            sub.as_deref(),
            Some("push" | "fetch" | "pull" | "clone" | "ls-remote")
        )
        && !positionals.is_empty())
        || matches!(cmd.as_str(), "curl" | "wget" | "ssh" | "scp");
    let target = if remote {
        Target::Remote
    } else {
        let mut paths: Vec<&String> = Vec::new();
        paths.extend(cd_targets.iter());
        paths.extend(git_c.iter());
        paths.extend(manifest.iter());
        paths.extend(positionals.iter().copied().filter(|a| looks_like_path(a)));
        if paths.is_empty() {
            match ctx.cwd.as_ref().and_then(|c| c.to_str()) {
                Some(cwd) => kind_of_path(cwd, ctx),
                None => Target::Unresolved,
            }
        } else {
            let kinds: Vec<Target> = paths.iter().map(|p| kind_of_path(p, ctx)).collect();
            if kinds.contains(&Target::Unresolved) {
                Target::Unresolved
            } else if kinds.contains(&Target::Worktree) {
                Target::Worktree
            } else if kinds.contains(&Target::MainCheckout) {
                Target::MainCheckout
            } else {
                Target::Other
            }
        }
    };
    EffectClass {
        tool: "Bash".into(),
        command: Some(cmd),
        subcommand: sub,
        target: Some(target),
        path_class: None,
    }
}

fn path_class_of(path: &str, ctx: &EffectContext) -> String {
    if path.contains('<') {
        return "<unresolved>".into();
    }
    let p = Path::new(path);
    let rel: PathBuf = match (&ctx.main_checkout, p.is_absolute()) {
        (Some(main), true) => match lexical_normalize(p).strip_prefix(main) {
            Ok(r) => r.to_path_buf(),
            Err(_) => return "/outside/**".into(),
        },
        (None, true) => return "<unresolved>".into(),
        (_, false) => lexical_normalize(p),
    };
    let mut comps = rel.components();
    match (comps.next(), comps.next()) {
        (Some(first), Some(_)) => format!("{}/**", first.as_os_str().to_string_lossy()),
        _ => "*".into(),
    }
}

/// The classifier: built-in rule semantics plus the active rule set.
#[derive(Debug, Clone, Default)]
pub struct Classifier {
    rules: Option<RuleSet>,
    ctx: EffectContext,
}

impl Classifier {
    /// A classifier over `rules`; `None` means the active settings could not be read, which
    /// makes every authorization stall `uncertain` (absence of a rule is not evidence of shape).
    pub fn new(rules: Option<RuleSet>, ctx: EffectContext) -> Self {
        Self { rules, ctx }
    }

    /// Classify one stalled call.
    pub fn classify(&self, stall: StallClass, tool: &str, input: CallInput<'_>) -> Classification {
        let effect = match (tool, input) {
            ("Bash", CallInput::Command(c) | CallInput::Shape(c)) => {
                effect_for_command(c, &self.ctx)
            }
            (t, CallInput::Path(p)) if is_edit_family(t) => {
                let mut e = EffectClass::bare(t);
                e.path_class = Some(path_class_of(p, &self.ctx));
                e
            }
            (t, _) if is_edit_family(t) => {
                let mut e = EffectClass::bare(t);
                e.path_class = Some("<unresolved>".into());
                e
            }
            ("Bash", _) => {
                let mut e = EffectClass::bare("Bash");
                e.target = Some(Target::Unresolved);
                e
            }
            (t, _) => EffectClass::bare(t),
        };
        let done = |concern, reason: &str, matched: Option<String>, rule_match: &str, patterns| {
            Classification {
                concern,
                reason: reason.to_owned(),
                matched_rule: matched,
                rule_match: rule_match.to_owned(),
                patterns,
                effect: effect.clone(),
            }
        };
        if stall != StallClass::Authorization {
            return done(
                ConcernClass::Uncertain,
                "non-authorization-stall",
                None,
                "none",
                vec![],
            );
        }
        let Some(rules) = self.rules.as_ref() else {
            return done(
                ConcernClass::Uncertain,
                "rules-unavailable",
                None,
                "none",
                vec![],
            );
        };
        let mut definite: Option<&Rule> = None;
        let mut possible: Option<&Rule> = None;
        let mut unparseable: Option<&Rule> = None;
        for rule in &rules.rules {
            match rules.relate(rule, tool, input) {
                Relation::Definite => {
                    // deny > ask: a deny match is the one reported.
                    if definite
                        .is_none_or(|d| d.kind == RuleKind::Ask && rule.kind == RuleKind::Deny)
                    {
                        definite = Some(rule);
                    }
                }
                Relation::Possible => possible = possible.or(Some(rule)),
                Relation::Unparseable => unparseable = unparseable.or(Some(rule)),
                Relation::Disjoint => {}
            }
        }
        if let Some(rule) = definite {
            let kind = if rule.kind == RuleKind::Deny {
                "deny"
            } else {
                "ask"
            };
            return done(
                ConcernClass::Risk,
                "explicit-rule",
                Some(rule.raw.clone()),
                kind,
                vec![],
            );
        }
        if let Some(rule) = unparseable {
            return done(
                ConcernClass::Uncertain,
                "unparseable-rule",
                Some(rule.raw.clone()),
                "unparseable",
                vec![],
            );
        }
        if let Some(rule) = possible {
            return done(
                ConcernClass::Uncertain,
                "rule-undecidable-from-redacted-input",
                Some(rule.raw.clone()),
                "possible",
                vec![],
            );
        }
        let patterns: Vec<String> = match (tool, input) {
            ("Bash", CallInput::Command(c) | CallInput::Shape(c)) => {
                shape_patterns(c).into_iter().map(str::to_owned).collect()
            }
            _ => vec![],
        };
        if patterns.is_empty() {
            done(
                ConcernClass::Uncertain,
                "no-declared-shape-pattern",
                None,
                "none",
                patterns,
            )
        } else {
            done(ConcernClass::Shape, "shape-pattern", None, "none", patterns)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn settings() -> Value {
        json!({"permissions": {
            "ask": ["Bash(git rebase*)", "Bash(git *push* +*)", "Edit(.claude/**)", "mcp__github__*"],
            "deny": ["Bash(git *reset --hard*)", "Bash(rm -rf:*)", "Read(.env)"],
            "allow": ["Bash(git status:*)"]
        }})
    }

    fn classifier() -> Classifier {
        Classifier::new(
            Some(RuleSet::from_settings(&[settings()])),
            EffectContext {
                main_checkout: Some("/p/elohim".into()),
                cwd: Some("/p/elohim/.worktrees/x".into()),
            },
        )
    }

    fn class(tool: &str, input: CallInput<'_>) -> Classification {
        classifier().classify(StallClass::Authorization, tool, input)
    }

    #[test]
    fn initial_shape_patterns_are_recognized() {
        for (cmd, pattern) in [
            ("cd app && pnpm exec vitest run foo.spec.ts", "cd-compound"),
            ("ls x ; cd y", "cd-compound"),
            ("cd a | cat", "cd-compound"),
            ("cat <<EOF > f\nbody\nEOF", "heredoc"),
            ("bash -c 'echo hi'", "bash-c"),
            ("echo one\necho two", "multi-line"),
        ] {
            let c = class("Bash", CallInput::Command(cmd));
            assert_eq!(c.concern, ConcernClass::Shape, "{cmd}");
            assert!(
                c.patterns.iter().any(|p| p == pattern),
                "{cmd}: {:?}",
                c.patterns
            );
        }
    }

    #[test]
    fn a_plain_command_with_no_rule_is_uncertain_not_shape() {
        // Absence of a matching rule alone never establishes shape.
        let c = class("Bash", CallInput::Command("cargo test"));
        assert_eq!(c.concern, ConcernClass::Uncertain);
        assert_eq!(c.reason, "no-declared-shape-pattern");
        // A here-string and `||` are not declared patterns.
        assert_eq!(
            class("Bash", CallInput::Command("cat <<< hi")).concern,
            ConcernClass::Uncertain
        );
        assert_eq!(
            class("Bash", CallInput::Command("cd a || true")).concern,
            ConcernClass::Uncertain
        );
    }

    #[test]
    fn explicit_ask_and_deny_rules_are_risk_and_deny_wins() {
        let c = class("Bash", CallInput::Command("git rebase --onto main feat"));
        assert_eq!(
            (c.concern, c.rule_match.as_str()),
            (ConcernClass::Risk, "ask")
        );
        let c = class("Bash", CallInput::Command("git -C /x reset --hard HEAD~1"));
        assert_eq!(
            (c.concern, c.rule_match.as_str()),
            (ConcernClass::Risk, "deny")
        );
        // A risky segment inside a shape-class compound is still risk.
        let c = class("Bash", CallInput::Command("cd app && git rebase main"));
        assert_eq!(c.concern, ConcernClass::Risk);
    }

    #[test]
    fn legacy_prefix_form_normalizes_to_star() {
        let c = class("Bash", CallInput::Command("rm -rf /tmp/x"));
        assert_eq!(
            (c.concern, c.matched_rule.as_deref()),
            (ConcernClass::Risk, Some("Bash(rm -rf:*)"))
        );
    }

    #[test]
    fn allow_entries_never_override_an_ask() {
        let both =
            json!({"permissions": {"allow": ["Bash(git rebase:*)"], "ask": ["Bash(git rebase*)"]}});
        let c = Classifier::new(
            Some(RuleSet::from_settings(&[both])),
            EffectContext::default(),
        );
        assert_eq!(
            c.classify(
                StallClass::Authorization,
                "Bash",
                CallInput::Command("git rebase main")
            )
            .concern,
            ConcernClass::Risk
        );
    }

    #[test]
    fn redacted_shapes_match_rules_conservatively() {
        // `+main` is redacted to <str>: the rule might match, so the call is uncertain.
        let c = class("Bash", CallInput::Shape("git push <str> <str>"));
        assert_eq!(
            (c.concern, c.rule_match.as_str()),
            (ConcernClass::Uncertain, "possible")
        );
        // Flags survive redaction, so a flag-bearing rule matches definitely.
        let c = class("Bash", CallInput::Shape("git rebase --onto <ref> <ref>"));
        assert_eq!(
            (c.concern, c.rule_match.as_str()),
            (ConcernClass::Risk, "ask")
        );
        // A redacted `-C <path>` could hide anything between `git` and `rebase`: undecidable.
        let c = class("Bash", CallInput::Shape("git -C <path> rebase main"));
        assert_eq!(
            (c.concern, c.rule_match.as_str()),
            (ConcernClass::Uncertain, "possible")
        );
        // A redacted shape with a declared pattern and no possible rule is shape.
        let c = class(
            "Bash",
            CallInput::Shape("cd <path> && pnpm exec vitest run <path>"),
        );
        assert_eq!(c.concern, ConcernClass::Shape);
    }

    #[test]
    fn bare_tool_path_mcp_and_unparseable_rules() {
        let s = json!({"permissions": {"ask": ["WebSearch", "WebFetch(domain:example.com)", "Edit(.claude/**)", "mcp__github__*", "mcp__slack"], "deny": ["Read(.env)"]}});
        let c = Classifier::new(Some(RuleSet::from_settings(&[s])), EffectContext::default());
        let k = |tool: &str, input| c.classify(StallClass::Authorization, tool, input);
        assert_eq!(
            k("WebSearch", CallInput::Opaque).concern,
            ConcernClass::Risk
        );
        // A specifier form outside the subset is unparseable for that tool, hence uncertain.
        assert_eq!(k("WebFetch", CallInput::Opaque).reason, "unparseable-rule");
        assert_eq!(
            k("Write", CallInput::Path(".claude/x.json")).concern,
            ConcernClass::Risk
        );
        assert_eq!(
            k("Edit", CallInput::Path("app/x.ts")).concern,
            ConcernClass::Uncertain
        );
        // No path was recorded: the path rule might match.
        assert_eq!(k("Edit", CallInput::Opaque).rule_match, "possible");
        assert_eq!(
            k("mcp__github__create_issue", CallInput::Opaque).concern,
            ConcernClass::Risk
        );
        assert_eq!(
            k("mcp__slack__post", CallInput::Opaque).concern,
            ConcernClass::Risk
        );
        assert_eq!(
            k("Read", CallInput::Path("a/b/.env")).concern,
            ConcernClass::Risk
        );
        assert_eq!(
            k("Read", CallInput::Path("a/b/env.txt")).concern,
            ConcernClass::Uncertain
        );
    }

    #[test]
    fn unclassifiable_inputs_are_uncertain() {
        let broken = json!({"permissions": {"ask": ["Bash(unterminated", 42]}});
        let c = Classifier::new(
            Some(RuleSet::from_settings(&[broken])),
            EffectContext::default(),
        );
        let r = c.classify(
            StallClass::Authorization,
            "Bash",
            CallInput::Command("cd a && ls"),
        );
        assert_eq!(
            (r.concern, r.reason.as_str()),
            (ConcernClass::Uncertain, "unparseable-rule")
        );
        // Settings that could not be read at all.
        let none = Classifier::new(None, EffectContext::default());
        let r = none.classify(
            StallClass::Authorization,
            "Bash",
            CallInput::Command("cd a && ls"),
        );
        assert_eq!(r.reason, "rules-unavailable");
        // Question and silence stalls are always uncertain, so no reshape is ever offered.
        let q = classifier().classify(
            StallClass::Question,
            "Bash",
            CallInput::Command("cd a && ls"),
        );
        assert_eq!(q.concern, ConcernClass::Uncertain);
        assert_eq!(
            classifier()
                .classify(StallClass::Silence, "unknown", CallInput::Opaque)
                .concern,
            ConcernClass::Uncertain
        );
    }

    #[test]
    fn effect_normalization_follows_the_ordered_target_rules() {
        let eff = |cmd: &str| class("Bash", CallInput::Command(cmd)).effect;
        let e = eff("git push origin main");
        assert_eq!(
            (e.command.as_deref(), e.subcommand.as_deref(), e.target),
            (Some("git"), Some("push"), Some(Target::Remote))
        );
        assert_eq!(eff("curl -s https://x.test/p").target, Some(Target::Remote));
        assert_eq!(
            eff("git -C /p/elohim/.worktrees/y rebase main").target,
            Some(Target::Worktree)
        );
        assert_eq!(
            eff("cargo test --manifest-path /p/elohim/elohim/Cargo.toml").target,
            Some(Target::MainCheckout)
        );
        assert_eq!(
            eff("cargo test --manifest-path /p/elohim/.worktrees/z/Cargo.toml").target,
            Some(Target::Worktree)
        );
        assert_eq!(eff("cd /tmp/scratch && ls").target, Some(Target::Other));
        // No path argument: the session's working directory decides (a worktree here).
        let e = eff("cargo test");
        assert_eq!(
            (e.subcommand.as_deref(), e.target),
            (Some("test"), Some(Target::Worktree))
        );
        let e = eff("cd app && pnpm exec vitest run foo.spec.ts");
        assert_eq!(
            (e.command.as_deref(), e.subcommand.as_deref()),
            (Some("pnpm"), Some("exec"))
        );
        // `git rebase` is a remote effect only with a remote name or URL.
        assert_eq!(eff("git fetch").target, Some(Target::Worktree));
    }

    #[test]
    fn redaction_leaves_path_targets_unresolved_and_never_equal() {
        let c = class("Bash", CallInput::Shape("git -C <path> rebase main"));
        assert_eq!(c.effect.target, Some(Target::Unresolved));
        assert!(!c.effect.same_effect(&c.effect.clone()));
        let a = class("Bash", CallInput::Command("cargo test"));
        assert!(a.effect.same_effect(&a.effect.clone()));
        let w = class("Edit", CallInput::Path("/p/elohim/app/x.ts"));
        assert_eq!(w.effect.path_class.as_deref(), Some("app/**"));
    }

    #[test]
    fn wildcard_matcher_is_exact_for_literals_and_loose_for_placeholders() {
        assert!(wild_match("git *push*", "git push origin", false));
        assert!(!wild_match("git push", "git push origin", false));
        assert!(wild_match("git push +*", "git push <str>", true));
        assert!(!wild_match("git push +*", "git push <str>", false));
        assert!(path_matches("*.env", "a/b/x.env"));
        assert!(path_matches("/src/**", "src/a/b.rs"));
        assert!(!path_matches("/src/*.rs", "lib/src/a.rs"));
    }
}
