//! Build rails over this crate's own source and manifest (interfaces spec §1.3).
//!
//! - **R-P8, one ranking home.** Placement rankings live in `elohim-placement-core`. This crate
//!   holds no `impl PlacementStrategy for` and no `impl Ranking for` outside the shim in
//!   `reconcile/placement.rs` (test modules excepted), and no greedy-diversity pass (an
//!   `if <domain-set>.insert(..)` loop) in `src/services/` or `src/reconcile/`. It does not look
//!   for `fn rank(`: `services/standing.rs`, `services/feedback_projector.rs` and `db/models.rs`
//!   hold unrelated rank functions (D19).
//! - **R-E3, the storage half of the eprfs direction.** This crate may depend on `eprfs-core` and
//!   `eprfs-serve` (its `testing` harness is consumed here as a dev-dependency), never on
//!   `eprfs-storage` or `eprfs-local`. The other half (no eprfs adapter depends on this crate) is
//!   `elohim/eprfs/eprfs-storage/tests/boundary.rs`.
//!
//! Both read files at test time from `CARGO_MANIFEST_DIR`, never through `include_str!`, so the
//! image's `check` stage needs no extra COPY (`just image-includes`).

use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// Rustfmt-formatted production source, excluding each `#[cfg(test)]` module rather than
/// everything after the first one. A module's closing brace has its declaration's indentation;
/// the crate's preceding format gate guarantees that shape. Test-only source files are skipped.
fn production_part(path: &Path, text: &str) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name == "tests.rs"
        || name.ends_with("_tests.rs")
        || path.components().any(|c| c.as_os_str() == "tests")
    {
        return String::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut production = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "#[cfg(test)]" {
            let next = (i + 1..lines.len())
                .find(|&j| !lines[j].trim().is_empty() && !lines[j].trim().starts_with("#["));
            if let Some(j) = next {
                let declaration = lines[j].trim();
                if declaration.starts_with("mod ")
                    || declaration.starts_with("pub mod ")
                    || declaration.starts_with("pub(crate) mod ")
                {
                    if declaration.ends_with(';') || declaration.ends_with('}') {
                        i = j + 1;
                        continue;
                    }
                    let indent = lines[j].len() - lines[j].trim_start().len();
                    let end = (j + 1..lines.len()).find(|&k| {
                        lines[k].trim() == "}"
                            && lines[k].len() - lines[k].trim_start().len() == indent
                    });
                    i = end.expect("rustfmt-formatted test module has a closing brace") + 1;
                    continue;
                }
            }
        }
        production.push(lines[i]);
        i += 1;
    }
    production.join("\n")
}

/// A line that opens a greedy-diversity pass: `if <set>.insert(` on a set named for a failure
/// domain (`domain`, `household`, `hh`, `arch`), the shape of `placement.rs`'s retired pass and
/// of `peer_selection.rs`'s. Generic dedup sets (`seen.insert(..)`) are not matched.
fn opens_diversity_pass(line: &str) -> bool {
    let Some(at) = line.find(".insert(") else {
        return false;
    };
    let head = line[..at].trim_end();
    let Some(rest) = head.trim_start().strip_prefix("if ") else {
        return false;
    };
    let ident = rest.trim_start_matches('!').trim();
    let ident = ident.rsplit('.').next().unwrap_or(ident);
    if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    ["domain", "household", "hh", "arch"]
        .iter()
        .any(|word| ident.split('_').any(|part| part.starts_with(word)))
}

/// The one greedy-diversity pass this rail tolerates, with the condition that retires it. It is
/// the ingest selector's household/archetype/node greedy (interfaces spec §3.2, first row): not the
/// same algorithm as `placement_core::diversify` (it adds archetype and node passes and orders by
/// `peer_id`, not by XOR), so replacing it now would move ingest placements, which the brief
/// forbids. It retires when ingest expresses needs to the floor.
const TOLERATED_DIVERSITY: &[(&str, &str)] = &[(
    "src/services/peer_selection.rs",
    "ingest expresses needs to the floor (spec §3.2); then rank + diversify + NotCarriedReason",
)];

#[test]
fn one_ranking_home() {
    let root = crate_root();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    assert!(files.len() > 100, "the source scan found too few files");
    let shim = root.join("src/reconcile/placement.rs");
    // This file names the needles it looks for; it is itself a test module (`#[cfg(test)] mod
    // boundary;` in lib.rs), so it is not production code.
    let this_file = root.join("src/boundary.rs");
    let mut found_shim_impls = 0;
    for path in files.iter().filter(|p| **p != this_file) {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path).unwrap();
        let prod = production_part(path, &text);
        for needle in [
            concat!("impl ", "PlacementStrategy for"),
            concat!("impl ", "Ranking for"),
            concat!("impl ", "placement_core::Ranking for"),
        ] {
            let count = prod.matches(needle).count();
            if *path == shim {
                if needle.contains("PlacementStrategy") {
                    found_shim_impls += count;
                } else {
                    assert_eq!(
                        count, 0,
                        "{rel}: the shim implements no Ranking; rankings live in placement-core (R-P8)"
                    );
                }
            } else {
                assert_eq!(
                    count, 0,
                    "{rel}: `{needle}` outside the shim in reconcile/placement.rs — a placement \
                     ranking belongs in elohim-placement-core as a Ranking with its own MethodId (R-P8)"
                );
            }
        }
        let governed = rel.starts_with("src/services/") || rel.starts_with("src/reconcile/");
        if governed {
            let tolerated = TOLERATED_DIVERSITY.iter().any(|(f, _)| *f == rel);
            for (n, line) in prod.lines().enumerate() {
                if opens_diversity_pass(line) && !tolerated {
                    panic!(
                        "{rel}:{}: a greedy-diversity pass outside placement-core — use \
                         placement_core::diversify over a Ranking's order (R-P8): {}",
                        n + 1,
                        line.trim()
                    );
                }
            }
        }
    }
    assert_eq!(
        found_shim_impls, 2,
        "the shim implements PlacementStrategy for exactly XorDistanceStrategy and \
         DiversityAwarePlacementStrategy"
    );
    // A tolerated file that no longer has the pass must leave the list (the list only shrinks).
    for (file, retire_when) in TOLERATED_DIVERSITY {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        let prod = production_part(Path::new(file), &text);
        assert!(
            prod.lines().any(opens_diversity_pass),
            "{file} no longer holds a greedy-diversity pass; remove it from TOLERATED_DIVERSITY \
             (it was tolerated until: {retire_when})"
        );
    }
}

#[test]
fn the_diversity_detector_sees_the_retired_shapes_and_skips_dedup() {
    for line in [
        "                if used_domains.insert(scored[i].2.as_str()) {",
        "            if seen_hh.insert(hh) {",
        "                if seen_arch.insert(arch.clone()) {",
        "        if !used_households.insert(h) {",
        "        if self.seen_hh.insert(x) {",
    ] {
        assert!(opens_diversity_pass(line), "missed: {line}");
    }
    for line in [
        "        if seen.insert(hash) {",
        "        if !seen.insert(peer_id.clone()) {",
        "            household_by_agent.insert(key.clone(), household_id.clone());",
        "                snapshot.queried_households.insert(cid.clone());",
    ] {
        assert!(!opens_diversity_pass(line), "false positive: {line}");
    }
}

#[test]
fn the_test_module_cut_keeps_production_code_only() {
    let text = "fn a() {}\n#[cfg(test)]\nfn helper() {}\nfn b() {}\n#[cfg(test)]\n#[allow(x)]\nmod tests {\n impl Ranking for X {}\n}\nfn after_tests() {}\n#[cfg(test)]\nmod more_tests {\n impl Ranking for Y {}\n}\nfn after_more_tests() {}\n";
    let prod = production_part(Path::new("src/x.rs"), text);
    assert!(
        prod.contains("fn b()"),
        "a cfg(test) item is not a test module"
    );
    assert!(!prod.contains("impl Ranking"), "test modules are omitted");
    assert!(prod.contains("fn after_tests()"));
    assert!(prod.contains("fn after_more_tests()"));
    assert!(production_part(Path::new("src/a/b_tests.rs"), "impl Ranking for X {}").is_empty());
}

/// Every package any dependency table of a Cargo manifest names, normal, dev, build and
/// target-specific, by the package cargo resolves (a `package = ` rename included), with the
/// table kind (`dependencies`, `dev-dependencies`, `build-dependencies`).
fn manifest_dependencies(manifest: &str) -> Vec<(String, String)> {
    let doc: toml::Table = manifest.parse().expect("Cargo.toml parses");
    let mut out = Vec::new();
    let mut tables: Vec<(String, &toml::Table)> = Vec::new();
    for kind in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "dev_dependencies",
        "build_dependencies",
    ] {
        if let Some(t) = doc.get(kind).and_then(|v| v.as_table()) {
            tables.push((kind.to_string(), t));
        }
    }
    if let Some(targets) = doc.get("target").and_then(|v| v.as_table()) {
        for target in targets.values().filter_map(|v| v.as_table()) {
            for kind in [
                "dependencies",
                "dev-dependencies",
                "build-dependencies",
                "dev_dependencies",
                "build_dependencies",
            ] {
                if let Some(t) = target.get(kind).and_then(|v| v.as_table()) {
                    tables.push((kind.to_string(), t));
                }
            }
        }
    }
    for (kind, table) in tables {
        for (key, value) in table {
            let package = value
                .as_table()
                .and_then(|t| t.get("package"))
                .and_then(|p| p.as_str())
                .unwrap_or(key);
            out.push((kind.replace('_', "-"), package.to_string()));
        }
    }
    out
}

#[test]
fn eprfs_direction() {
    let manifest = std::fs::read_to_string(crate_root().join("Cargo.toml")).unwrap();
    let deps = manifest_dependencies(&manifest);
    assert!(
        deps.len() > 20,
        "the manifest reader found too few dependencies"
    );
    for denied in ["eprfs-storage", "eprfs-local", "eprfs-agent", "eprfs-host"] {
        assert!(
            !deps.iter().any(|(_, p)| p == denied),
            "elohim-storage depends on {denied}: it may take eprfs-core and eprfs-serve only (rail R-E3)"
        );
    }
    assert!(
        deps.iter()
            .any(|(k, p)| k == "dependencies" && p == "eprfs-core"),
        "the blob-store tier implements eprfs-core's EprfsStorage"
    );
}

#[test]
fn the_manifest_reader_sees_every_form() {
    for manifest in [
        "[dependencies]\nstore = { package = \"eprfs-storage\", path = \"x\" }\n",
        "[dependencies.eprfs-storage]\npath = \"x\"\n",
        "[target.'cfg(unix)'.dependencies]\neprfs-storage = \"1\"\n",
        "[target.'cfg(unix)'.dev-dependencies.eprfs-storage]\nversion = \"1\"\n",
        "[build-dependencies]\nstore = { package = \"eprfs-storage\", version = \"1\" }\n",
        "[dev_dependencies]\nstore = { package = \"eprfs-storage\", version = \"1\" }\n",
        "[target.'cfg(unix)'.build_dependencies.eprfs-storage]\nversion = \"1\"\n",
    ] {
        assert!(
            manifest_dependencies(manifest)
                .iter()
                .any(|(_, p)| p == "eprfs-storage"),
            "the R-E3 reader missed eprfs-storage in:\n{manifest}"
        );
    }
}
