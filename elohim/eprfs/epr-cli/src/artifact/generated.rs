//! Ignored generated outputs a mesh-build key may leave out, and the mechanical check that no
//! build in the view reads them.
//!
//! The key refuses ignored files inside the method's view because a recorded build must never
//! vouch for bytes its key does not cover. A normal checkout nevertheless holds ignored outputs
//! that other tools generate beside sources cargo reads: a JavaScript package's `dist/` (written
//! by `tsc`/`vite` from the package's tracked sources) and a crate's ts-rs `bindings/` export
//! directory (written by `cargo test export_bindings`). Refusing them leaves every ordinary dev
//! checkout unable to record a build.
//!
//! Leaving such a directory out is sound only when no build in the view can read it, so the
//! exclusion is closed on two sides, and it holds only when both hold:
//!
//! 1. **A declared shape.** `<dir>/dist` beside a tracked `<dir>/package.json`, or
//!    `<crate>/bindings` beside a tracked `<crate>/Cargo.toml` that names `ts-rs`. Nothing else is
//!    left out; every other ignored path still refuses. A name alone (`dist`, `bindings`) never
//!    qualifies.
//! 2. **No tracked reader names it.** Every tracked Rust source and Cargo manifest inside the view
//!    is lexed for string literals (after blanking a leading byte-order mark and shebang line, as
//!    rustc drops them; raw byte and raw C strings escape nothing). A literal that resolves (against the source's directory, its
//!    crate root, the checkout root, or after `$CARGO_MANIFEST_DIR`/a `concat!` tail) into an
//!    excluded directory refuses, and so does a path-shaped macro or attribute argument onto one
//!    of its ancestors when resolved against the crate root, after `$CARGO_MANIFEST_DIR` or as
//!    an absolute path (where a directory-walking macro resolves it): that is how
//!    `include_bytes!`/`include_str!`/`include!`/`include_dir!`, rust-embed's `#[folder]`, a
//!    `#[path]` module and a manifest `path`/`include`/`build` reach it (a manifest only by a
//!    path into it: a path dependency on a package directory reads that package's tracked
//!    sources, which are lexed themselves, never its ignored files). Build scripts and
//!    proc-macro crates run code at build time and can assemble a path from pieces, so in those
//!    files any literal with the excluded directory's leaf name as a path component refuses.
//!
//! A submodule inside the view is lexed from its own index (its checked-out tracked files under
//! the view); when that listing cannot be read, nothing is left out.
//!
//! The excluded bytes never enter the key, so a clean checkout and one with the generated outputs
//! present key identically. A tracked symlink inside the view that targets, holds or sits inside
//! an excluded root refuses (`refuse_escaping_symlinks` refuses only targets leaving the view). A
//! file that does not read (other than absent or not UTF-8) refuses; a manifest that does not
//! read or parse makes its whole crate build-time code; `package.build` names the build script.
//!
//! Named residuals: a build script or proc macro in a dependency outside the view; a compile-time
//! reader that assembles the path without its leaf literal inside a non-build-script source
//! (including a module a build script pulls in from another file); a submodule nested inside a
//! submodule (not listed); non-Rust configuration a build tool reads is not lexed (a
//! `tauri.conf.json` `frontendDist: "../dist"` beside a tracked package.json would pass); and an
//! existence-only probe (`Path::exists`) of an empty directory.
//!
//! Why not the alternatives: the view format (`eprfs_core::View`) is an antichain of included
//! paths with no exclusions, so carving the outputs out of the descriptor would change the method
//! format and every method CID; and a bare name list (`dist`, `bindings`) would also exempt the
//! frontend bundle a crate embeds, which is exactly the case the refusal exists for.

use std::collections::BTreeSet;
use std::path::Path;

use super::{git_ok, lexical, under_view, ArtifactError, ArtifactResult};

/// A declared generated-output shape: its leaf directory and the tracked manifest beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A JavaScript package's build output.
    PackageDist,
    /// A crate's ts-rs default export directory.
    TsRsBindings,
}

impl Shape {
    const ALL: [Shape; 2] = [Shape::PackageDist, Shape::TsRsBindings];

    fn leaf(self) -> &'static str {
        match self {
            Shape::PackageDist => "dist",
            Shape::TsRsBindings => "bindings",
        }
    }

    fn manifest(self) -> &'static str {
        match self {
            Shape::PackageDist => "package.json",
            Shape::TsRsBindings => "Cargo.toml",
        }
    }
}

/// `prefix/name`, or `name` at the checkout root.
fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

/// `path` is `dir` or under it; the checkout root (`""`) holds everything.
fn within(path: &str, dir: &str) -> bool {
    dir.is_empty() || path == dir || path.starts_with(&format!("{dir}/"))
}

/// A declared-shape directory a path sits at or under, with the manifest that would make it one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShapeRoot {
    root: String,
    shape: Shape,
    manifest: String,
}

/// Every declared-shape root `path` sits at or under, nearest the checkout root first.
fn shape_roots(path: &str) -> Vec<ShapeRoot> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut roots = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        for shape in Shape::ALL {
            if *part == shape.leaf() {
                let dir = parts[..i].join("/");
                roots.push(ShapeRoot {
                    root: join(&dir, part),
                    shape,
                    manifest: join(&dir, shape.manifest()),
                });
            }
        }
    }
    roots
}

/// Split the ignored paths `git status --ignored` listed inside the view into the generated-output
/// roots left out of the key and the paths that still refuse.
///
/// # Errors
///
/// [`ArtifactError::Gap`] when git cannot answer, or a tracked source inside the view names a
/// would-be excluded directory (the build could read it).
pub(super) fn left_out(
    repo: &Path,
    view: &[String],
    gitlinks: &[String],
    ignored: Vec<String>,
) -> ArtifactResult<(Vec<String>, Vec<String>)> {
    if ignored.is_empty() {
        return Ok((Vec::new(), ignored));
    }
    let all = ignored.clone();
    let candidates: Vec<(String, Vec<ShapeRoot>)> = ignored
        .into_iter()
        .map(|path| {
            let roots = shape_roots(path.trim_end_matches('/'));
            (path, roots)
        })
        .collect();
    let manifests: BTreeSet<&str> = candidates
        .iter()
        .flat_map(|(_, roots)| roots.iter().map(|r| r.manifest.as_str()))
        .collect();
    let tracked = tracked_files(repo, manifests.iter().copied())?;
    let qualifies = |shape: Shape, manifest: &str| {
        tracked.contains(manifest)
            && match shape {
                Shape::PackageDist => true,
                Shape::TsRsBindings => std::fs::read_to_string(repo.join(manifest))
                    .is_ok_and(|text| depends_on_ts_rs(&text)),
            }
    };
    let mut excluded = BTreeSet::new();
    let mut refusing = Vec::new();
    for (path, roots) in candidates {
        match roots.into_iter().find(|r| qualifies(r.shape, &r.manifest)) {
            Some(r) => {
                excluded.insert(r.root);
            }
            None => refusing.push(path),
        }
    }
    let excluded: Vec<String> = excluded.into_iter().collect();
    if !excluded.is_empty() {
        let Some(listed) = view_files(repo, view, gitlinks)? else {
            return Ok((Vec::new(), all));
        };
        refuse_aliasing_symlinks(repo, &listed.symlinks, &excluded)?;
        refuse_readers(repo, view, &listed.files, &excluded)?;
    }
    Ok((excluded, refusing))
}

/// The tracked entries inside the view, checkout-relative.
struct ViewFiles {
    /// Regular tracked files.
    files: Vec<String>,
    /// Tracked symlinks (mode 120000).
    symlinks: Vec<String>,
}

/// Split one `git ls-files -s -z` listing into regular files, symlinks and gitlinks, each
/// prefixed with `prefix`.
fn split_staged(listed: &str, prefix: &str, out: &mut ViewFiles, links: &mut BTreeSet<String>) {
    for entry in listed.split('\0').filter(|p| !p.is_empty()) {
        // `<mode> <oid> <stage>\t<path>`
        let Some((meta, path)) = entry.split_once('\t') else {
            continue;
        };
        let path = join(prefix, path);
        if meta.starts_with("160000 ") {
            links.insert(path);
        } else if meta.starts_with("120000 ") {
            out.symlinks.push(path);
        } else {
            out.files.push(path);
        }
    }
}

/// Refuse a tracked symlink inside the view whose target is, holds or sits inside an excluded
/// root: reading through it reads the ignored bytes under a name the lexical check never sees.
/// (`refuse_escaping_symlinks` refuses only targets leaving the view; an excluded root is inside
/// it.) An unreadable link refuses too.
fn refuse_aliasing_symlinks(
    repo: &Path,
    symlinks: &[String],
    excluded: &[String],
) -> ArtifactResult<()> {
    for link in symlinks {
        let target = std::fs::read_link(repo.join(link)).map_err(|e| {
            ArtifactError::Gap(format!(
                "the tracked symlink {link} inside the method's view does not read ({e}), so it \
                 cannot be shown not to alias an ignored generated output"
            ))
        })?;
        let target = target.to_string_lossy();
        let dir = link.rsplit_once('/').map_or("", |(dir, _)| dir);
        let resolved = if target.starts_with('/') {
            None
        } else {
            lexical(&format!("{dir}/{target}"))
        };
        let aliased = match &resolved {
            Some(resolved) => excluded
                .iter()
                .find(|root| within(resolved, root) || within(root, resolved)),
            // An absolute target: left to `refuse_escaping_symlinks`, which refuses it.
            None => None,
        };
        if let Some(root) = aliased {
            return Err(ArtifactError::Gap(format!(
                "the tracked symlink {link} inside the method's view points at {target}, which \
                 reaches the ignored generated output {root}; a build could read it and no \
                 closure line names it"
            )));
        }
    }
    Ok(())
}

/// `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` or any
/// `[target.*.…dependencies]` table of a Cargo manifest names `ts-rs` (as its key, or renamed
/// through `package = "ts-rs"`); a comment or a string elsewhere does not. An unparsable manifest
/// does not qualify.
fn depends_on_ts_rs(text: &str) -> bool {
    let Ok(table) = text.parse::<toml::Table>() else {
        return false;
    };
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let names = |deps: Option<&toml::Value>| {
        deps.and_then(toml::Value::as_table).is_some_and(|deps| {
            deps.iter().any(|(key, value)| {
                key == "ts-rs"
                    || value
                        .as_table()
                        .and_then(|v| v.get("package"))
                        .and_then(toml::Value::as_str)
                        == Some("ts-rs")
            })
        })
    };
    KINDS.iter().any(|kind| names(table.get(*kind)))
        || table
            .get("target")
            .and_then(toml::Value::as_table)
            .is_some_and(|targets| {
                targets.values().any(|target| {
                    target
                        .as_table()
                        .is_some_and(|t| KINDS.iter().any(|kind| names(t.get(*kind))))
                })
            })
}

/// Every tracked file inside the view, checkout-relative: the superproject's index and, for each
/// submodule the view reaches (a gitlink above a view path or one listed inside it), that
/// submodule's own index. `None` when a checked-out submodule's
/// listing cannot be read. A submodule that is not checked out holds no file a build could read.
fn view_files(
    repo: &Path,
    view: &[String],
    gitlinks: &[String],
) -> ArtifactResult<Option<ViewFiles>> {
    let mut args = vec!["ls-files", "-z", "-s", "--full-name", "--"];
    args.extend(view.iter().map(String::as_str));
    let mut out = ViewFiles {
        files: Vec::new(),
        symlinks: Vec::new(),
    };
    let mut links: BTreeSet<String> = gitlinks.iter().cloned().collect();
    split_staged(&git_ok(repo, &args)?, "", &mut out, &mut links);
    let mut nested = BTreeSet::new();
    for link in &links {
        let dir = repo.join(link);
        if !dir.join(".git").exists() {
            continue;
        }
        let inner: Vec<String> = view
            .iter()
            .filter_map(|v| {
                if within(link, v) {
                    Some(".".to_string())
                } else {
                    v.strip_prefix(&format!("{link}/")).map(str::to_string)
                }
            })
            .collect();
        let mut args = vec!["ls-files", "-z", "-s", "--"];
        args.extend(inner.iter().map(String::as_str));
        let Ok(listed) = git_ok(&dir, &args) else {
            return Ok(None);
        };
        split_staged(&listed, link, &mut out, &mut nested);
    }
    Ok(Some(out))
}

/// The tracked files among `paths` (`git ls-files`), exactly as named.
fn tracked_files<'a>(
    repo: &Path,
    paths: impl IntoIterator<Item = &'a str>,
) -> ArtifactResult<BTreeSet<String>> {
    let paths: Vec<&str> = paths.into_iter().collect();
    if paths.is_empty() {
        return Ok(BTreeSet::new());
    }
    let mut args = vec!["ls-files", "-z", "--full-name", "--"];
    args.extend(paths.iter().copied());
    let listed = git_ok(repo, &args)?;
    Ok(listed
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

/// Refuse when a tracked Rust source or Cargo manifest inside the view names an excluded root.
fn refuse_readers(
    repo: &Path,
    view: &[String],
    files: &[String],
    excluded: &[String],
) -> ArtifactResult<()> {
    let mut crates: std::collections::BTreeMap<String, CrateInfo> = Default::default();
    let repo_abs = std::fs::canonicalize(repo)
        .ok()
        .and_then(|p| p.to_str().map(str::to_string));
    for file in files.iter().map(String::as_str) {
        if !under_view(file, view) {
            continue;
        }
        let name = file.rsplit('/').next().unwrap_or(file);
        let manifest = name == "Cargo.toml";
        if !(manifest || name.ends_with(".rs")) {
            continue;
        }
        // Every file is lexed: a directory-walking macro can name an ancestor of an excluded
        // root without its leaf name appearing anywhere in the file.
        let text = match std::fs::read_to_string(repo.join(file)) {
            Ok(text) => text,
            // Absent: a deletion `git status` reports as dirty. Not UTF-8: no literal of an
            // excluded path can be in it as text.
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidData
                ) =>
            {
                continue
            }
            Err(e) => {
                return Err(ArtifactError::Gap(format!(
                    "the tracked {file} inside the method's view does not read ({e}), so it \
                     cannot be shown not to read an ignored generated output"
                )))
            }
        };
        let file_dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        let crate_dir = crate_root(repo, file_dir);
        let info = crates
            .entry(crate_dir.clone())
            .or_insert_with(|| crate_info(repo, &crate_dir));
        let reader = if manifest {
            Reader::Manifest
        } else if name == "build.rs"
            || info.runs_at_build
            || info.build_script.as_deref() == Some(file)
        {
            Reader::BuildCode
        } else {
            Reader::Source
        };
        let literals = if manifest {
            toml_strings(&text)
        } else {
            rust_string_literals(&text)
        };
        for literal in literals {
            if let Some(root) = named_root(
                &literal,
                file_dir,
                &crate_dir,
                repo_abs.as_deref(),
                excluded,
                reader,
            ) {
                return Err(ArtifactError::Gap(format!(
                    "the ignored generated output {root} inside the method's view is named by the \
                     tracked {file} (\"{}\"); a build could read it and no closure line \
                     names it",
                    literal.text
                )));
            }
        }
    }
    Ok(())
}

/// The nearest directory at or above `dir` holding a `Cargo.toml`, or the checkout root.
fn crate_root(repo: &Path, dir: &str) -> String {
    let mut current = dir.to_string();
    loop {
        if repo.join(&current).join("Cargo.toml").is_file() {
            return current;
        }
        match current.rsplit_once('/') {
            Some((parent, _)) => current = parent.to_string(),
            None if !current.is_empty() => current = String::new(),
            None => return current,
        }
    }
}

/// What a crate's manifest says about code that runs at build time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrateInfo {
    /// Every source of the crate runs at build time: a proc-macro crate, or a manifest that does
    /// not read or parse (fail closed).
    runs_at_build: bool,
    /// The build script's checkout-relative path: `package.build` when it is a path, else
    /// `build.rs` at the crate root; `None` when `build = false`.
    build_script: Option<String>,
}

fn crate_info(repo: &Path, crate_dir: &str) -> CrateInfo {
    let default_script = Some(join(crate_dir, "build.rs"));
    let text = match std::fs::read_to_string(repo.join(crate_dir).join("Cargo.toml")) {
        Ok(text) => text,
        // No manifest at all: the checkout root outside any crate.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return CrateInfo {
                runs_at_build: false,
                build_script: default_script,
            }
        }
        Err(_) => {
            return CrateInfo {
                runs_at_build: true,
                build_script: default_script,
            }
        }
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return CrateInfo {
            runs_at_build: true,
            build_script: default_script,
        };
    };
    let proc_macro = table
        .get("lib")
        .and_then(toml::Value::as_table)
        .is_some_and(|lib| {
            ["proc-macro", "proc_macro"]
                .iter()
                .any(|key| lib.get(*key).and_then(toml::Value::as_bool) == Some(true))
        });
    let build_script = match table
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("build"))
    {
        Some(toml::Value::String(path)) => lexical(&format!("{crate_dir}/{path}")),
        Some(toml::Value::Boolean(false)) => None,
        _ => default_script,
    };
    CrateInfo {
        runs_at_build: proc_macro,
        build_script,
    }
}

/// How a tracked file can read paths when cargo builds the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reader {
    /// Ordinary Rust source: compile-time reads take a literal path (`include_*!`, `#[path]`,
    /// an attribute macro's folder), which may name a directory a macro walks.
    Source,
    /// A build script or proc-macro crate: it runs code and can join a path from pieces.
    BuildCode,
    /// A Cargo manifest: `path`, `build`, `include`, `workspace` name files or packages. A path
    /// dependency naming a package's directory does not read that package's ignored files (cargo
    /// lists a package's files through git, which leaves ignored paths out; the package's own
    /// sources are lexed in turn), so only a literal resolving into an excluded root refuses.
    Manifest,
}

/// One string literal of a tracked file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Literal {
    text: String,
    /// Inside a macro invocation or an attribute (`name!(…)`, `#[…]`), where a compile-time
    /// reader takes its path; a runtime `Path::join("..")` is not.
    in_macro: bool,
    /// The first argument of a macro invocation (`include_dir!("web")`) or an attribute value
    /// (`#[folder = "."]`): where a directory-walking macro takes the directory it embeds, so a
    /// bare name (no `/`) resolved against the crate root can walk onto an excluded root's
    /// ancestor.
    walk_arg: bool,
}

/// The excluded root `literal` reaches, if any (see the module docs for the resolution rules).
fn named_root(
    literal: &Literal,
    file_dir: &str,
    crate_dir: &str,
    repo_abs: Option<&str>,
    excluded: &[String],
    reader: Reader,
) -> Option<String> {
    let in_macro = literal.in_macro || reader == Reader::BuildCode;
    let walk_arg = literal.walk_arg;
    let literal = literal.text.as_str();
    if literal.is_empty() || literal.len() > 4096 || literal.contains('\n') {
        return None;
    }
    if reader == Reader::BuildCode {
        let components: BTreeSet<&str> = literal.split(['/', '\\']).collect();
        if let Some(root) = excluded
            .iter()
            .find(|root| components.contains(root.rsplit('/').next().unwrap_or(root.as_str())))
        {
            return Some(root.clone());
        }
    }
    // `(candidate, walk)`: `walk` marks a resolution a directory-walking reader can take (the
    // crate root, which `include_dir!` and rust-embed's `#[folder]` resolve against, and an
    // absolute path), where a path-shaped literal onto an ancestor of an excluded root reads it
    // too. Every other resolution refuses only a literal into one.
    let path_shaped = (literal.contains('/') || walk_arg) && reader != Reader::Manifest && in_macro;
    let mut resolved = Vec::new();
    if let Some(rest) = literal.strip_prefix("$CARGO_MANIFEST_DIR") {
        resolved.push((
            lexical(&format!("{crate_dir}/{rest}")),
            reader != Reader::Manifest && in_macro,
        ));
    } else if let Some(rest) = repo_abs.and_then(|abs| {
        literal
            .strip_prefix(abs)
            .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    }) {
        resolved.push((lexical(rest), reader != Reader::Manifest && in_macro));
    } else if literal.starts_with('/') {
        // A `concat!(env!("CARGO_MANIFEST_DIR"), "/…")` tail.
        resolved.push((lexical(&format!("{crate_dir}{literal}")), path_shaped));
    } else {
        resolved.push((lexical(&format!("{file_dir}/{literal}")), false));
        resolved.push((lexical(&format!("{crate_dir}/{literal}")), path_shaped));
        resolved.push((lexical(literal), false));
    }
    resolved
        .into_iter()
        .filter_map(|(candidate, walk)| candidate.map(|c| (c, walk)))
        .find_map(|(candidate, walk)| {
            excluded
                .iter()
                .find(|root| {
                    within(&candidate, root)
                        || (walk && candidate != **root && within(root, &candidate))
                })
                .cloned()
        })
}

/// Every string value in a TOML document (keys excluded); the Rust lexer's answer when it does
/// not parse, so an unparsable manifest is still read.
fn toml_strings(text: &str) -> Vec<Literal> {
    fn walk(value: &toml::Value, out: &mut Vec<Literal>) {
        match value {
            toml::Value::String(s) => out.push(Literal {
                text: s.clone(),
                in_macro: false,
                walk_arg: false,
            }),
            toml::Value::Array(items) => items.iter().for_each(|item| walk(item, out)),
            toml::Value::Table(table) => table.values().for_each(|item| walk(item, out)),
            _ => {}
        }
    }
    match text.parse::<toml::Table>() {
        Ok(table) => {
            let mut out = Vec::new();
            table.values().for_each(|value| walk(value, &mut out));
            out
        }
        Err(_) => rust_string_literals(text),
    }
}

/// One open delimiter of a Rust source.
#[derive(Debug, Clone, Copy, Default)]
struct Frame {
    /// At or inside a macro invocation or attribute.
    in_macro: bool,
    /// At or inside an attribute (`#[…]`, `#![…]`).
    in_attr: bool,
    /// This delimiter opened a macro invocation (`name!(`).
    opened_by_macro: bool,
}

/// Blank what rustc drops before lexing, keeping every index: a leading byte-order mark, and a
/// first-line shebang. As rustc does, `#!` opens a shebang unless the next token after
/// whitespace and comments is `[` (an inner attribute); an unclosed comment there leaves no `[`,
/// so the line is a shebang. Without this a shebang line such as `#!x /*` reads as a comment
/// opening and hides every literal after it.
fn blank_preamble(chars: &mut [char]) {
    let start = usize::from(chars.first() == Some(&'\u{feff}'));
    if start == 1 {
        chars[0] = ' ';
    }
    if chars.get(start) != Some(&'#') || chars.get(start + 1) != Some(&'!') {
        return;
    }
    let mut j = start + 2;
    loop {
        match (chars.get(j), chars.get(j + 1)) {
            (Some(c), _) if c.is_whitespace() => j += 1,
            (Some('/'), Some('/')) => {
                while j < chars.len() && chars[j] != '\n' {
                    j += 1;
                }
            }
            (Some('/'), Some('*')) => {
                let mut depth = 0usize;
                loop {
                    match (chars.get(j), chars.get(j + 1)) {
                        // Unclosed: no `[` follows, so rustc reads a shebang.
                        (None, _) => break,
                        (Some('/'), Some('*')) => {
                            depth += 1;
                            j += 2;
                        }
                        (Some('*'), Some('/')) => {
                            depth -= 1;
                            j += 2;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => j += 1,
                    }
                }
            }
            (Some('['), _) => return,
            _ => break,
        }
    }
    for c in chars[start..].iter_mut().take_while(|c| **c != '\n') {
        *c = ' ';
    }
}

/// The string literals of a Rust source (`"…"`, `b"…"`, `r#"…"#`), comments skipped. Escapes are
/// kept as written except `\"` and `\\`; a path literal needs neither.
fn rust_string_literals(text: &str) -> Vec<Literal> {
    let mut chars: Vec<char> = text.chars().collect();
    blank_preamble(&mut chars);
    let mut out = Vec::new();
    let mut i = 0;
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    // One frame per open delimiter.
    let mut frames: Vec<Frame> = Vec::new();
    // The last two significant characters outside literals and comments.
    let mut last = ' ';
    let mut before_last = ' ';
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let frame = frames.last().copied().unwrap_or_default();
        let in_macro = frame.in_macro;
        // The first token inside a delimiter a macro opened, or the value after `=` in an
        // attribute.
        let walk_arg = (frame.opened_by_macro && matches!(last, '(' | '[' | '{'))
            || (frame.in_attr && last == '=');
        match c {
            '/' if next == Some('/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if next == Some('*') => {
                let mut depth = 0usize;
                while i < chars.len() {
                    if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                        depth += 1;
                        i += 2;
                    } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            'r' if (i == 0
                || !ident(chars[i - 1])
                || (matches!(chars[i - 1], 'b' | 'c') && (i < 2 || !ident(chars[i - 2]))))
                && matches!(next, Some('"' | '#')) =>
            {
                let mut j = i + 1;
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if chars.get(j) != Some(&'"') {
                    // A raw identifier (`r#type`).
                    i = j;
                    continue;
                }
                let start = j + 1;
                let mut k = start;
                let end = loop {
                    if k >= chars.len() {
                        break chars.len();
                    }
                    if chars[k] == '"' && (1..=hashes).all(|h| chars.get(k + h) == Some(&'#')) {
                        break k;
                    }
                    k += 1;
                };
                out.push(Literal {
                    text: chars[start..end].iter().collect(),
                    in_macro,
                    walk_arg,
                });
                i = end + 1 + hashes;
                before_last = last;
                last = '"';
            }
            '"' => {
                let mut literal = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        match chars.get(i + 1) {
                            Some(&e @ ('"' | '\\')) => literal.push(e),
                            Some(&e) => {
                                literal.push('\\');
                                literal.push(e);
                            }
                            None => {}
                        }
                        i += 2;
                    } else {
                        literal.push(chars[i]);
                        i += 1;
                    }
                }
                out.push(Literal {
                    text: literal,
                    in_macro,
                    walk_arg,
                });
                i += 1;
                before_last = last;
                last = '"';
            }
            '\'' => {
                // A char literal (`'"'`, `'\''`) or a lifetime (`'a`).
                if next == Some('\\') {
                    // Past the opening quote, the backslash and the escaped character.
                    i += 3;
                    while i < chars.len() && chars[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if chars.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            '(' | '[' | '{' => {
                let attr = c == '[' && (last == '#' || (last == '!' && before_last == '#'));
                let by_macro = last == '!' && !attr;
                frames.push(Frame {
                    in_macro: in_macro || by_macro || attr,
                    in_attr: frame.in_attr || attr,
                    opened_by_macro: by_macro,
                });
                before_last = last;
                last = c;
                i += 1;
            }
            ')' | ']' | '}' => {
                frames.pop();
                before_last = last;
                last = c;
                i += 1;
            }
            _ => {
                if !c.is_whitespace() {
                    before_last = last;
                    last = c;
                }
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_c_strings_shebangs_and_byte_order_marks_lex_as_rustc_lexes_them() {
        // A raw C string's backslash escapes nothing: the literal after it is still seen.
        let src = r##"const A: &CStr = cr"\"; fn f() { let _ = include_bytes!("../dist/x"); }"##;
        assert!(rust_string_literals(src)
            .iter()
            .any(|l| l.text == "../dist/x"));
        assert!(
            rust_string_literals(r##"const A: &CStr = cr#"\"#; const B: &str = "b";"##)
                .iter()
                .any(|l| l.text == "b")
        );
        // rustc drops a first-line shebang: it opens no comment, even an unclosed one.
        for src in [
            "#!x /*\nfn f() { let _ = include_bytes!(\"../dist/y\"); }\n// */\n",
            "#! /*\nconst A: &[u8] = include_bytes!(\"../dist/y\");\n",
        ] {
            assert!(
                rust_string_literals(src)
                    .iter()
                    .any(|l| l.text == "../dist/y"),
                "{src}"
            );
        }
        // `#!` then `[`, after whitespace and comments, is an inner attribute and is read.
        for src in [
            "#![doc = \"attr\"]\n",
            "#! /* c */ [doc = \"attr\"]\n",
            "#!\n// c\n[doc = \"attr\"]\n",
        ] {
            assert!(
                rust_string_literals(src).iter().any(|l| l.text == "attr"),
                "{src}"
            );
        }
        // A byte-order mark before a shebang is dropped the same way.
        let src = "\u{feff}#!shebang \"x\"\nconst A: &str = \"out\";";
        let texts: Vec<_> = rust_string_literals(src)
            .into_iter()
            .map(|l| l.text)
            .collect();
        assert_eq!(texts, vec!["out".to_string()]);
    }

    #[test]
    fn literals_skip_comments_chars_and_read_raw_strings() {
        let text = r####"
            // include_bytes!("../dist/commented.js")
            /* "block" /* "nested" */ "still" */
            let q = '"'; let e = '\''; fn f<'a>(x: &'a str) {}
            let a = include_bytes!("../dist/a.js");
            let b = r#"raw "quoted" ../dist"#;
            let c = br"bytes";
            let d = "esc \"x\" \\ \n";
            let r#type = 1;
        "####;
        let literals = rust_string_literals(text);
        assert_eq!(
            literals.iter().map(|l| l.text.clone()).collect::<Vec<_>>(),
            vec![
                "../dist/a.js".to_string(),
                r#"raw "quoted" ../dist"#.to_string(),
                "bytes".to_string(),
                r#"esc "x" \ \n"#.to_string(),
            ]
        );
        // Only the include_bytes! argument sits inside a macro invocation.
        assert_eq!(
            literals.iter().map(|l| l.in_macro).collect::<Vec<_>>(),
            vec![true, false, false, false]
        );
        let args = rust_string_literals(
            "#![doc = \"d\"] #[folder = \".\"] struct A; include_dir!(\"web\"); \
             assert_eq!(x, \"..\"); #[cfg(feature = \"f\")] fn g() {}",
        );
        assert_eq!(
            args.iter()
                .map(|l| (l.text.as_str(), l.walk_arg))
                .collect::<Vec<_>>(),
            vec![
                ("d", true),
                (".", true),
                ("web", true),
                ("..", false),
                ("f", true)
            ]
        );
        let nested = rust_string_literals(
            "#[folder = \"../app\"] struct A; let p = root.join(\"../..\"); \
             x!(f(\"in\")); if !(a) { g(\"neg\") }",
        );
        assert_eq!(
            nested
                .iter()
                .map(|l| (l.text.as_str(), l.in_macro))
                .collect::<Vec<_>>(),
            vec![
                ("../app", true),
                ("../..", false),
                ("in", true),
                ("neg", false)
            ]
        );
    }

    #[test]
    fn shape_roots_name_the_manifest_beside_each_leaf() {
        assert_eq!(
            shape_roots("elohim/sdk/epr-ts/dist"),
            vec![ShapeRoot {
                root: "elohim/sdk/epr-ts/dist".to_string(),
                shape: Shape::PackageDist,
                manifest: "elohim/sdk/epr-ts/package.json".to_string(),
            }]
        );
        assert_eq!(
            shape_roots("elohim/epr/bindings"),
            vec![ShapeRoot {
                root: "elohim/epr/bindings".to_string(),
                shape: Shape::TsRsBindings,
                manifest: "elohim/epr/Cargo.toml".to_string(),
            }]
        );
        assert!(shape_roots("crate/src/schema.generated").is_empty());
    }

    #[test]
    fn literals_resolve_into_or_onto_an_excluded_root() {
        let excluded = vec!["elohim/sdk/epr-ts/dist".to_string()];
        let hit_in = |literal: &str, file_dir: &str, crate_dir: &str, build: bool, in_macro| {
            let reader = if build {
                Reader::BuildCode
            } else {
                Reader::Source
            };
            let literal = Literal {
                text: literal.to_string(),
                in_macro,
                walk_arg: false,
            };
            named_root(
                &literal,
                file_dir,
                crate_dir,
                Some("/repo"),
                &excluded,
                reader,
            )
        };
        let hit = |literal: &str, file_dir: &str, crate_dir: &str, build: bool| {
            hit_in(literal, file_dir, crate_dir, build, true)
        };
        let src = "elohim/sdk/storage-rs/src";
        let krate = "elohim/sdk/storage-rs";
        // include_bytes! is relative to the source file.
        assert!(hit("../../epr-ts/dist/index.js", src, krate, false).is_some());
        // A concat! tail after CARGO_MANIFEST_DIR, include_dir!'s spelling and an absolute path.
        assert!(hit("/../epr-ts/dist", src, krate, false).is_some());
        assert!(hit("$CARGO_MANIFEST_DIR/../epr-ts/dist", src, krate, false).is_some());
        assert!(hit("/repo/elohim/sdk/epr-ts/dist/a.js", src, krate, false).is_some());
        // Path-shaped onto an ancestor: the walk would read the whole package.
        assert!(hit("../epr-ts/", krate, krate, false).is_some());
        // Outside a macro or attribute the same literal is a runtime path, never a build read.
        assert!(hit_in("../epr-ts/", krate, krate, false, false).is_none());
        assert!(hit_in("../../..", krate, krate, false, false).is_none());
        // A checkout-relative directory prefix is no walk any reader takes from there.
        assert!(hit("elohim/", src, krate, false).is_none());
        assert!(hit("elohim/sdk/epr-ts/dist/a.js", src, krate, false).is_some());
        // Unrelated literals and a bare leaf name in ordinary code do not.
        assert!(hit("dist", src, krate, false).is_none());
        assert!(hit("../../epr-ts/src/generated/", src, krate, false).is_none());
        assert!(hit(".", src, krate, false).is_none());
        // As a walking macro's argument or an attribute value a bare name resolves against the
        // crate root: `#[folder = "."]` embeds the crate, its bindings/ included.
        let bindings = vec!["elohim/epr/bindings".to_string()];
        let walk = |literal: &str, walk_arg: bool| {
            named_root(
                &Literal {
                    text: literal.to_string(),
                    in_macro: true,
                    walk_arg,
                },
                "elohim/epr/src",
                "elohim/epr",
                Some("/repo"),
                &bindings,
                Reader::Source,
            )
        };
        assert!(walk(".", true).is_some());
        assert!(walk(".", false).is_none());
        assert!(walk("src", true).is_none());
        // A build script's bare leaf name does: it can join the path from pieces.
        assert!(hit("dist", krate, krate, true).is_some());
        // A manifest's path dependency on the package's directory does not; a path into it does.
        let manifest = |literal: &str| {
            named_root(
                &Literal {
                    text: literal.to_string(),
                    in_macro: false,
                    walk_arg: false,
                },
                krate,
                krate,
                Some("/repo"),
                &excluded,
                Reader::Manifest,
            )
        };
        assert!(manifest("../epr-ts").is_none());
        assert!(manifest("../epr-ts/dist/crate").is_some());
    }
}
