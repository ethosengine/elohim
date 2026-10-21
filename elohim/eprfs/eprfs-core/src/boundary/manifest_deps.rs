//! The one Cargo-manifest dependency reader the eprfs boundary rails share: R-E1 in eprfs-core's
//! `lib.rs` and R-E3 in `eprfs-storage/tests/boundary.rs` (which includes this file by path). It
//! is test-only source, outside the crate's module tree in every non-test build.
//!
//! A rail that reads a manifest must not be slipped past by the *form* of a dependency, so every
//! form Cargo accepts is read: a `[dependencies]`, `[dev-dependencies]` or `[build-dependencies]`
//! table, its `target.<cfg>.` form, a dotted table `[dependencies.<name>]` (target form too),
//! a dotted key `name.workspace = true`, a quoted key, and a rename (`package = "…"`) in an
//! inline table, a dotted key or a dotted table. A multi-line array value is not read as keys.
//! `[workspace.dependencies]`, `[patch.*]` and `[replace]` are not a crate's dependencies.

/// Which dependency table a declaration is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

/// One declared dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub kind: DepKind,
    /// The `cfg(…)` or triple of a `[target.<it>.…]` table, as written.
    pub target: Option<String>,
    /// The name the manifest declares it under.
    pub name: String,
    /// The package it renames, when it renames one.
    pub package: Option<String>,
}

impl Dep {
    /// The package Cargo resolves: the renamed package, else the declared name.
    pub fn package_name(&self) -> &str {
        self.package.as_deref().unwrap_or(&self.name)
    }
}

/// Split at every `sep` outside quotes, stopping at an unquoted `stop` (if any). Quotes are kept;
/// a backslash escapes the next character inside a basic (double-quoted) string.
fn split_unquoted(text: &str, sep: char, stop: Option<char>) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        let part = parts.last_mut().expect("never empty");
        match quote {
            Some(q) => {
                part.push(c);
                if escaped {
                    escaped = false;
                } else if q == '"' && c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                part.push(c);
            }
            None if Some(c) == stop => break,
            None if c == sep => parts.push(String::new()),
            None => part.push(c),
        }
    }
    parts
}

fn unquote(segment: &str) -> String {
    let s = segment.trim();
    let quoted = s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')));
    if quoted {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// The text before an unquoted `#`.
fn before_comment(text: &str) -> String {
    split_unquoted(text, '\n', Some('#')).swap_remove(0)
}

/// A `key = value` line's dotted key, each segment unquoted, and its value without a trailing
/// comment.
fn key_value(text: &str) -> Option<(Vec<String>, String)> {
    let parts = split_unquoted(text, '=', None);
    if parts.len() < 2 {
        return None;
    }
    let key = split_unquoted(&parts[0], '.', None)
        .iter()
        .map(|k| unquote(k))
        .collect();
    Some((
        key,
        before_comment(&parts[1..].join("=")).trim().to_string(),
    ))
}

/// The value of `package = "…"` on a line: a dotted key ending in `package`, or a `package` field
/// of an inline-table value.
fn package_in(line: &str) -> Option<String> {
    let (key, value) = key_value(line)?;
    if key.last().map(String::as_str) == Some("package") {
        return Some(unquote(&value));
    }
    let inner = value.strip_prefix('{')?;
    split_unquoted(inner, ',', Some('}'))
        .iter()
        .filter_map(|field| key_value(field))
        .find(|(k, _)| k.len() == 1 && k[0] == "package")
        .map(|(_, v)| unquote(&v))
}

/// Net bracket depth a line opens, ignoring brackets in strings and anything after `#`.
fn depth_change(text: &str) -> i32 {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some('"') if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '#' => break,
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                _ => {}
            },
        }
    }
    depth
}

fn kind_of(segment: &str) -> Option<DepKind> {
    match segment {
        "dependencies" => Some(DepKind::Normal),
        "dev-dependencies" | "dev_dependencies" => Some(DepKind::Dev),
        "build-dependencies" | "build_dependencies" => Some(DepKind::Build),
        _ => None,
    }
}

/// What a table header opens.
enum Table {
    /// `[kind]` or `[target.X.kind]`: each key is a dependency.
    Deps(DepKind, Option<String>),
    /// `[kind.name]` or `[target.X.kind.name]`: one dependency (its index), its fields the body.
    One(usize),
    Other,
}

/// Every dependency a Cargo manifest declares, in every table form.
pub fn dependencies(manifest: &str) -> Vec<Dep> {
    let mut deps: Vec<Dep> = Vec::new();
    let mut table = Table::Other;
    let mut depth = 0;
    for raw in manifest.lines() {
        let line = raw.trim();
        if depth > 0 {
            depth += depth_change(line);
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            let array = line.starts_with("[[");
            let segments: Vec<String> =
                split_unquoted(line.trim_start_matches('['), '.', Some(']'))
                    .iter()
                    .map(|s| unquote(s))
                    .collect();
            let (target, rest) = match segments.first().map(String::as_str) {
                Some("target") if segments.len() >= 3 => {
                    (Some(segments[1].clone()), &segments[2..])
                }
                _ => (None, &segments[..]),
            };
            table = match (array, rest) {
                (false, [kind]) => match kind_of(kind) {
                    Some(kind) => Table::Deps(kind, target),
                    None => Table::Other,
                },
                (false, [kind, name]) => match kind_of(kind) {
                    Some(kind) => {
                        deps.push(Dep {
                            kind,
                            target,
                            name: name.clone(),
                            package: None,
                        });
                        Table::One(deps.len() - 1)
                    }
                    None => Table::Other,
                },
                _ => Table::Other,
            };
            continue;
        }
        depth = depth_change(line).max(0);
        match &table {
            Table::Other => {}
            Table::Deps(kind, target) => {
                let Some(name) = key_value(line).and_then(|(key, _)| key.into_iter().next()) else {
                    continue;
                };
                if name.is_empty() {
                    continue;
                }
                let package = package_in(line);
                match deps
                    .iter_mut()
                    .find(|d| d.kind == *kind && d.target == *target && d.name == name)
                {
                    // `name.version = …` then `name.package = …`: one dependency.
                    Some(existing) => {
                        if package.is_some() {
                            existing.package = package;
                        }
                    }
                    None => deps.push(Dep {
                        kind: *kind,
                        target: target.clone(),
                        name,
                        package,
                    }),
                }
            }
            Table::One(at) => {
                if let Some(package) = package_in(line) {
                    deps[*at].package = Some(package);
                }
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY_FORM: &str = r#"
[package]
name = "x"
dependencies = "not a table"

[dependencies]
plain = "1"
tokio.workspace = true
"quoted-name" = { version = "1" }
store = { package = "elohim-storage", path = "x" }
alias.package = "renamed-by-dotted-key"
alias.version = "1"
spread = { version = "1", features = [
    "not-a-dependency",
    "nor-this",
] }

[dependencies.dotted-table]
version = "1"

[dependencies.dotted-rename]
package = "elohim-storage-client"
path = "y"

[target.'cfg(unix)'.dependencies]
unix-only = "1"

[target."cfg(target_os = \"linux\")".dependencies.linux-dotted]
version = "1"

[target.x86_64-unknown-linux-gnu.build-dependencies]
build-target = "1"

[dev-dependencies]
dev-plain = "1"

[workspace.dependencies]
workspace-only = "1"

[[bin]]
name = "a-binary"
"#;

    #[test]
    fn every_dependency_form_is_read() {
        let deps = dependencies(EVERY_FORM);
        let normal: Vec<&str> = deps
            .iter()
            .filter(|d| d.kind == DepKind::Normal)
            .map(Dep::package_name)
            .collect();
        assert_eq!(
            normal,
            vec![
                "plain",
                "tokio",
                "quoted-name",
                "elohim-storage",
                "renamed-by-dotted-key",
                "spread",
                "dotted-table",
                "elohim-storage-client",
                "unix-only",
                "linux-dotted",
            ]
        );
        let unix = deps.iter().find(|d| d.name == "unix-only").unwrap();
        assert_eq!(unix.target.as_deref(), Some("cfg(unix)"));
        let linux = deps.iter().find(|d| d.name == "linux-dotted").unwrap();
        assert_eq!(
            linux.target.as_deref(),
            Some("cfg(target_os = \\\"linux\\\")")
        );
        assert!(deps
            .iter()
            .any(|d| d.name == "build-target" && d.kind == DepKind::Build));
        assert!(deps
            .iter()
            .any(|d| d.name == "dev-plain" && d.kind == DepKind::Dev));
        for absent in [
            "workspace-only",
            "a-binary",
            "not-a-dependency",
            "nor-this",
            "x",
        ] {
            assert!(
                !deps.iter().any(|d| d.name == absent),
                "{absent} is not a dependency"
            );
        }
    }
}
