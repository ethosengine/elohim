//! SURFACE — which files the semantic fold covers (governed-discovery station 4, task 4.3).
//!
//! The measure owns its surface: `surfaces.paths` in `recall-semantic-index.json` are
//! gitignore-style globs over repository-relative paths. A file is admitted when an include
//! matches and no `!` negation does — negations always win, and nothing re-includes. `**` spans
//! directories, `*` and `?` stay within one path component (`require_literal_separator`), and a
//! leading dot needs no literal match (`**/*.md` reaches `.claude/…`). A trailing `/**` means
//! everything beneath.
//!
//! Two refusals sit on top of any declaration: the contract's `discovery.exclude_directories`
//! (a named directory anywhere in a path), and [`PRIVATE_CHAIN`] — a recall session's private
//! store and any sibling checkout — which no contract can admit, because it is a floor, not a
//! setting.
//!
//! **The candidates are the declared source.** On a git checkout they are `git ls-files -z`
//! (bounded by the fold's own listing budget, `fold_listing_bytes`, and `scan_seconds` — task
//! 4.6 moved the listing off the discovery traversal's `scan_bytes`, which a growing tree's
//! listing would otherwise exhaust, attesting the fold `failed`): ignored and
//! untracked derived state is not source. Elsewhere, a walk. Either way a candidate that exists
//! but cannot be read, and a directory that cannot be listed, is HELD — counted, its prior rows
//! kept current, never demoted as if it were gone.
use std::os::unix::fs::MetadataExt;

use super::discovery::atom_search_excluded;
use super::*;
use glob::{MatchOptions, Pattern};

/// The private chain's floor — refused whatever the contract excludes.
pub const PRIVATE_CHAIN: [&str; 2] = [".eprfs/status/recall/**", "**/worktrees/**"];

const MATCH: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// Where the candidates came from.
pub const GIT_TRACKED: &str = "git-tracked";
pub const WALK: &str = "walk";

fn compile(pattern: &str) -> Result<Pattern, String> {
    // A trailing `**` means everything beneath; the glob crate spells a file under it `**/*`.
    let spelled = if pattern == "**" {
        "**/*".to_string()
    } else if let Some(stem) = pattern.strip_suffix("/**") {
        format!("{stem}/**/*")
    } else {
        pattern.to_string()
    };
    Pattern::new(&spelled).map_err(|error| format!("surface pattern `{pattern}`: {error}"))
}

/// The compiled surface: includes (kept with their declared text — each is a head root),
/// negations, the private-chain floor and the contract's excluded directory names.
pub struct Surface {
    includes: Vec<(String, Pattern)>,
    negations: Vec<Pattern>,
    private: Vec<Pattern>,
    excluded: BTreeSet<String>,
}

impl Surface {
    pub fn declared(paths: &[String], contract: &Contract) -> Result<Self, String> {
        let mut includes = Vec::new();
        let mut negations = Vec::new();
        for path in paths {
            match path.strip_prefix('!') {
                Some(negated) => negations.push(compile(negated)?),
                None => includes.push((path.clone(), compile(path)?)),
            }
        }
        if includes.is_empty() {
            return Err("the surface declares no include pattern".into());
        }
        Ok(Self {
            includes,
            negations,
            private: PRIVATE_CHAIN
                .iter()
                .map(|p| compile(p))
                .collect::<Result<_, _>>()?,
            excluded: atom_search_excluded(contract),
        })
    }

    /// Refused whatever the declaration says: an excluded directory anywhere, or the private chain.
    fn refused(&self, rel: &str) -> bool {
        rel.split('/').any(|part| self.excluded.contains(part))
            || self.private.iter().any(|p| p.matches_with(rel, MATCH))
    }

    pub fn admits(&self, rel: &str) -> bool {
        !self.refused(rel)
            && self
                .includes
                .iter()
                .any(|(_, p)| p.matches_with(rel, MATCH))
            && !self.negations.iter().any(|p| p.matches_with(rel, MATCH))
    }

    /// The declared include patterns, in order — one head root each.
    pub fn includes(&self) -> impl Iterator<Item = (&str, &Pattern)> {
        self.includes.iter().map(|(text, p)| (text.as_str(), p))
    }

    pub fn include_matches(pattern: &Pattern, rel: &str) -> bool {
        pattern.matches_with(rel, MATCH)
    }
}

/// One admitted file, with the stat the plan compares against the store's.
pub struct Candidate {
    pub rel: String,
    pub size: u64,
    pub mtime_ns: i64,
}

pub fn stat_of(meta: &std::fs::Metadata) -> (u64, i64) {
    (meta.len(), meta.mtime() * 1_000_000_000 + meta.mtime_nsec())
}

/// Every admitted candidate, and everything that could not be seen.
#[derive(Default)]
pub struct Listing {
    pub files: Vec<Candidate>,
    /// Admitted paths that exist but are not readable regular files: their rows stay current.
    pub held: BTreeSet<String>,
    /// Directories the walk could not list: rows beneath them stay current.
    pub held_dirs: Vec<String>,
    /// Unlistable directories and undecodable admitted names — counted, never skipped.
    pub errors: usize,
}

impl Listing {
    /// Is a stored path one the listing could not see (rather than one that is gone)?
    pub fn holds(&self, rel: &str) -> bool {
        self.held.contains(rel)
            || self
                .held_dirs
                .iter()
                .any(|dir| dir.is_empty() || rel.starts_with(&format!("{dir}/")))
    }

    fn consider(&mut self, root: &Path, surface: &Surface, rel: &str) {
        if !surface.admits(rel) {
            return;
        }
        match std::fs::symlink_metadata(root.join(rel)) {
            Ok(meta) if meta.file_type().is_file() => {
                let (size, mtime_ns) = stat_of(&meta);
                self.files.push(Candidate {
                    rel: rel.to_string(),
                    size,
                    mtime_ns,
                });
            }
            // A symlink is not followed: it is not a file of this tree.
            Ok(meta) if meta.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => {
                self.held.insert(rel.to_string());
            }
        }
    }
}

/// `git-tracked` when the root is a git checkout, else `walk`.
pub fn source_of(root: &Path) -> &'static str {
    if root.join(".git").exists() {
        GIT_TRACKED
    } else {
        WALK
    }
}

/// The admitted candidates under `root`, sorted. `Err` names why the source could not be listed
/// at all — a fold then attests `failed` rather than demoting everything it cannot see.
pub fn list(root: &Path, contract: &Contract, surface: &Surface) -> Result<Listing, String> {
    let mut listing = if source_of(root) == GIT_TRACKED {
        git_tracked(root, contract, surface)?
    } else {
        walk(root, surface)
    };
    listing.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(listing)
}

fn git_tracked(root: &Path, contract: &Contract, surface: &Surface) -> Result<Listing, String> {
    // A missing or non-positive budget is a named refusal, never a zero-byte listing that would
    // read as "the source is empty".
    let listing_bytes = contract
        .positive_limit("fold_listing_bytes")
        .map_err(|error| error.to_string())? as usize;
    let args = vec![
        "-C".to_string(),
        root.to_string_lossy().into_owned(),
        "ls-files".to_string(),
        "-z".to_string(),
    ];
    let outcome = bounded_process(
        "git",
        &args,
        listing_bytes,
        contract.limit_secs("scan_seconds"),
        None,
    )
    .map_err(|error| format!("git ls-files cannot run: {error}"))?;
    if let Some(error) = outcome.error {
        return Err(format!("git ls-files: {error}"));
    }
    if outcome.status != Some(0) {
        let said = String::from_utf8_lossy(&outcome.stderr);
        return Err(format!(
            "git ls-files failed: {}",
            said.lines().last().unwrap_or_default().trim()
        ));
    }
    let mut listing = Listing::default();
    for raw in outcome.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        match std::str::from_utf8(raw) {
            Ok(rel) => listing.consider(root, surface, rel),
            Err(_) if surface.admits(&String::from_utf8_lossy(raw)) => listing.errors += 1,
            Err(_) => {}
        }
    }
    Ok(listing)
}

fn walk(root: &Path, surface: &Surface) -> Listing {
    let mut listing = Listing::default();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            listing.errors += 1;
            listing.held_dirs.push(rel);
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                listing.errors += 1;
                listing.held_dirs.push(rel.clone());
                continue;
            };
            let raw = entry.file_name();
            let name = raw.to_string_lossy();
            let child = if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            };
            let Ok(kind) = entry.file_type() else {
                if surface.admits(&child) {
                    listing.held.insert(child);
                }
                continue;
            };
            if kind.is_dir() {
                // A probe beneath the directory: one the floor or the contract refuses is never
                // entered at all (a sibling checkout is a whole second tree).
                if !surface.refused(&format!("{child}/-")) {
                    stack.push((entry.path(), child));
                }
            } else if kind.is_file() {
                if raw.to_str().is_none() {
                    if surface.admits(&child) {
                        listing.errors += 1;
                    }
                    continue;
                }
                listing.consider(root, surface, &child);
            }
        }
    }
    listing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(paths: &[&str], excluded: &[&str]) -> Surface {
        let paths: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        Surface {
            includes: paths
                .iter()
                .filter(|p| !p.starts_with('!'))
                .map(|p| (p.clone(), compile(p).unwrap()))
                .collect(),
            negations: paths
                .iter()
                .filter_map(|p| p.strip_prefix('!'))
                .map(|p| compile(p).unwrap())
                .collect(),
            private: PRIVATE_CHAIN.iter().map(|p| compile(p).unwrap()).collect(),
            excluded: excluded.iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn includes_span_directories_and_negations_always_win() {
        let s = surface(
            &[
                "CLAUDE.md",
                "**/*.md",
                ".epr-meta/**/*.json",
                "!genesis/data/**",
                "!**/generated/**",
            ],
            &[],
        );
        assert!(s.admits("CLAUDE.md"));
        assert!(s.admits("README.md"), "`**/` matches zero directories");
        assert!(
            s.admits(".claude/memory/note.md"),
            "a leading dot needs no literal"
        );
        assert!(s.admits(".epr-meta/x.json"));
        assert!(s.admits(".epr-meta/elohim/algorithms/recall-contract.json"));
        assert!(
            !s.admits("genesis/x.json"),
            "`*.json` is only declared under .epr-meta"
        );
        assert!(
            !s.admits("genesis/data/a/b.md"),
            "a trailing /** negates everything beneath"
        );
        assert!(!s.admits("elohim/sdk/generated/types.md"));
        assert!(
            s.admits("genesis/database.md"),
            "`data/**` is not a prefix match"
        );
    }

    #[test]
    fn a_single_star_stays_within_one_component() {
        let s = surface(&["genesis/*.md"], &[]);
        assert!(s.admits("genesis/a.md"));
        assert!(!s.admits("genesis/docs/a.md"));
    }

    #[test]
    fn the_private_chain_is_refused_whatever_the_contract_excludes() {
        let s = surface(&["**/*.md", "**/*.json"], &[]);
        assert!(!s.admits(".eprfs/status/recall/s1/receipt.md"));
        assert!(!s.admits(".claude/worktrees/agent-a/CLAUDE.md"));
        assert!(!s.admits("app/worktrees/x/y.md"));
        assert!(
            s.admits(".eprfs/other/x.md"),
            "only the recall store is the private chain"
        );
        assert!(
            s.refused(".claude/worktrees/agent-a/-"),
            "a worktree is never entered"
        );
    }

    #[test]
    fn an_excluded_directory_anywhere_refuses_the_path() {
        let s = surface(&["**/*.md"], &["node_modules"]);
        assert!(!s.admits("app/node_modules/pkg/README.md"));
        assert!(s.admits("app/README.md"));
    }
}
