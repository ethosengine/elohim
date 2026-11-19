//! The members of a static archive, read without any dependency (L2a re-review, ruling 2): put
//! reconciles each member of an archive a build script left in OUT_DIR with a judged compile,
//! because cc's `Build::object` chains a prebuilt object from anywhere into the archive beside
//! the compiled ones (cc-1.11.1 lib.rs:818 and :3852).
//!
//! The format is the System V / GNU one cc's `ar` writes on Linux: the `!<arch>\n` magic, then
//! per member a 60-byte header (name 16, mtime 12, uid 6, gid 6, mode 8, size 10, `` `\n ``)
//! and its data padded to an even length. Names are `name/`, or `/<offset>` into the GNU `//`
//! long-name table (entries ended by `/\n`); `/` and `/SYM64/` are the symbol tables. Anything
//! else this reader does not know (a thin archive, a BSD `#1/<len>` name, a bad header) is an
//! error by name, never an archive with no members.

/// One member: its name and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Member {
    pub name: String,
    pub data: Vec<u8>,
}

const MAGIC: &[u8] = b"!<arch>\n";
const THIN: &[u8] = b"!<thin>\n";
const HEADER: usize = 60;

/// The members of the archive `bytes`, symbol tables and the long-name table excluded.
pub(super) fn members(bytes: &[u8]) -> Result<Vec<Member>, String> {
    if bytes.starts_with(THIN) {
        return Err("a thin archive (!<thin>), whose members live outside it".into());
    }
    if !bytes.starts_with(MAGIC) {
        return Err("not an ar archive (no !<arch> magic)".into());
    }
    let mut at = MAGIC.len();
    let mut long_names: Option<&[u8]> = None;
    let mut found = Vec::new();
    while at < bytes.len() {
        let header = bytes
            .get(at..at + HEADER)
            .ok_or_else(|| format!("a truncated member header at byte {at}"))?;
        if &header[58..60] != b"`\n" {
            return Err(format!(
                "a member header at byte {at} without its `\\n terminator"
            ));
        }
        let field = |range: std::ops::Range<usize>| {
            std::str::from_utf8(&header[range])
                .map(str::trim_end)
                .map_err(|_| format!("a member header at byte {at} that is not text"))
        };
        let raw_name = field(0..16)?;
        let size: usize = field(48..58)?
            .parse()
            .map_err(|_| format!("a member at byte {at} with an unreadable size"))?;
        let start = at + HEADER;
        let data = bytes
            .get(start..start + size)
            .ok_or_else(|| format!("a member at byte {at} that runs past the archive's end"))?;
        at = start + size + (size % 2);
        match raw_name {
            "/" | "/SYM64/" => continue,
            "//" => {
                long_names = Some(data);
                continue;
            }
            _ => {}
        }
        let name = if let Some(offset) = raw_name.strip_prefix('/') {
            let offset: usize = offset
                .parse()
                .map_err(|_| format!("a member name {raw_name:?} this reader does not know"))?;
            let table = long_names.ok_or_else(|| {
                format!("a long member name {raw_name:?} before any // long-name table")
            })?;
            let tail = table
                .get(offset..)
                .ok_or_else(|| format!("a long member name {raw_name:?} past the name table"))?;
            let end = tail
                .windows(2)
                .position(|w| w == b"/\n")
                .ok_or_else(|| format!("a long member name {raw_name:?} with no /\\n end"))?;
            String::from_utf8(tail[..end].to_vec())
                .map_err(|_| format!("a long member name {raw_name:?} that is not UTF-8"))?
        } else if raw_name.starts_with("#1/") {
            return Err(format!(
                "a BSD member name {raw_name:?}, a format this reader does not read"
            ));
        } else if let Some(name) = raw_name.strip_suffix('/') {
            name.to_owned()
        } else {
            return Err(format!(
                "a member name {raw_name:?} this reader does not know"
            ));
        };
        found.push(Member {
            name,
            data: data.to_vec(),
        });
    }
    Ok(found)
}

#[cfg(test)]
#[path = "isa_c_archive_tests.rs"]
mod tests;
