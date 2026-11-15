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
//!    is lexed for string literals. A literal that resolves (against the source's directory, its
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
//! present key identically. Named residuals: a build script or proc macro in a dependency outside
//! the view, a compile-time reader that assembles the path without its leaf literal inside a
//! non-build-script source, a submodule nested inside a submodule (not listed), and an
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
                    .is_ok_and(|text| text.contains("ts-rs")),
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
        let Some(files) = view_files(repo, view, gitlinks)? else {
            return Ok((Vec::new(), all));
        };
        refuse_readers(repo, view, &files, &excluded)?;
    }
    Ok((excluded, refusing))
}

/// Every tracked file inside the view, checkout-relative: the superproject's index and, for each
/// submodule the view reaches (a gitlink above a view path or one listed inside it), that
/// submodule's own index. `None` when a checked-out submodule's
/// listing cannot be read. A submodule that is not checked out holds no file a build could read.
fn view_files(
    repo: &Path,
    view: &[String],
    gitlinks: &[String],
) -> ArtifactResult<Option<Vec<String>>> {
    let mut args = vec!["ls-files", "-z", "-s", "--full-name", "--"];
    args.extend(view.iter().map(String::as_str));
    let mut files = Vec::new();
    let mut links: BTreeSet<String> = gitlinks.iter().cloned().collect();
    for entry in git_ok(repo, &args)?.split('\0').filter(|p| !p.is_empty()) {
        // `<mode> <oid> <stage>\t<path>`
        let Some((meta, path)) = entry.split_once('\t') else {
            continue;
        };
        if meta.starts_with("160000 ") {
            links.insert(path.to_string());
        } else {
            files.push(path.to_string());
        }
    }
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
        let mut args = vec!["ls-files", "-z", "--"];
        args.extend(inner.iter().map(String::as_str));
        let Ok(listed) = git_ok(&dir, &args) else {
            return Ok(None);
        };
        files.extend(
            listed
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(|p| format!("{link}/{p}")),
        );
    }
    Ok(Some(files))
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
    let leaves: BTreeSet<&str> = excluded
        .iter()
        .map(|root| root.rsplit('/').next().unwrap_or(root))
        .collect();
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
        let Ok(text) = std::fs::read_to_string(repo.join(file)) else {
            // Absent (a deletion `git status` reports as dirty) or not UTF-8 (no literal of an
            // excluded path can be in it as text).
            continue;
        };
        if !leaves.iter().any(|leaf| text.contains(leaf)) {
            continue;
        }
        let file_dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        let crate_dir = crate_root(repo, file_dir);
        let reader = if manifest {
            Reader::Manifest
        } else if name == "build.rs" || is_proc_macro(repo, &crate_dir) {
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

/// The crate at `crate_dir` is a proc-macro crate (its code runs inside the compiler).
fn is_proc_macro(repo: &Path, crate_dir: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(repo.join(crate_dir).join("Cargo.toml")) else {
        return false;
    };
    text.parse::<toml::Table>().is_ok_and(|table| {
        table
            .get("lib")
            .and_then(toml::Value::as_table)
            .is_some_and(|lib| {
                ["proc-macro", "proc_macro"]
                    .iter()
                    .any(|key| lib.get(*key).and_then(toml::Value::as_bool) == Some(true))
            })
    })
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
    let path_shaped = literal.contains('/') && reader != Reader::Manifest && in_macro;
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

/// The string literals of a Rust source (`"…"`, `b"…"`, `r#"…"#`), comments skipped. Escapes are
/// kept as written except `\"` and `\\`; a path literal needs neither.
fn rust_string_literals(text: &str) -> Vec<Literal> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    // One frame per open delimiter: whether it is (inside) a macro invocation or attribute.
    let mut frames: Vec<bool> = Vec::new();
    // The last significant character outside literals and comments.
    let mut last = ' ';
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let in_macro = frames.last().copied().unwrap_or(false);
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
                || (chars[i - 1] == 'b' && (i < 2 || !ident(chars[i - 2]))))
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
                });
                i = end + 1 + hashes;
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
                });
                i += 1;
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
                frames.push(in_macro || last == '!' || (c == '[' && last == '#'));
                last = c;
                i += 1;
            }
            ')' | ']' | '}' => {
                frames.pop();
                last = c;
                i += 1;
            }
            _ => {
                if !c.is_whitespace() {
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
        // A build script's bare leaf name does: it can join the path from pieces.
        assert!(hit("dist", krate, krate, true).is_some());
        // A manifest's path dependency on the package's directory does not; a path into it does.
        let manifest = |literal: &str| {
            named_root(
                &Literal {
                    text: literal.to_string(),
                    in_macro: false,
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
