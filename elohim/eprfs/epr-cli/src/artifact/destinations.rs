//! Read-only destination admission. No probe files, directory creation or format claims.
use super::*;
use rustix::fs::{access, Access};

fn writable(path: &Path, directory: bool) -> ArtifactResult<()> {
    let mut existing = path;
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| unrecorded("destination has no existing parent"))?;
    }
    let meta = std::fs::metadata(existing)?;
    if (existing != path || directory) && !meta.is_dir() {
        return Err(unrecorded(format!(
            "destination {} is not a directory",
            existing.display()
        )));
    }
    if existing == path && !directory && !meta.is_file() {
        return Err(unrecorded(format!(
            "destination {} is not a regular file",
            existing.display()
        )));
    }
    let mode = if meta.is_dir() {
        Access::WRITE_OK | Access::EXEC_OK
    } else {
        Access::WRITE_OK
    };
    access(existing, mode).map_err(|e| {
        unrecorded(format!(
            "destination {} is not writable: {e}",
            existing.display()
        ))
    })?;
    Ok(())
}

pub(super) fn payload(path: &Path) -> ArtifactResult<()> {
    crate::payload::format_of(path).map_err(unrecorded)?;
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        use std::os::unix::fs::PermissionsExt;
        if meta.file_type().is_symlink() || meta.permissions().mode() & 0o077 != 0 {
            return Err(unrecorded(format!(
                "payload store {} must be a private directory, not a symlink",
                path.display()
            )));
        }
    }
    writable(path, true)?;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            writable(&path, path.is_dir())?;
        }
    }
    Ok(())
}

pub(super) fn flow(root: &Path) -> ArtifactResult<()> {
    let path = root.join(".eprfs/status/flows.jsonl");
    writable(&path, false)?;
    if path.exists() {
        let file = std::fs::File::open(&path)?;
        file.lock_shared()?;
        elohim_epr_rea::decode_flow_records(&std::fs::read_to_string(&path)?)
            .map_err(unrecorded)?;
    }
    Ok(())
}

// Brit's object store and notes live in the common Git directory, including in worktrees.
pub(super) fn brit(repo: &Path) -> ArtifactResult<()> {
    let common = git_ok(repo, &["rev-parse", "--git-common-dir"])?;
    let common = repo.join(common.trim());
    for path in [
        common.clone(),
        common.join("objects"),
        common.join("refs/notes"),
        common.join("brit"),
        common.join("brit/objects"),
        common.join("refs/notes/brit/build"),
    ] {
        writable(&path, true)?;
    }
    Ok(())
}
