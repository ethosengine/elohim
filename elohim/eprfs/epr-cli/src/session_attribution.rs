//! Read-only runtime metadata adapters for passive commit presentation.
//! Model metadata is a claim about execution, not actor identity or permission.

use std::path::Path;

use elohim_device_runtime::{envelope::Envelope, session::Registry};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

/// Read one exact thread's model from the locally observed Codex state-v5 schema.
pub fn codex_model(root: &Path, session: &str) -> Result<String, String> {
    if !root.is_absolute() {
        return Err("Codex model metadata root must be absolute".into());
    }
    let db = Connection::open_with_flags(
        root.join("state_5.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "Codex model metadata unavailable (state-v5)".to_string())?;
    db.busy_timeout(std::time::Duration::from_millis(100))
        .map_err(|e| e.to_string())?;
    // This adapter supports the indexed v5 key, never a scan across another session's data.
    let keys: Vec<String> = db
        .prepare("SELECT name FROM pragma_table_info('threads') WHERE pk > 0")
        .map_err(|_| "Codex model schema unavailable")?
        .query_map([], |row| row.get(0))
        .map_err(|_| "Codex model schema unavailable")?
        .collect::<Result<_, _>>()
        .map_err(|_| "Codex model schema unavailable")?;
    if keys != ["id"] {
        return Err("Codex model schema unsupported: expected threads.id primary key".into());
    }
    let model: Option<String> = db
        .query_row(
            "SELECT model FROM threads WHERE id = ?1",
            [session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Codex model metadata unreadable")?;
    model
        .filter(|s| !s.trim().is_empty())
        .ok_or("Codex thread model is unmeasured".into())
}

/// Select the declared live observation method, or retain mounted context for other runtimes.
pub fn decorate(record: &Envelope, registry: &Registry) -> Result<Vec<(String, String)>, String> {
    let mut context = record.clone();
    let source = registry
        .runtimes
        .iter()
        .find(|r| r.id == record.identity.runtime)
        .and_then(|r| r.model_source.as_deref());
    match source {
        Some("codex-state-v5") => {
            let home = std::env::var_os("CODEX_HOME")
                .map(std::path::PathBuf::from)
                .ok_or("Codex model metadata root is unmeasured: CODEX_HOME absent")?;
            context.model = Some(codex_model(&home, record.id())?);
        }
        Some(_) => return Err("runtime model metadata adapter unsupported".into()),
        None => {}
    }
    elohim_device_runtime::commit_attribution::trailers(&context, registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_metadata_is_exact_session_read_only_and_tracks_changes() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("state_5.sqlite")).unwrap();
        db.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, model TEXT); INSERT INTO threads VALUES ('ours', 'model-a'), ('other', 'private-other-model');").unwrap();
        let before = std::fs::read(dir.path().join("state_5.sqlite")).unwrap();
        assert_eq!(codex_model(dir.path(), "ours").unwrap(), "model-a");
        assert_eq!(
            std::fs::read(dir.path().join("state_5.sqlite")).unwrap(),
            before
        );
        assert!(codex_model(dir.path(), "missing").is_err());
        assert!(codex_model(dir.path(), "ours' OR 1=1 --").is_err());
        db.execute("UPDATE threads SET model='model-b' WHERE id='ours'", [])
            .unwrap();
        assert_eq!(codex_model(dir.path(), "ours").unwrap(), "model-b");
        db.execute("UPDATE threads SET model=NULL WHERE id='ours'", [])
            .unwrap();
        assert!(codex_model(dir.path(), "ours").is_err());
        let missing = tempfile::tempdir().unwrap();
        assert!(codex_model(missing.path(), "ours").is_err());
        assert!(!missing.path().join("state_5.sqlite").exists());
    }
}
