//! Compare two verified method documents and two verified input documents field by field and line
//! by line, and say for each difference whether it can change what the build produces.
//!
//! The classes are deliberately asymmetric. `byte-affecting` means the difference reaches
//! something cargo, rustc or the linker reads, so it can change the bytes. `key-only` is claimed
//! only where this crate's own code shows the value never reaches the build for the `mesh-build`
//! recipe. Everything else is `unresolved`: a difference no code here proves harmless, which is
//! never read as harmless.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use eprfs_core::{BlobCid, MethodDescriptor};
use serde::Serialize;

use super::super::{
    is_c_family_var, CRATE_INPUT, MESH_BUILD, NATIVE_BUILD_ENV, PATH_INPUT, RESOURCE_CAPS,
};

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
    /// A comparison this tool did not make (the document's order past the alignment budget).
    NotCompared,
}

impl Change {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Change::Changed => "changed",
            Change::OrderOnly => "order-only",
            Change::OnlyA => "only-a",
            Change::OnlyB => "only-b",
            Change::NotCompared => "not-compared",
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

/// One line at its 1-based position in its document: its text without the `\n` (a `\r` stays),
/// and whether the `\n` was there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Located {
    pub position: usize,
    pub line: String,
    pub terminated: bool,
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
    if is_c_family_var(name) {
        return (
            Class::ByteAffecting,
            "the cc crate reads it as a C compiler or its flags",
        );
    }
    if RESOURCE_CAPS.contains(&name) {
        return (
            Class::Unresolved,
            "a resource cap; cargo hands a build script NUM_JOBS, and a script that writes it \
             into generated source changes the bytes, which nothing here rules out",
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

/// `a` and `b` differ (the caller compares them first), so this always adds at least one delta.
fn path_deltas(a: &str, b: &str, deltas: &mut Vec<MethodDelta>) {
    let before = deltas.len();
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
    if deltas.len() == before {
        // Only a repeated entry differs: lookups stop at the first match, yet the literal value
        // is what a build script reading PATH itself sees.
        deltas.push(method_delta(
            field,
            Change::Changed,
            (
                Class::Unresolved,
                "the literal PATH differs only by a repeated entry; tool lookups stop at the first \
                 match, but a build script reading PATH sees the literal value",
            ),
            Some(a),
            Some(b),
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
                let (raw_a, raw_b) = (left.unwrap_or_default(), right.unwrap_or_default());
                let before = deltas.len();
                let left = version_fields(raw_a);
                let right = version_fields(raw_b);
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
                if deltas.len() == before {
                    // Every field agrees by first occurrence: the text differs in order,
                    // repetition or whitespace, which the field view above cannot show.
                    fn sorted(text: &str) -> Vec<&str> {
                        let mut lines: Vec<&str> = text.lines().collect();
                        lines.sort_unstable();
                        lines
                    }
                    let (change, class) = if sorted(raw_a) == sorted(raw_b) {
                        (
                            Change::OrderOnly,
                            (
                                Class::Unresolved,
                                "the same rustc -vV lines in another order; one compiler prints \
                                 them in a fixed order, so another binary or wrapper printed one",
                            ),
                        )
                    } else {
                        (
                            Change::Changed,
                            (
                                Class::ByteAffecting,
                                "the compiler's identity text (rustc -vV) differs beyond its \
                                 named fields, so it is a different compiler output",
                            ),
                        )
                    };
                    deltas.push(method_delta(
                        "tools.rustc",
                        change,
                        class,
                        Some(raw_a),
                        Some(raw_b),
                    ));
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
    // Multiset difference: a flag present twice on one side and once on the other leaves one.
    let only = |side: &[String], other: &[String]| -> Vec<String> {
        let mut remaining: Vec<&String> = other.iter().collect();
        side.iter()
            .filter(|flag| match remaining.iter().position(|o| o == flag) {
                Some(index) => {
                    remaining.swap_remove(index);
                    false
                }
                None => true,
            })
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

/// One line of an input document exactly as written: its text without the `\n` terminator
/// (a `\r` before it stays in the text), and whether the terminator was there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Line<'a> {
    text: &'a str,
    terminated: bool,
}

/// Every line with its terminator kept, so `x\n`, `x\r\n` and a final `x` are three lines.
fn split_lines(document: &str) -> Vec<Line<'_>> {
    document
        .split_inclusive('\n')
        .map(|raw| match raw.strip_suffix('\n') {
            Some(text) => Line {
                text,
                terminated: true,
            },
            None => Line {
                text: raw,
                terminated: false,
            },
        })
        .collect()
}

/// The label of one input line: its first tab-separated field.
fn label(line: &str) -> &str {
    line.split('\t').next().unwrap_or(line)
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').skip(1).collect()
}

/// The most cells the ordered alignment may allocate (4 bytes each): 16 MiB.
pub(crate) const ALIGNMENT_CELLS: usize = 4_000_000;

/// The label of the delta that says the order of a large document was not compared.
pub(crate) const ALIGNMENT_LABEL: &str = "(alignment)";

/// Matched index pairs of `a` and `b`, and whether they are an ordered alignment. Common prefix
/// and suffix always align. The middle is a longest common subsequence when its table fits in
/// `budget` cells; otherwise its lines are matched as a multiset, and order is not compared.
fn common_lines(a: &[Line<'_>], b: &[Line<'_>], budget: usize) -> (Vec<(usize, usize)>, bool) {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (mid_a, mid_b) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let (n, m) = (mid_a.len(), mid_b.len());
    let mut pairs: Vec<(usize, usize)> = (0..prefix).map(|i| (i, i)).collect();
    let aligned = (n + 1)
        .checked_mul(m + 1)
        .is_some_and(|cells| cells <= budget);
    if aligned {
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
    } else {
        let mut free: HashMap<Line<'_>, VecDeque<usize>> = HashMap::new();
        for (j, line) in mid_b.iter().enumerate() {
            free.entry(*line).or_default().push_back(j);
        }
        for (i, line) in mid_a.iter().enumerate() {
            if let Some(j) = free.get_mut(line).and_then(VecDeque::pop_front) {
                pairs.push((prefix + i, prefix + j));
            }
        }
    }
    pairs.extend((0..suffix).map(|k| (a.len() - suffix + k, b.len() - suffix + k)));
    (pairs, aligned)
}

/// Both content fields parse as CIDs; equality of anything else proves nothing.
fn both_cids(a: &str, b: &str) -> bool {
    BlobCid::parse(a).is_ok() && BlobCid::parse(b).is_ok()
}

const NOT_A_CID: &str = "a content field is not a CID, so its equality proves nothing about the \
                         bytes it names";

/// The class of one paired line whose value changed.
fn changed_class(label: &str, a: &Line<'_>, b: &Line<'_>) -> (Class, &'static str) {
    if a.text.trim_end_matches('\r') == b.text.trim_end_matches('\r') {
        return (
            Class::Unresolved,
            "the line differs only in its terminator (a carriage return or the final newline); \
             the producer writes \\n-terminated lines, so another writer made one of them",
        );
    }
    let (left, right) = (fields(a.text), fields(b.text));
    if label.starts_with("cargo-config:") || label.starts_with("cargo-home:") {
        return (
            Class::ByteAffecting,
            "cargo reads this configuration, and its content CID differs",
        );
    }
    if label.starts_with("tool:") {
        return match (left.as_slice(), right.as_slice()) {
            ([_, ld], [_, rd]) if !both_cids(ld, rd) => (Class::Unresolved, NOT_A_CID),
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
            ([_, lc], [_, rc]) if !both_cids(lc, rc) => (Class::Unresolved, NOT_A_CID),
            ([lp, lc], [rp, rc]) if lc == rc && lp != rp => (
                Class::Unresolved,
                "equal bytes at another path; the linker records a shared object without a \
                 DT_SONAME by the path it was given, so the path can reach the output",
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

/// [`input_deltas_within`] at the default [`ALIGNMENT_CELLS`] budget.
pub(crate) fn input_deltas(a: &str, b: &str) -> Vec<InputDelta> {
    input_deltas_within(a, b, ALIGNMENT_CELLS)
}

/// Every differing line of two input documents, in document order. Lines keep their terminators.
/// Lines matched by the alignment are equal; an unmatched line whose exact text is unmatched on
/// the other side too is order-only; the rest pair by label as changed, else stand alone. Past
/// `budget` alignment cells, order is not compared and one `not-compared` delta says so.
pub(crate) fn input_deltas_within(a: &str, b: &str, budget: usize) -> Vec<InputDelta> {
    let left = split_lines(a);
    let right = split_lines(b);
    let (common, aligned) = common_lines(&left, &right, budget);
    let mut free_a: Vec<bool> = vec![true; left.len()];
    let mut free_b: Vec<bool> = vec![true; right.len()];
    for (i, j) in &common {
        free_a[*i] = false;
        free_b[*j] = false;
    }
    let at = |lines: &[Line<'_>], index: usize| Located {
        position: index + 1,
        line: lines[index].text.to_owned(),
        terminated: lines[index].terminated,
    };
    // Free lines of `b` by exact line and by label, earliest first.
    let mut by_line: HashMap<Line<'_>, VecDeque<usize>> = HashMap::new();
    let mut by_label: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (j, line) in right.iter().enumerate().filter(|(j, _)| free_b[*j]) {
        by_line.entry(*line).or_default().push_back(j);
        by_label.entry(label(line.text)).or_default().push_back(j);
    }
    let take = |queue: Option<&mut VecDeque<usize>>, free_b: &mut [bool]| -> Option<usize> {
        let queue = queue?;
        while let Some(j) = queue.pop_front() {
            if free_b[j] {
                free_b[j] = false;
                return Some(j);
            }
        }
        None
    };
    let mut deltas: Vec<(usize, usize, InputDelta)> = Vec::new();
    for i in 0..left.len() {
        if !free_a[i] {
            continue;
        }
        if let Some(j) = take(by_line.get_mut(&left[i]), &mut free_b) {
            free_a[i] = false;
            deltas.push((
                i,
                j,
                InputDelta {
                    label: label(left[i].text).to_owned(),
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
        let name = label(left[i].text);
        if let Some(j) = take(by_label.get_mut(name), &mut free_b) {
            free_a[i] = false;
            let (class, reason) = changed_class(name, &left[i], &right[j]);
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
            let (class, reason) = one_sided_class(label(line.text));
            deltas.push((
                i,
                usize::MAX,
                InputDelta {
                    label: label(line.text).to_owned(),
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
            let (class, reason) = one_sided_class(label(line.text));
            deltas.push((
                usize::MAX,
                j,
                InputDelta {
                    label: label(line.text).to_owned(),
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
    let mut deltas: Vec<InputDelta> = deltas.into_iter().map(|(_, _, delta)| delta).collect();
    if !aligned {
        deltas.push(InputDelta {
            label: ALIGNMENT_LABEL.to_owned(),
            change: Change::NotCompared,
            class: Class::Unresolved,
            reason: format!(
                "{} by {} lines exceed the {budget}-cell alignment budget; lines were matched as \
                 a multiset, so a line moved to another position is not reported",
                left.len(),
                right.len()
            ),
            a: None,
            b: None,
        });
    }
    deltas
}

/// When the two method CIDs differ and no field comparison explains it, say so: a CID mismatch is
/// never reported as identical.
pub(crate) fn unexplained_method(deltas: &mut Vec<MethodDelta>, a: &str, b: &str) {
    if a != b && deltas.is_empty() {
        deltas.push(method_delta(
            "(document)",
            Change::Changed,
            (
                Class::Unresolved,
                "the method CIDs differ, and no field comparison here explains it; the canonical \
                 bytes differ in a way this comparison does not model",
            ),
            Some(a),
            Some(b),
        ));
    }
}

/// As [`unexplained_method`], for the input documents.
pub(crate) fn unexplained_input(deltas: &mut Vec<InputDelta>, a: &str, b: &str) {
    if a != b && deltas.is_empty() {
        deltas.push(InputDelta {
            label: "(document)".to_owned(),
            change: Change::Changed,
            class: Class::Unresolved,
            reason: format!(
                "the input CIDs differ ({a} and {b}), and no line comparison here explains it"
            ),
            a: None,
            b: None,
        });
    }
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
    fn labels_are_key_only_resource_caps_unresolved_and_directories_byte_affecting() {
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
        // Cargo hands a build script NUM_JOBS, which it may write into generated source.
        assert_eq!(
            find(&deltas, "nonPathInputs.CARGO_BUILD_JOBS").class,
            Class::Unresolved
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

    fn cid(seed: &str) -> String {
        eprfs_core::BlobCid::compute_bytes(seed.as_bytes()).to_string()
    }

    /// A document whose content fields are real CIDs, named by their seeds.
    fn doc() -> String {
        format!(
            "elohim/eprfs\t040000 tree aaaa\n\
             cargo-config:0:config.toml\t{}\n\
             tool:link-driver\t/usr/bin/gcc\t{}\n\
             link:crt1.o\t/usr/lib64/crt1.o\t{}\n\
             link:libm.so\t/usr/lib64/libm.so\t{}\n\
             link:libc.so\t/usr/lib64/libc.so\t{}\n",
            cid("config"),
            cid("version"),
            cid("crt1"),
            cid("libm"),
            cid("libc")
        )
    }

    #[test]
    fn identical_documents_have_no_deltas() {
        assert!(input_deltas(&doc(), &doc()).is_empty());
    }

    #[test]
    fn a_link_path_with_equal_content_is_unresolved_and_new_content_byte_affecting() {
        // A shared object without DT_SONAME is recorded in DT_NEEDED by the path it was linked
        // from, so equal content at another path is not shown to leave the output unchanged.
        let moved = doc().replace("/usr/lib64/crt1.o", "/lib/x86_64-linux-gnu/crt1.o");
        let deltas = input_deltas(&doc(), &moved);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].label, "link:crt1.o");
        assert_eq!(deltas[0].change, Change::Changed);
        assert_eq!(deltas[0].class, Class::Unresolved);
        assert_eq!(deltas[0].a.as_ref().unwrap().position, 4);
        let changed = doc().replace(&cid("crt1"), &cid("other"));
        assert_eq!(
            input_deltas(&doc(), &changed)[0].class,
            Class::ByteAffecting
        );
    }

    #[test]
    fn equal_content_fields_that_are_not_cids_prove_nothing() {
        let forged = |path: &str| format!("link:crt1.o\t{path}\tnot-a-cid\n");
        let deltas = input_deltas(&forged("/usr/lib64/crt1.o"), &forged("/lib/crt1.o"));
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].class, Class::Unresolved);
        assert!(
            deltas[0].reason.contains("not a CID"),
            "{}",
            deltas[0].reason
        );
        let tool = |path: &str| format!("tool:link-driver\t{path}\tnot-a-cid\n");
        let deltas = input_deltas(&tool("/usr/bin/gcc"), &tool("/opt/gcc"));
        assert!(
            deltas[0].reason.contains("not a CID"),
            "{}",
            deltas[0].reason
        );
    }

    #[test]
    fn a_line_terminator_or_final_newline_difference_is_reported() {
        let deltas = input_deltas("x\tone\n", "x\tone\r\n");
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].class, Class::Unresolved);
        assert!(deltas[0].b.as_ref().unwrap().line.ends_with('\r'));
        let deltas = input_deltas("x\tone\n", "x\tone");
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert!(!deltas[0].b.as_ref().unwrap().terminated);
    }

    #[test]
    fn documents_past_the_alignment_budget_compare_as_a_multiset_and_say_so() {
        let a: String = (0..50).map(|i| format!("p{i}\tblob {i}\n")).collect();
        let b: String = (0..50).rev().map(|i| format!("p{i}\tblob {i}\n")).collect();
        let deltas = input_deltas_within(&a, &b, 100);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].label, ALIGNMENT_LABEL);
        assert_eq!(deltas[0].class, Class::Unresolved);
        let c = b.replace("p7\tblob 7\n", "p7\tblob seven\n");
        let deltas = input_deltas_within(&a, &c, 100);
        assert!(deltas
            .iter()
            .any(|d| d.label == "p7" && d.class == Class::ByteAffecting));
        assert!(deltas.iter().any(|d| d.label == ALIGNMENT_LABEL));
        // Within budget the same pair aligns and names its order-only moves.
        assert!(input_deltas_within(&a, &b, 1_000_000)
            .iter()
            .all(|d| d.change == Change::OrderOnly));
    }

    #[test]
    fn an_unexplained_cid_difference_is_an_unresolved_delta_never_identical() {
        let mut method = Vec::new();
        unexplained_method(&mut method, "bafya", "bafyb");
        assert_eq!(method.len(), 1);
        assert_eq!(method[0].class, Class::Unresolved);
        let mut none = Vec::new();
        unexplained_method(&mut none, "bafya", "bafya");
        assert!(none.is_empty());
        let mut inputs = Vec::new();
        unexplained_input(&mut inputs, "bafka", "bafkb");
        assert_eq!(inputs[0].class, Class::Unresolved);
    }

    #[test]
    fn rustc_lines_keep_their_order_and_multiplicity() {
        let mut b = descriptor();
        let rustc = descriptor().tools["rustc"].clone();
        let mut lines: Vec<&str> = rustc.lines().collect();
        lines.swap(1, 2);
        b.tools.insert("rustc".into(), lines.join("\n") + "\n");
        let deltas = method_deltas(&descriptor(), &b);
        assert!(
            !deltas.is_empty(),
            "a reordered rustc -vV is another output"
        );
        assert!(deltas.iter().all(|d| d.field.starts_with("tools.rustc")));
        b.tools
            .insert("rustc".into(), rustc.clone() + "binary: rustc\n");
        assert!(
            !method_deltas(&descriptor(), &b).is_empty(),
            "a repeated line counts"
        );
    }

    #[test]
    fn a_repeated_path_entry_and_a_repeated_flag_are_reported() {
        let mut b = descriptor();
        b.non_path_inputs
            .insert("PATH".into(), "/a/bin:/usr/bin:/bin:/bin".into());
        let deltas = method_deltas(&descriptor(), &b);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].field, "nonPathInputs.PATH");
        assert_eq!(deltas[0].class, Class::Unresolved);
        let mut c = descriptor();
        c.flags.push("--bin=epr".into());
        let deltas = method_deltas(&descriptor(), &c);
        assert_eq!(deltas.len(), 1, "{deltas:#?}");
        assert_eq!(deltas[0].change, Change::OnlyB);
        assert_eq!(deltas[0].b.as_deref(), Some("--bin=epr"));
    }

    #[test]
    fn a_tool_path_with_equal_output_is_unresolved() {
        let moved = doc().replace("/usr/bin/gcc", "/nix/store/x-gcc/bin/gcc");
        let deltas = input_deltas(&doc(), &moved);
        assert_eq!(deltas[0].class, Class::Unresolved);
        let changed = doc().replace(&cid("version"), &cid("other"));
        assert_eq!(
            input_deltas(&doc(), &changed)[0].class,
            Class::ByteAffecting
        );
    }

    #[test]
    fn a_reorder_is_order_only_and_names_both_positions() {
        let (libm, libc) = (
            format!("link:libm.so\t/usr/lib64/libm.so\t{}\n", cid("libm")),
            format!("link:libc.so\t/usr/lib64/libc.so\t{}\n", cid("libc")),
        );
        let reordered = doc().replace(&format!("{libm}{libc}"), &format!("{libc}{libm}"));
        let deltas = input_deltas(&doc(), &reordered);
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
        let other = doc().replace("tree aaaa", "tree bbbb").replace(
            &format!("cargo-config:0:config.toml\t{}\n", cid("config")),
            "",
        ) + "crates/extra\t040000 tree cccc\n";
        let deltas = input_deltas(&doc(), &other);
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
