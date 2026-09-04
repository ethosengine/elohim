//! Readers over the repository's GENERATED registers.
//!
//! `genesis/manifests/habits.yaml` is a projection that something else owns — it is written by
//! `.claude/scripts/habits-project.py` from the `.epr-meta` habit atoms. Nothing here ever
//! writes it. Reading a generated file and then editing it is how a projection acquires a
//! second author, and a register with two authors is no longer a register.
//!
//! This is the first Rust reader of that file. It deserializes only the fields a reader needs,
//! so a field added to the register does not break this one.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{FlowError, FlowResult};

/// The generated habit register, relative to the repository root.
pub const HABITS_REGISTER_REL: &str = "genesis/manifests/habits.yaml";

#[derive(Debug, Deserialize)]
struct HabitRegister {
    #[serde(default)]
    habits: Vec<HabitEntry>,
}

/// One habit as the register projects it. A habit is a SCOPE — the standard a piece of work is
/// accounted to — so the fields read here are the ones that say what the standard is
/// (`invariant`, `checks`), where it stands (`status`, `active`), and what names it (`refs`).
#[derive(Debug, Clone, Deserialize)]
pub struct HabitEntry {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub refs: Vec<String>,
}

/// Absolute path of the habit register under `root`.
pub fn habits_register_path(root: &Path) -> PathBuf {
    root.join(HABITS_REGISTER_REL)
}

/// Every habit the register declares.
///
/// A missing or unparseable register is an ERROR naming the file rather than an empty list: a
/// caller asking "is `x` a declared habit" must never be told "no" because the register could
/// not be read. That is the difference between an absent habit and an absent register.
pub fn read_habits(root: &Path) -> FlowResult<Vec<HabitEntry>> {
    let path = habits_register_path(root);
    let text = std::fs::read_to_string(&path).map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    let register: HabitRegister =
        serde_yaml::from_str(&text).map_err(|source| FlowError::Yaml { path, source })?;
    Ok(register.habits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A fixture register under a temp dir. The live repository is never read: a test that
    /// asserted against the real register would fail the day someone legitimately re-projected
    /// it.
    fn fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(HABITS_REGISTER_REL);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            r#"version: 1
habits:
  - id: dev-system-equilibrium
    status: red
    active: false
    checks:
      - "epr flow stocks --window <START..END> --per week --stock commitments --check"
    refs:
      - "elohim/eprfs/epr-cli/src/flow/stocks.rs"
  - id: epr-atom-home
    status: green
    active: true
    checks:
      - "a2o @concern:epr-atom-home"
    refs: []
"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn the_register_reads_as_data_and_an_unknown_id_is_simply_absent() {
        let dir = fixture();
        let habits = read_habits(dir.path()).expect("register reads");
        let ids: Vec<&str> = habits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["dev-system-equilibrium", "epr-atom-home"]);
        assert_eq!(habits[0].status, "red");
        assert!(!habits[0].active);
        assert!(habits[1].active);
        assert!(!ids.contains(&"not-a-habit"));
    }

    #[test]
    fn a_missing_register_is_an_error_that_names_the_file_never_an_empty_list() {
        let dir = TempDir::new().unwrap();
        let err = read_habits(dir.path()).expect_err("an unreadable register is not an empty one");
        assert!(
            err.to_string().contains(HABITS_REGISTER_REL),
            "the refusal must name the register file; got: {err}"
        );
    }
}
