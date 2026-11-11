//! Parser/IO budgets, not governance watermarks: refuse before growing a collection or copying
//! bytes. Measured experiment: 1,553 files, 17,575,280 bytes, largest 2,344,040, depth 6.
//! Caps leave >20x count, >27x largest-file and >30x payload headroom for glibc + headers.
use super::{ArtifactError, ArtifactResult};
use std::io::{Read, Write};
use std::path::Path;

pub const MAX_MEMBERS: u64 = 32_768;
pub const MAX_DEPTH: u64 = 64;
pub const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
/// glibc/gcc scripts on the build device use at most four operands including OUTPUT_FORMAT.
/// Count duplicates too, before resolution; 8,192 admits the 5,000-target budget regression.
pub const MAX_SCRIPT_OPERANDS: u64 = 8_192;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// Magic + manifest length + bounded manifest + one length per member + bounded payload.
pub const MAX_ARCHIVE_BYTES: u64 = 16 + MAX_MANIFEST_BYTES + 8 * MAX_MEMBERS + MAX_TOTAL_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Limit {
    #[error("sysroot: expanded member count limit exceeded")]
    Members,
    #[error("sysroot: per-script operand count limit exceeded")]
    ScriptOperands,
    #[error("sysroot: directory depth limit exceeded")]
    Depth,
    #[error("sysroot: manifest byte limit exceeded")]
    ManifestBytes,
    #[error("sysroot: per-file byte limit exceeded")]
    FileBytes,
    #[error("sysroot: cumulative byte limit exceeded")]
    TotalBytes,
    #[error("sysroot: archive read byte limit exceeded")]
    ArchiveBytes,
}

impl Limit {
    pub(super) fn maximum(self) -> u64 {
        match self {
            Self::Members => MAX_MEMBERS,
            Self::ScriptOperands => MAX_SCRIPT_OPERANDS,
            Self::Depth => MAX_DEPTH,
            Self::ManifestBytes => MAX_MANIFEST_BYTES,
            Self::FileBytes => MAX_FILE_BYTES,
            Self::TotalBytes => MAX_TOTAL_BYTES,
            Self::ArchiveBytes => MAX_ARCHIVE_BYTES,
        }
    }

    pub(super) fn check(self, value: u64) -> ArtifactResult<()> {
        if value > self.maximum() {
            Err(ArtifactError::SysrootLimit(self))
        } else {
            Ok(())
        }
    }
}

/// One counter shared by every manifest entry and evidence producer. Charge before retaining
/// strings, including repeated paths and duplicate script resolutions. The encoder separately
/// bounds JSON framing/escaping; neither check replaces the other.
#[derive(Default)]
pub(super) struct EvidenceBudget {
    pub bytes: u64,
}

impl EvidenceBudget {
    pub fn retain(&mut self, text: &str) -> ArtifactResult<String> {
        self.charge(text.len())?;
        Ok(text.to_owned())
    }

    pub fn charge(&mut self, len: usize) -> ArtifactResult<()> {
        let next = self.bytes.saturating_add(len as u64);
        Limit::ManifestBytes.check(next)?;
        self.bytes = next;
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Budget {
    pub members: u64,
    pub bytes: u64,
}

impl Budget {
    pub fn member(&mut self, path: &str) -> ArtifactResult<()> {
        Limit::Depth.check(path.split('/').count() as u64)?;
        Limit::Members.check(self.members + 1)?;
        self.members += 1;
        Ok(())
    }

    pub fn file(&mut self, len: u64) -> ArtifactResult<()> {
        Limit::FileBytes.check(len)?;
        Limit::TotalBytes.check(self.bytes.saturating_add(len))?;
        self.bytes += len;
        Ok(())
    }
}

/// Read incrementally, checking before every Vec growth, including a file that grows after stat.
fn read(mut reader: impl Read, limits: &[(Limit, u64)]) -> ArtifactResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(bytes);
        }
        for &(limit, already) in limits {
            limit.check(
                already
                    .saturating_add(bytes.len() as u64)
                    .saturating_add(count as u64),
            )?;
        }
        let needed = bytes.len() + count;
        if needed > bytes.capacity() {
            let ceiling = limits
                .iter()
                .map(|(limit, already)| limit.maximum().saturating_sub(*already))
                .min()
                .unwrap_or(0);
            let capacity = (needed.next_power_of_two() as u64).min(ceiling) as usize;
            bytes
                .try_reserve_exact(capacity - bytes.len())
                .map_err(std::io::Error::other)?;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

pub(super) fn read_file(path: &Path, total: u64) -> ArtifactResult<Vec<u8>> {
    read_path(path, &[(Limit::FileBytes, 0), (Limit::TotalBytes, total)])
}

pub fn read_archive(path: &Path) -> ArtifactResult<Vec<u8>> {
    read_path(path, &[(Limit::ArchiveBytes, 0)])
}

fn read_path(path: &Path, limits: &[(Limit, u64)]) -> ArtifactResult<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    for &(limit, already) in limits {
        limit.check(already.saturating_add(len))?;
    }
    read(file, limits)
}

/// Count JSON bytes without allocating the encoded manifest or a JSON value tree.
pub(super) fn manifest_size(value: &impl serde::Serialize) -> ArtifactResult<()> {
    struct Counter(u64);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len() as u64);
            if self.0 > MAX_MANIFEST_BYTES {
                return Err(std::io::Error::other("manifest limit"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    let result = serde_json::to_writer(&mut counter, value);
    Limit::ManifestBytes.check(counter.0)?;
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growing_streams_are_checked_before_copying_each_byte() {
        // Exercise the read loop without relying on stat, at each byte budget's exact boundary.
        for limit in [
            Limit::ManifestBytes,
            Limit::FileBytes,
            Limit::TotalBytes,
            Limit::ArchiveBytes,
        ] {
            let remaining = [(limit, limit.maximum() - 1)];
            assert_eq!(read(&b"x"[..], &remaining).unwrap(), b"x");
            assert!(
                matches!(read(&b"xy"[..], &remaining), Err(ArtifactError::SysrootLimit(actual)) if actual == limit)
            );
        }
    }
}
