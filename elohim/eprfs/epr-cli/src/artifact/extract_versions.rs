//! Validate GNU version-need chains explicitly; iterator success alone is insufficient.
use std::collections::{BTreeMap, BTreeSet};

use object::elf;
use object::read::elf::SectionHeader;

use super::{layout::Budget, text, unrecorded, ArtifactResult, Elf};

/// Sonames glibc itself installs and versions with `GLIBC_x.y` (glibc's `shlib-versions` files,
/// 2.17 through 2.41, every Linux port). One glibc build supplies all of them together, so a
/// `GLIBC_x.y` need on any of them needs that glibc, whichever library carries it: after the
/// 2.34 merge `libpthread`, `libdl`, `librt` and `libutil` are stubs whose old version nodes
/// remain, and `libm`/`libmvec` gain new nodes on their own (libm's `GLIBC_2.38`, `GLIBC_2.39`).
/// `libcrypt.so.1` is left out on purpose: distributions ship it from libxcrypt, which exports
/// `GLIBC_2.2.5`-style compatibility nodes that say nothing about the installed glibc.
/// Non-glibc nodes (`GLIBCXX_`, `GCC_`, `OPENSSL_`, ...) are not read here at all.
const GLIBC_SONAMES: [&str; 13] = [
    "libc.so.6",
    "libm.so.6",
    "libmvec.so.1",
    "libpthread.so.0",
    "libdl.so.2",
    "librt.so.1",
    "libresolv.so.2",
    "libutil.so.1",
    "libanl.so.1",
    "libnsl.so.1",
    "libBrokenLocale.so.1",
    "libthread_db.so.1",
    "libc_malloc_debug.so.0",
];

/// glibc's dynamic loaders, by the names its ports give them: `ld-linux*.so.N` (x86-64, i386,
/// x32, aarch64, arm, riscv, loongarch, ...), `ld64.so.N` (powerpc64, s390x) and `ld.so.1`
/// (powerpc, mips, s390). The loader carries needs such as `GLIBC_2.3` for `__tls_get_addr`.
fn glibc_library(soname: &str) -> bool {
    GLIBC_SONAMES.contains(&soname)
        || (soname.starts_with("ld-linux") && soname.contains(".so."))
        || matches!(soname, "ld64.so.1" | "ld64.so.2" | "ld.so.1")
}

/// One `GLIBC_x.y` need: its numeric version and every glibc library that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GlibcNeed {
    pub components: Vec<u32>,
    pub libraries: BTreeSet<String>,
}

/// Every `GLIBC_x.y` version need on a glibc library ([`glibc_library`]), keyed by node name.
/// A non-numeric `GLIBC_*` node on such a library (`GLIBC_PRIVATE`, `GLIBC_ABI_DT_RELR`) is not a
/// version and no floor expresses it, so it refuses rather than being skipped.
pub(super) fn glibc_versions(
    file: &Elf<'_>,
    bytes: &[u8],
) -> ArtifactResult<BTreeMap<String, GlibcNeed>> {
    let e = file.endian();
    let table = file.elf_section_table();
    let mut versions = BTreeMap::new();
    let mut budget = Budget::new(bytes.len(), 16, "version records");
    for s in table
        .iter()
        .filter(|s| s.sh_type(e) == elf::SHT_GNU_VERNEED)
    {
        let data = s.data(e, bytes).map_err(unrecorded)?;
        let strings = table
            .strings(e, bytes, object::SectionIndex(s.sh_link(e) as usize))
            .map_err(unrecorded)?;
        let count = s.sh_info(e) as usize;
        if count == 0 && !data.is_empty() {
            return Err(unrecorded("version needs: zero declared count"));
        }
        let mut occupied = BTreeSet::new();
        let mut cursor = 0;
        for index in 0..count {
            budget.take(1)?;
            let need = record(data, cursor, &mut occupied)?;
            if u16::from_le_bytes([need[0], need[1]]) != 1 {
                return Err(unrecorded("version needs: vn_version must be 1"));
            }
            let aux_count = u16::from_le_bytes([need[2], need[3]]) as usize;
            if aux_count == 0 {
                return Err(unrecorded("version needs: empty auxiliary chain"));
            }
            let library = strings
                .get(word(need, 4))
                .map_err(|_| unrecorded("version needs: invalid library string"))?;
            // A soname that is not UTF-8 names no glibc library; its needs are not read.
            let soname = std::str::from_utf8(library).unwrap_or_default().to_owned();
            let glibc = glibc_library(&soname);
            let mut aux = advance(cursor, word(need, 8), data.len())?;
            let next = word(need, 12);
            for aux_index in 0..aux_count {
                budget.take(1)?;
                let entry = record(data, aux, &mut occupied)?;
                let name = text(
                    strings
                        .get(word(entry, 8))
                        .map_err(|_| unrecorded("version needs: invalid version string"))?,
                )?;
                if glibc {
                    if let Some(number) = name.strip_prefix("GLIBC_") {
                        let components = number
                            .split('.')
                            .map(str::parse::<u32>)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|_| {
                                unrecorded(format!(
                                    "unsupported {} version need {}",
                                    super::layout::name(&soname),
                                    super::layout::name(&name)
                                ))
                            })?;
                        if components.len() < 2 {
                            return Err(unrecorded("invalid GLIBC version"));
                        }
                        versions
                            .entry(name)
                            .or_insert_with(|| GlibcNeed {
                                components,
                                libraries: BTreeSet::new(),
                            })
                            .libraries
                            .insert(soname.clone());
                    }
                }
                let next_aux = word(entry, 12);
                if aux_index + 1 == aux_count {
                    if next_aux != 0 {
                        return Err(unrecorded("version needs: auxiliary chain exceeds vn_cnt"));
                    }
                } else {
                    aux = advance(aux, next_aux, data.len())?;
                }
            }
            if index + 1 == count {
                if next != 0 {
                    return Err(unrecorded("version needs: chain exceeds declared count"));
                }
            } else {
                let target = advance(cursor, next, data.len())?;
                cursor = target;
            }
        }
    }
    Ok(versions)
}

fn word(record: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        record[offset],
        record[offset + 1],
        record[offset + 2],
        record[offset + 3],
    ])
}

fn record<'a>(
    data: &'a [u8],
    offset: usize,
    occupied: &mut BTreeSet<usize>,
) -> ArtifactResult<&'a [u8]> {
    // Need and auxiliary chains may be interleaved or grouped separately (LLD).
    // Their offsets must still identify distinct, non-overlapping 16-byte records.
    if occupied
        .range(..=offset)
        .next_back()
        .is_some_and(|old| old.saturating_add(16) > offset)
        || occupied
            .range(offset..)
            .next()
            .is_some_and(|next| *next < offset.saturating_add(16))
    {
        return Err(unrecorded("version needs: overlapping records"));
    }
    occupied.insert(offset);
    data.get(offset..offset.saturating_add(16))
        .ok_or_else(|| unrecorded("version needs: record outside section"))
}

fn advance(offset: usize, delta: u32, length: usize) -> ArtifactResult<usize> {
    if delta < 16 {
        return Err(unrecorded("version needs: chain makes no forward progress"));
    }
    offset
        .checked_add(delta as usize)
        .filter(|next| next.checked_add(16).is_some_and(|end| end <= length))
        .ok_or_else(|| unrecorded("version needs: offset outside section"))
}
