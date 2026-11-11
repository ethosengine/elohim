//! Validate GNU version-need chains explicitly; iterator success alone is insufficient.
use std::collections::{BTreeMap, BTreeSet};

use object::elf;
use object::read::elf::SectionHeader;

use super::{layout::Budget, text, unrecorded, ArtifactResult, Elf};

pub(super) fn glibc_versions(
    file: &Elf<'_>,
    bytes: &[u8],
) -> ArtifactResult<BTreeMap<String, Vec<u32>>> {
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
            let libc = strings
                .get(word(need, 4))
                .map_err(|_| unrecorded("version needs: invalid library string"))?
                == b"libc.so.6";
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
                if libc {
                    if let Some(number) = name.strip_prefix("GLIBC_") {
                        let components = number
                            .split('.')
                            .map(str::parse::<u32>)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|_| {
                                unrecorded(format!(
                                    "unsupported libc.so.6 version need {}",
                                    super::layout::name(&name)
                                ))
                            })?;
                        if components.len() < 2 {
                            return Err(unrecorded("invalid GLIBC version"));
                        }
                        versions.insert(name, components);
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
