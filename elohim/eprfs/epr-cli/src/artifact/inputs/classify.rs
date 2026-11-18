//! Compare two verified method documents and two verified input documents field by field and line
//! by line, and say for each difference whether it can change what the build produces.
//!
//! The classes are deliberately asymmetric. `byte-affecting` means the difference reaches
//! something cargo, rustc or the linker reads, so it can change the bytes. `key-only` is claimed
//! only where this crate's own code shows the value never reaches the build for the `mesh-build`
//! recipe. Everything else is `unresolved`: a difference no code here proves harmless, which is
//! never read as harmless.

use std::collections::{BTreeMap, BTreeSet};

use eprfs_core::MethodDescriptor;
use serde::Serialize;

use super::super::{CRATE_INPUT, MESH_BUILD, NATIVE_BUILD_ENV, PATH_INPUT, RESOURCE_CAPS};

/// Whether a difference can reach the build's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Class {
    ByteAffecting,
    KeyOnly,
    Unresolved,
}

impl Class {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Class::ByteAffecting => "byte-affecting",
            Class::KeyOnly => "key-only",
            Class::Unresolved => "unresolved",
        }
    }
}

/// How one entry differs between the two documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Change {
    /// Present on both sides with different values.
    Changed,
    /// The same entry at a different position: only the order differs.
    OrderOnly,
    /// Present only in the first document.
    OnlyA,
    /// Present only in the second document.
    OnlyB,
}

impl Change {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Change::Changed => "changed",
            Change::OrderOnly => "order-only",
            Change::OnlyA => "only-a",
            Change::OnlyB => "only-b",
        }
    }
}

/// One differing method field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MethodDelta {
    pub field: String,
    pub change: Change,
    pub class: Class,
    pub reason: String,
    pub a: Option<String>,
    pub b: Option<String>,
}

/// One line at its 1-based position in its document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Located {
    pub position: usize,
    pub line: String,
}

/// One differing input line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputDelta {
    pub label: String,
    pub change: Change,
    pub class: Class,
    pub reason: String,
    pub a: Option<Located>,
    pub b: Option<Located>,
}

fn method_delta(
    field: impl Into<String>,
    change: Change,
    (class, reason): (Class, &str),
    a: Option<&str>,
    b: Option<&str>,
) -> MethodDelta {
    MethodDelta {
        field: field.into(),
        change,
        class,
        reason: reason.to_owned(),
        a: a.map(str::to_owned),
        b: b.map(str::to_owned),
    }
}

/// Entries present on both sides, compared by name, then by value.
fn keyed_change<'a>(a: Option<&'a str>, b: Option<&'a str>) -> Option<Change> {
    match (a, b) {
        (Some(a), Some(b)) if a == b => None,
        (Some(_), Some(_)) => Some(Change::Changed),
        (Some(_), None) => Some(Change::OnlyA),
        (None, Some(_)) => Some(Change::OnlyB),
        (None, None) => None,
    }
}

/// `rustc -vV` as `field -> value`; a line without `: ` is keyed by its whole text.
fn version_fields(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once(": ") {
            Some((field, value)) => (field.trim().to_owned(), value.trim().to_owned()),
            None => (line.trim().to_owned(), line.trim().to_owned()),
        })
        .collect()
}

fn rustflags_remap(descriptor: &MethodDescriptor) -> bool {
    descriptor
        .non_path_inputs
        .iter()
        .any(|(name, value)| name.ends_with("RUSTFLAGS") && value.contains("remap-path-prefix"))
}

/// The class of one recorded non-path input, by name, for the mesh-build recipe.
fn non_path_class(name: &str, remapped: bool) -> (Class, &'static str) {
    if name == CRATE_INPUT {
        return (Class::ByteAffecting, "names the crate cargo builds");
    }
    if name == "RUSTFLAGS" || name.ends_with("_RUSTFLAGS") {
        return (Class::ByteAffecting, "passed to every rustc invocation");
    }
    if name.starts_with("target.") {
        return (Class::ByteAffecting, "a declared target input");
    }
    if NATIVE_BUILD_ENV.contains(&name) {
        return (
            Class::ByteAffecting,
            "cargo and build scripts hand it to native compilation",
        );
    }
    if RESOURCE_CAPS.contains(&name) {
        return (
            Class::KeyOnly,
            "a resource cap: it sets how many jobs run, never rustc's arguments or inputs \
             (RESOURCE_CAPS; describe omits it)",
        );
    }
    if matches!(name, "CARGO_HOME" | "CARGO_TARGET_DIR" | "RUSTUP_HOME") {
        return if remapped {
            (
                Class::Unresolved,
                "a directory prefix that reaches debug info and panic locations; a recorded \
                 --remap-path-prefix may cover it, which is not checked here",
            )
        } else {
            (
                Class::ByteAffecting,
                "a directory prefix that reaches debug info and panic locations (registry \
                 sources, OUT_DIR); no --remap-path-prefix is recorded",
            )
        };
    }
    if matches!(name, "RUST_BACKTRACE" | "RUST_LOG") {
        return (
            Class::Unresolved,
            "rustc's diagnostics read it, and a build script may too",
        );
    }
    if name == "RUSTC_WRAPPER" || name == "RUSTC_WORKSPACE_WRAPPER" {
        return (
            Class::Unresolved,
            "a wrapper around every rustc invocation; whether it changes the output is not \
             observed",
        );
    }
    if name.starts_with("CARGO_") || name.starts_with("RUST") {
        return (
            Class::ByteAffecting,
            "cargo reads CARGO_* and RUST* variables as build configuration",
        );
    }
    (
        Class::Unresolved,
        "a caller-declared input; its effect on the build is not modelled",
    )
}

const PATH_REASON: &str = "cargo, rustc, build scripts and the linker driver look tools up on \
                           PATH; the key observes only the selections in its tool: and link: \
                           lines, and any other lookup is unobserved";

const PATH_RELATIVE_REASON: &str = "a relative PATH entry resolves against each process's working \
                                    directory, so what it names differs per crate and build \
                                    script; the key observes only its tool: and link: lines";

fn path_deltas(a: &str, b: &str, deltas: &mut Vec<MethodDelta>) {
    let field = format!("nonPathInputs.{PATH_INPUT}");
    let left: Vec<&str> = a.split(':').collect();
    let right: Vec<&str> = b.split(':').collect();
    let left_set: BTreeSet<&str> = left.iter().copied().collect();
    let right_set: BTreeSet<&str> = right.iter().copied().collect();
    let reason = |entry: &str| {
        if entry.starts_with('/') {
            (Class::Unresolved, PATH_REASON)
        } else {
            (Class::Unresolved, PATH_RELATIVE_REASON)
        }
    };
    for (position, entry) in left.iter().enumerate() {
        if !right_set.contains(entry) {
            deltas.push(method_delta(
                format!("{field}[{}]", position + 1),
                Change::OnlyA,
                reason(entry),
                Some(entry),
                None,
            ));
        }
    }
    for (position, entry) in right.iter().enumerate() {
        if !left_set.contains(entry) {
            deltas.push(method_delta(
                format!("{field}[{}]", position + 1),
                Change::OnlyB,
                reason(entry),
                None,
                Some(entry),
            ));
        }
    }
    // Resolution stops at an entry's first occurrence, so a repeat never changes a lookup.
    let first_shared = |entries: &[&'_ str], other: &BTreeSet<&str>| -> Vec<String> {
        let mut seen = BTreeSet::new();
        entries
            .iter()
            .filter(|entry| other.contains(*entry) && seen.insert(**entry))
            .map(|entry| (*entry).to_owned())
            .collect()
    };
    let common_a = first_shared(&left, &right_set);
    let common_b = first_shared(&right, &left_set);
    if common_a != common_b {
        deltas.push(method_delta(
            format!("{field} order"),
            Change::OrderOnly,
            (
                Class::Unresolved,
                "the shared entries (first occurrences) are searched in a different order; the \
                 first match wins, so a tool present in two of them resolves differently",
            ),
            Some(&common_a.join(":")),
            Some(&common_b.join(":")),
        ));
    }
}

/// Every differing field of two method documents. The descriptor order is fixed: name, step,
/// params, exec, embedder, deterministic, tools, flags, view, non-path inputs.
pub(crate) fn method_deltas(a: &MethodDescriptor, b: &MethodDescriptor) -> Vec<MethodDelta> {
    let mut deltas = Vec::new();
    let mesh = a.name == MESH_BUILD && b.name == MESH_BUILD;
    if a.name != b.name {
        deltas.push(method_delta(
            "name",
            Change::Changed,
            (
                Class::Unresolved,
                "two different recipes; the classes below assume mesh-build",
            ),
            Some(&a.name),
            Some(&b.name),
        ));
    }
    let label = if mesh {
        (
            Class::KeyOnly,
            "a recipe label describe writes from --project; cargo never receives it, and the \
             crate, flags and view that select the build are keyed separately",
        )
    } else {
        (Class::Unresolved, "a label of an unmodelled recipe")
    };
    if let Some(change) = keyed_change(a.step.as_deref(), b.step.as_deref()) {
        deltas.push(method_delta(
            "step",
            change,
            label,
            a.step.as_deref(),
            b.step.as_deref(),
        ));
    }
    let unset = (
        Class::Unresolved,
        "mesh-build never sets it, so this is not a mesh-build describe output",
    );
    let cid = |c: &Option<eprfs_core::BlobCid>| c.as_ref().map(ToString::to_string);
    for (field, left, right) in [
        ("params", cid(&a.params), cid(&b.params)),
        ("exec", cid(&a.exec), cid(&b.exec)),
        ("embedder", a.embedder.clone(), b.embedder.clone()),
    ] {
        if let Some(change) = keyed_change(left.as_deref(), right.as_deref()) {
            deltas.push(method_delta(
                field,
                change,
                unset,
                left.as_deref(),
                right.as_deref(),
            ));
        }
    }
    if a.deterministic != b.deterministic {
        deltas.push(method_delta(
            "deterministic",
            Change::Changed,
            (
                Class::KeyOnly,
                "a declaration bound to the record (C15); cargo never reads it",
            ),
            Some(&a.deterministic.to_string()),
            Some(&b.deterministic.to_string()),
        ));
    }
    tool_deltas(a, b, &mut deltas);
    flag_deltas(&a.flags, &b.flags, &mut deltas);
    let left: BTreeSet<_> = a.view().paths().cloned().collect();
    let right: BTreeSet<_> = b.view().paths().cloned().collect();
    let view_reason = (
        Class::Unresolved,
        "the declared view names what the key reads (the step's sources and cargo's path \
         packages); a path package cargo resolves differently also shows as Cargo.toml or \
         Cargo.lock lines, and this path alone is not shown to reach the build",
    );
    for path in left.symmetric_difference(&right) {
        let text = path.to_string();
        let (change, a_side, b_side) = if left.contains(path) {
            (Change::OnlyA, Some(text.as_str()), None)
        } else {
            (Change::OnlyB, None, Some(text.as_str()))
        };
        deltas.push(method_delta("view", change, view_reason, a_side, b_side));
    }
    let remapped = rustflags_remap(a) || rustflags_remap(b);
    let names: BTreeSet<&String> = a
        .non_path_inputs
        .keys()
        .chain(b.non_path_inputs.keys())
        .collect();
    for name in names {
        let left = a.non_path_inputs.get(name).map(String::as_str);
        let right = b.non_path_inputs.get(name).map(String::as_str);
        let Some(change) = keyed_change(left, right) else {
            continue;
        };
        if name == PATH_INPUT {
            if let (Some(left), Some(right)) = (left, right) {
                path_deltas(left, right, &mut deltas);
                continue;
            }
        }
        let class = if name == PATH_INPUT {
            (Class::Unresolved, PATH_REASON)
        } else {
            non_path_class(name, remapped)
        };
        deltas.push(method_delta(
            format!("nonPathInputs.{name}"),
            change,
            class,
            left,
            right,
        ));
    }
    deltas
}

fn tool_deltas(a: &MethodDescriptor, b: &MethodDescriptor, deltas: &mut Vec<MethodDelta>) {
    let names: BTreeSet<&String> = a.tools.keys().chain(b.tools.keys()).collect();
    let same = |name: &str| a.tools.get(name) == b.tools.get(name);
    for name in names {
        let left = a.tools.get(name).map(String::as_str);
        let right = b.tools.get(name).map(String::as_str);
        let Some(change) = keyed_change(left, right) else {
            continue;
        };
        match name.as_str() {
            "rustc" if change == Change::Changed => {
                let left = version_fields(left.unwrap_or_default());
                let right = version_fields(right.unwrap_or_default());
                let fields: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
                for field in fields {
                    let (l, r) = (
                        left.get(field).map(String::as_str),
                        right.get(field).map(String::as_str),
                    );
                    if let Some(change) = keyed_change(l, r) {
                        deltas.push(method_delta(
                            format!("tools.rustc[{field}]"),
                            change,
                            (
                                Class::ByteAffecting,
                                "the compiler's identity (rustc -vV) differs, so it is a \
                                 different compiler",
                            ),
                            l,
                            r,
                        ));
                    }
                }
            }
            "rustc" | "cargo" => deltas.push(method_delta(
                format!("tools.{name}"),
                change,
                (
                    Class::ByteAffecting,
                    "a different compiler or cargo plans and invokes the build differently",
                ),
                left,
                right,
            )),
            "toolchain" => {
                let class = if same("rustc") && same("cargo") {
                    (
                        Class::Unresolved,
                        "the selector differs while rustc -vV and cargo -V agree; the \
                         toolchain's installed std and bundled linker are keyed only through \
                         that identity",
                    )
                } else {
                    (
                        Class::ByteAffecting,
                        "selects a different compiler (see tools.rustc and tools.cargo)",
                    )
                };
                deltas.push(method_delta("tools.toolchain", change, class, left, right));
            }
            _ => deltas.push(method_delta(
                format!("tools.{name}"),
                change,
                (
                    Class::Unresolved,
                    "a tool mesh-build does not record; its effect is not modelled",
                ),
                left,
                right,
            )),
        }
    }
}

fn flag_name(flag: &str) -> &str {
    flag.split_once('=').map_or(flag, |(name, _)| name)
}

fn flag_deltas(a: &[String], b: &[String], deltas: &mut Vec<MethodDelta>) {
    let mut left: Vec<&String> = a.iter().collect();
    let mut right: Vec<&String> = b.iter().collect();
    left.sort();
    right.sort();
    if left == right {
        if a != b {
            deltas.push(method_delta(
                "flags order",
                Change::OrderOnly,
                (
                    Class::Unresolved,
                    "the same cargo selection flags in another order; describe writes them in a \
                     fixed order, so a reorder means another producer",
                ),
                Some(&a.join(" ")),
                Some(&b.join(" ")),
            ));
        }
        return;
    }
    let reason = (
        Class::ByteAffecting,
        "cargo builds a different selection (binary, features, profile or target)",
    );
    let only = |side: &[String], other: &[String]| -> Vec<String> {
        side.iter()
            .filter(|flag| !other.contains(flag))
            .cloned()
            .collect()
    };
    let mut only_b = only(b, a);
    for flag in only(a, b) {
        let paired = only_b
            .iter()
            .position(|other| flag_name(other) == flag_name(&flag));
        match paired.map(|index| only_b.remove(index)) {
            Some(other) => deltas.push(method_delta(
                format!("flags[{}]", flag_name(&flag)),
                Change::Changed,
                reason,
                Some(&flag),
                Some(&other),
            )),
            None => deltas.push(method_delta(
                format!("flags[{}]", flag_name(&flag)),
                Change::OnlyA,
                reason,
                Some(&flag),
                None,
            )),
        }
    }
    for flag in only_b {
        deltas.push(method_delta(
            format!("flags[{}]", flag_name(&flag)),
            Change::OnlyB,
            reason,
            None,
            Some(&flag),
        ));
    }
}

/// The label of one input line: its first tab-separated field.
fn label(line: &str) -> &str {
    line.split('\t').next().unwrap_or(line)
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').skip(1).collect()
}

/// Indices of a longest common subsequence of `a` and `b`, as matched pairs.
fn common_subsequence(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (mid_a, mid_b) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let (n, m) = (mid_a.len(), mid_b.len());
    // table[i][j]: the LCS length of mid_a[i..] and mid_b[j..].
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if mid_a[i] == mid_b[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let mut pairs: Vec<(usize, usize)> = (0..prefix).map(|i| (i, i)).collect();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if mid_a[i] == mid_b[j] {
            pairs.push((prefix + i, prefix + j));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs.extend((0..suffix).map(|k| (a.len() - suffix + k, b.len() - suffix + k)));
    pairs
}

/// The class of one paired line whose value changed.
fn changed_class(label: &str, a: &str, b: &str) -> (Class, &'static str) {
    let (left, right) = (fields(a), fields(b));
    if label.starts_with("cargo-config:") || label.starts_with("cargo-home:") {
        return (
            Class::ByteAffecting,
            "cargo reads this configuration, and its content CID differs",
        );
    }
    if label.starts_with("tool:") {
        return match (left.as_slice(), right.as_slice()) {
            ([lp, ld], [rp, rd]) if ld == rd && lp != rp => (
                Class::Unresolved,
                "equal probe output from a different executable; the executable's own bytes are \
                 not keyed",
            ),
            _ => (
                Class::ByteAffecting,
                "a different probe output (--version or the predefined-macro dump): a different \
                 tool or target default",
            ),
        };
    }
    if label.starts_with("link:") {
        return match (left.as_slice(), right.as_slice()) {
            ([lp, lc], [rp, rc]) if lc == rc && lp != rp => (
                Class::KeyOnly,
                "the linker reads this file's bytes, which are equal by content CID; the \
                 resolved location is not an input to the link",
            ),
            _ => (Class::ByteAffecting, "the linker reads different bytes"),
        };
    }
    (
        Class::ByteAffecting,
        "the source the build reads at this path differs (git mode, type, object or gitlink)",
    )
}

fn one_sided_class(label: &str) -> (Class, &'static str) {
    if label.starts_with("cargo-config:") || label.starts_with("cargo-home:") {
        (
            Class::ByteAffecting,
            "a cargo configuration slot present in only one document",
        )
    } else if label.starts_with("tool:") {
        (Class::ByteAffecting, "a tool selected in only one document")
    } else if label.starts_with("link:") {
        (
            Class::ByteAffecting,
            "a link input resolved in only one document",
        )
    } else {
        (
            Class::Unresolved,
            "a path in only one method's view; see the method's view field",
        )
    }
}

const ORDER_REASON: &str = "the same line at another position; the producer writes view, config, \
                            tool and link lines in a fixed order, so a move means another probe \
                            order, and link order can matter to the linker";

/// Every differing line of two input documents, in document order: lines matched by a longest
/// common subsequence are equal; an unmatched line whose exact text is unmatched on the other side
/// too is order-only; the rest pair by label as changed, else stand alone.
pub(crate) fn input_deltas(a: &str, b: &str) -> Vec<InputDelta> {
    let left: Vec<&str> = a.lines().collect();
    let right: Vec<&str> = b.lines().collect();
    let common = common_subsequence(&left, &right);
    let mut free_a: Vec<bool> = vec![true; left.len()];
    let mut free_b: Vec<bool> = vec![true; right.len()];
    for (i, j) in &common {
        free_a[*i] = false;
        free_b[*j] = false;
    }
    let at = |lines: &[&str], index: usize| Located {
        position: index + 1,
        line: lines[index].to_owned(),
    };
    let mut deltas: Vec<(usize, usize, InputDelta)> = Vec::new();
    fn take(free: &mut [bool], matches: &dyn Fn(usize) -> bool) -> Option<usize> {
        let index = (0..free.len()).find(|&j| free[j] && matches(j))?;
        free[index] = false;
        Some(index)
    }
    for i in 0..left.len() {
        if !free_a[i] {
            continue;
        }
        if let Some(j) = take(&mut free_b, &|j| right[j] == left[i]) {
            free_a[i] = false;
            deltas.push((
                i,
                j,
                InputDelta {
                    label: label(left[i]).to_owned(),
                    change: Change::OrderOnly,
                    class: Class::Unresolved,
                    reason: ORDER_REASON.to_owned(),
                    a: Some(at(&left, i)),
                    b: Some(at(&right, j)),
                },
            ));
        }
    }
    for i in 0..left.len() {
        if !free_a[i] {
            continue;
        }
        let name = label(left[i]);
        if let Some(j) = take(&mut free_b, &|j| label(right[j]) == name) {
            free_a[i] = false;
            let (class, reason) = changed_class(name, left[i], right[j]);
            deltas.push((
                i,
                j,
                InputDelta {
                    label: name.to_owned(),
                    change: Change::Changed,
                    class,
                    reason: reason.to_owned(),
                    a: Some(at(&left, i)),
                    b: Some(at(&right, j)),
                },
            ));
        }
    }
    for (i, line) in left.iter().enumerate() {
        if free_a[i] {
            let (class, reason) = one_sided_class(label(line));
            deltas.push((
                i,
                usize::MAX,
                InputDelta {
                    label: label(line).to_owned(),
                    change: Change::OnlyA,
                    class,
                    reason: reason.to_owned(),
                    a: Some(at(&left, i)),
                    b: None,
                },
            ));
        }
    }
    for (j, line) in right.iter().enumerate() {
        if free_b[j] {
            let (class, reason) = one_sided_class(label(line));
            deltas.push((
                usize::MAX,
                j,
                InputDelta {
                    label: label(line).to_owned(),
                    change: Change::OnlyB,
                    class,
                    reason: reason.to_owned(),
                    a: None,
                    b: Some(at(&right, j)),
                },
            ));
        }
    }
    deltas.sort_by_key(|(i, j, _)| (*i, *j));
    deltas.into_iter().map(|(_, _, delta)| delta).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::TreePath;

    fn descriptor() -> MethodDescriptor {
        MethodDescriptor {
            name: MESH_BUILD.into(),
            tools: BTreeMap::from([
                (
                    "rustc".into(),
                    "rustc 1.100.0 (abc 2026-11-01)\nbinary: rustc\nrelease: 1.100.0\n\
                     LLVM version: 22.1.2\n"
                        .into(),
                ),
                ("cargo".into(), "cargo 1.100.0 (def 2026-10-30)".into()),
                ("toolchain".into(), "1.100.0".into()),
            ]),
            flags: vec![
                "--bin=epr".into(),
                "--profile=dev".into(),
                "--target=x86_64-unknown-linux-gnu".into(),
            ],
            params: None,
            exec: None,
            step: Some("mesh-build:eprfs".into()),
            embedder: None,
            view: vec![TreePath::from_slash_bytes(b"elohim/eprfs").unwrap()],
            non_path_inputs: BTreeMap::from([
                ("PATH".into(), "/a/bin:/usr/bin:/bin".into()),
                ("RUSTFLAGS".into(), String::new()),
                ("crate".into(), "elohim/eprfs/epr-cli".into()),
            ]),
            deterministic: false,
        }
    }

    fn find<'a>(deltas: &'a [MethodDelta], field: &str) -> &'a MethodDelta {
        deltas
            .iter()
            .find(|delta| delta.field == field)
            .unwrap_or_else(|| panic!("no {field} in {deltas:#?}"))
    }

    #[test]
    fn identical_methods_have_no_deltas() {
        assert!(method_deltas(&descriptor(), &descriptor()).is_empty());
    }

    #[test]
    fn a_rustc_version_line_is_byte_affecting_and_named_by_its_field() {
        let mut b = descriptor();
        b.tools.insert(
            "rustc".into(),
            "rustc 1.100.0 (abc 2026-11-01)\nbinary: rustc\nrelease: 1.100.0\n\
             LLVM version: 22.1.3\n"
                .into(),
        );
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        let llvm = find(&deltas, "tools.rustc[LLVM version]");
        assert_eq!(llvm.class, Class::ByteAffecting);
        assert_eq!(llvm.a.as_deref(), Some("22.1.2"));
        assert_eq!(llvm.b.as_deref(), Some("22.1.3"));
    }

    #[test]
    fn a_toolchain_selector_alone_is_unresolved_and_with_a_new_rustc_byte_affecting() {
        let mut b = descriptor();
        b.tools.insert("toolchain".into(), "unpinned".into());
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(find(&deltas, "tools.toolchain").class, Class::Unresolved);
        b.tools.insert("cargo".into(), "cargo 1.99.0".into());
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(find(&deltas, "tools.toolchain").class, Class::ByteAffecting);
        assert_eq!(find(&deltas, "tools.cargo").class, Class::ByteAffecting);
    }

    #[test]
    fn path_reports_entries_and_order_separately_and_never_as_key_only() {
        let mut b = descriptor();
        b.non_path_inputs
            .insert("PATH".into(), "/usr/bin:/a/bin:/bin:/extra".into());
        let deltas = method_deltas(&descriptor(), &b);
        let extra = find(&deltas, "nonPathInputs.PATH[4]");
        assert_eq!(extra.change, Change::OnlyB);
        assert_eq!(extra.b.as_deref(), Some("/extra"));
        let order = find(&deltas, "nonPathInputs.PATH order");
        assert_eq!(order.change, Change::OrderOnly);
        assert_eq!(order.a.as_deref(), Some("/a/bin:/usr/bin:/bin"));
        assert_eq!(order.b.as_deref(), Some("/usr/bin:/a/bin:/bin"));
        assert!(deltas.iter().all(|d| d.class == Class::Unresolved));
    }

    #[test]
    fn a_repeated_path_entry_is_not_an_order_change_and_a_relative_one_says_so() {
        let mut b = descriptor();
        b.non_path_inputs.insert(
            "PATH".into(),
            "node_modules/.bin:/a/bin:/usr/bin:/a/bin:/bin".into(),
        );
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        let relative = find(&deltas, "nonPathInputs.PATH[1]");
        assert_eq!(relative.change, Change::OnlyB);
        assert_eq!(relative.class, Class::Unresolved);
        assert!(relative.reason.contains("working directory"));
    }

    #[test]
    fn labels_and_resource_caps_are_key_only_and_directories_byte_affecting() {
        let mut b = descriptor();
        b.step = Some("mesh-build:epr".into());
        b.deterministic = true;
        b.non_path_inputs
            .insert("CARGO_BUILD_JOBS".into(), "2".into());
        b.non_path_inputs
            .insert("CARGO_TARGET_DIR".into(), "/pool/dev".into());
        b.non_path_inputs
            .insert("RUSTFLAGS".into(), "-C debuginfo=0".into());
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(find(&deltas, "step").class, Class::KeyOnly);
        assert_eq!(find(&deltas, "deterministic").class, Class::KeyOnly);
        assert_eq!(
            find(&deltas, "nonPathInputs.CARGO_BUILD_JOBS").class,
            Class::KeyOnly
        );
        let target = find(&deltas, "nonPathInputs.CARGO_TARGET_DIR");
        assert_eq!(target.class, Class::ByteAffecting);
        assert_eq!(target.change, Change::OnlyB);
        assert_eq!(
            find(&deltas, "nonPathInputs.RUSTFLAGS").class,
            Class::ByteAffecting
        );
        // A remap flag on either side leaves a directory prefix unresolved, never key-only.
        b.non_path_inputs.insert(
            "RUSTFLAGS".into(),
            "--remap-path-prefix=/pool=/target".into(),
        );
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(
            find(&deltas, "nonPathInputs.CARGO_TARGET_DIR").class,
            Class::Unresolved
        );
    }

    #[test]
    fn flags_pair_by_name_and_a_reorder_is_order_only() {
        let mut b = descriptor();
        b.flags[0] = "--bin=eprfs".into();
        b.flags.push("--features=lsp".into());
        let deltas = method_deltas(&descriptor(), &b);
        let bin = find(&deltas, "flags[--bin]");
        assert_eq!(bin.change, Change::Changed);
        assert_eq!(bin.class, Class::ByteAffecting);
        assert_eq!(find(&deltas, "flags[--features]").change, Change::OnlyB);
        let mut c = descriptor();
        c.flags.reverse();
        let deltas = method_deltas(&descriptor(), &c);
        assert_eq!(find(&deltas, "flags order").change, Change::OrderOnly);
    }

    #[test]
    fn a_differing_view_path_is_unresolved() {
        let mut b = descriptor();
        b.view
            .push(TreePath::from_slash_bytes(b"crates/extra").unwrap());
        let deltas = method_deltas(&descriptor(), &b);
        let view = find(&deltas, "view");
        assert_eq!(view.change, Change::OnlyB);
        assert_eq!(view.class, Class::Unresolved);
    }

    const DOC: &str = "elohim/eprfs\t040000 tree aaaa\n\
                       cargo-config:0:config.toml\tbafkconfig\n\
                       tool:link-driver\t/usr/bin/gcc\tbafkversion\n\
                       link:crt1.o\t/usr/lib64/crt1.o\tbafkcrt1\n\
                       link:libm.so\t/usr/lib64/libm.so\tbafklibm\n\
                       link:libc.so\t/usr/lib64/libc.so\tbafklibc\n";

    #[test]
    fn identical_documents_have_no_deltas() {
        assert!(input_deltas(DOC, DOC).is_empty());
    }

    #[test]
    fn a_link_path_with_equal_content_is_key_only_and_new_content_byte_affecting() {
        let moved = DOC.replace("/usr/lib64/crt1.o", "/lib/x86_64-linux-gnu/crt1.o");
        let deltas = input_deltas(DOC, &moved);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].label, "link:crt1.o");
        assert_eq!(deltas[0].change, Change::Changed);
        assert_eq!(deltas[0].class, Class::KeyOnly);
        assert_eq!(deltas[0].a.as_ref().unwrap().position, 4);
        let changed = DOC.replace("bafkcrt1", "bafkother");
        assert_eq!(input_deltas(DOC, &changed)[0].class, Class::ByteAffecting);
    }

    #[test]
    fn a_tool_path_with_equal_output_is_unresolved() {
        let moved = DOC.replace("/usr/bin/gcc", "/nix/store/x-gcc/bin/gcc");
        let deltas = input_deltas(DOC, &moved);
        assert_eq!(deltas[0].class, Class::Unresolved);
        let changed = DOC.replace("bafkversion", "bafkother");
        assert_eq!(input_deltas(DOC, &changed)[0].class, Class::ByteAffecting);
    }

    #[test]
    fn a_reorder_is_order_only_and_names_both_positions() {
        let reordered = DOC.replace(
            "link:libm.so\t/usr/lib64/libm.so\tbafklibm\nlink:libc.so\t/usr/lib64/libc.so\tbafklibc\n",
            "link:libc.so\t/usr/lib64/libc.so\tbafklibc\nlink:libm.so\t/usr/lib64/libm.so\tbafklibm\n",
        );
        let deltas = input_deltas(DOC, &reordered);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        let delta = &deltas[0];
        assert_eq!(delta.change, Change::OrderOnly);
        assert_eq!(delta.class, Class::Unresolved);
        let (a, b) = (delta.a.as_ref().unwrap(), delta.b.as_ref().unwrap());
        assert_eq!(a.line, b.line);
        assert_ne!(a.position, b.position);
    }

    #[test]
    fn source_and_one_sided_lines_are_classified() {
        let other = DOC
            .replace("tree aaaa", "tree bbbb")
            .replace("cargo-config:0:config.toml\tbafkconfig\n", "")
            + "crates/extra\t040000 tree cccc\n";
        let deltas = input_deltas(DOC, &other);
        let by_label = |label: &str| deltas.iter().find(|d| d.label == label).unwrap();
        assert_eq!(by_label("elohim/eprfs").class, Class::ByteAffecting);
        let config = by_label("cargo-config:0:config.toml");
        assert_eq!(config.change, Change::OnlyA);
        assert_eq!(config.class, Class::ByteAffecting);
        let extra = by_label("crates/extra");
        assert_eq!(extra.change, Change::OnlyB);
        assert_eq!(extra.class, Class::Unresolved);
    }
}
