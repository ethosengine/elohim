//! Range-only .eh_frame reader for ELF64 x86-64. CFI instructions are not executed.
//! Supports DWARF32/64 CIE versions 1/3 and z/L/P/R/S augmentation. Unknown
//! encodings refuse, rather than silently omitting potential code. No unwind dependency.
use std::collections::BTreeMap;
use std::ops::Range;

use object::{Object, ObjectSection};

use super::{unrecorded, ArtifactResult, Elf};

#[derive(Clone, Copy)]
struct Cie {
    encoding: u8,
    augmented: bool,
}

pub(super) fn ranges(file: &Elf<'_>) -> ArtifactResult<Vec<Range<u64>>> {
    let Some(section) = file.section_by_name(".eh_frame") else {
        return Ok(Vec::new());
    };
    read(section.data().map_err(unrecorded)?, section.address())
        .map_err(|e| unrecorded(format!(".eh_frame: {e}")))
}

pub(super) fn read(data: &[u8], address: u64) -> ArtifactResult<Vec<Range<u64>>> {
    let mut reader = Reader {
        data,
        pos: 0,
        address,
    };
    let mut cies = BTreeMap::new();
    let mut ranges = Vec::new();
    while reader.pos < data.len() {
        let start = reader.pos;
        let length = reader.uint(4)?;
        if length == 0 {
            if data[reader.pos..].iter().any(|b| *b != 0) {
                return Err(unrecorded("nonzero bytes after .eh_frame terminator"));
            }
            break;
        }
        let length = if length == u64::from(u32::MAX) {
            reader.uint(8)?
        } else if length >= 0xfffffff0 {
            return Err(unrecorded("reserved .eh_frame record length"));
        } else {
            length
        };
        let id_offset = reader.pos;
        let record = reader.take(usize::try_from(length).map_err(unrecorded)?)?;
        let mut entry = Reader {
            data: record,
            pos: 0,
            address: address
                .checked_add(id_offset as u64)
                .ok_or_else(|| unrecorded("frame address overflow"))?,
        };
        // Unlike .debug_frame, .eh_frame CIE IDs/pointers remain 32-bit in DWARF64.
        let id = entry.uint(4)?;
        if id == 0 {
            cies.insert(start, read_cie(&mut entry)?);
        } else {
            let offset = id_offset
                .checked_sub(usize::try_from(id).map_err(unrecorded)?)
                .ok_or_else(|| unrecorded("invalid backward CIE pointer"))?;
            let cie = cies
                .get(&offset)
                .ok_or_else(|| unrecorded("FDE references missing CIE"))?;
            // Indirection changes the function address, so it cannot be ignored for an FDE.
            if cie.encoding & 0x80 != 0 {
                return Err(unrecorded("indirect FDE PC encoding unsupported"));
            }
            let pc = entry.pointer(cie.encoding)?;
            let size = entry.pointer(cie.encoding & 0x0f)?;
            if cie.augmented {
                let length = usize::try_from(entry.leb(false)?).map_err(unrecorded)?;
                entry.take(length)?;
            }
            if size != 0 {
                let end = pc
                    .checked_add(size)
                    .ok_or_else(|| unrecorded("FDE PC range overflow"))?;
                ranges.push(pc..end);
            }
        }
    }
    Ok(ranges)
}

fn read_cie(reader: &mut Reader<'_>) -> ArtifactResult<Cie> {
    let version = reader.uint(1)?;
    if !matches!(version, 1 | 3) {
        return Err(unrecorded(format!("unsupported CIE version {version}")));
    }
    let start = reader.pos;
    while reader.uint(1)? != 0 {}
    let augmentation = &reader.data[start..reader.pos - 1];
    reader.leb(false)?; // code alignment
    reader.leb(true)?; // data alignment
    if version == 1 {
        reader.uint(1)?;
    } else {
        reader.leb(false)?;
    }
    let mut cie = Cie {
        encoding: 0,
        augmented: false,
    };
    if augmentation.is_empty() {
        return Ok(cie);
    }
    if augmentation[0] != b'z' {
        return Err(unrecorded("unsupported non-z CIE augmentation"));
    }
    cie.augmented = true;
    let length = usize::try_from(reader.leb(false)?).map_err(unrecorded)?;
    let address = reader
        .address
        .checked_add(reader.pos as u64)
        .ok_or_else(|| unrecorded("CIE address overflow"))?;
    let data = reader.take(length)?;
    let mut aug = Reader {
        data,
        pos: 0,
        address,
    };
    for kind in &augmentation[1..] {
        match kind {
            b'L' => {
                aug.uint(1)?;
            } // LSDA is not needed to locate code.
            b'R' => cie.encoding = aug.uint(1)? as u8,
            b'P' => {
                let encoding = aug.uint(1)? as u8;
                // Only skip the personality value; never dereference an indirect pointer.
                aug.pointer(encoding & 0x7f)?;
            }
            b'S' => {}
            _ => {
                return Err(unrecorded(format!(
                    "unsupported CIE augmentation byte {kind}"
                )))
            }
        }
    }
    if aug.pos != aug.data.len() {
        return Err(unrecorded("unconsumed CIE augmentation bytes"));
    }
    Ok(cie)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    address: u64,
}

impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> ArtifactResult<&'a [u8]> {
        let end = self
            .pos
            .checked_add(size)
            .ok_or_else(|| unrecorded("frame offset overflow"))?;
        let data = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| unrecorded("truncated frame record"))?;
        self.pos = end;
        Ok(data)
    }

    fn uint(&mut self, size: usize) -> ArtifactResult<u64> {
        let mut value = [0; 8];
        value[..size].copy_from_slice(self.take(size)?);
        Ok(u64::from_le_bytes(value))
    }

    fn leb(&mut self, signed: bool) -> ArtifactResult<i128> {
        let mut value = 0i128;
        for shift in (0..70).step_by(7) {
            let byte = self.uint(1)? as u8;
            value |= i128::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                if signed && byte & 0x40 != 0 {
                    value |= !0i128 << (shift + 7);
                }
                let valid = if signed {
                    i64::try_from(value).is_ok()
                } else {
                    u64::try_from(value).is_ok()
                };
                return if valid {
                    Ok(value)
                } else {
                    Err(unrecorded("LEB128 overflow"))
                };
            }
        }
        Err(unrecorded("unterminated LEB128"))
    }

    fn pointer(&mut self, encoding: u8) -> ArtifactResult<u64> {
        let base = match encoding & 0xf0 {
            0 => 0,
            0x10 => self
                .address
                .checked_add(self.pos as u64)
                .ok_or_else(|| unrecorded("pointer base overflow"))?,
            _ => {
                return Err(unrecorded(format!(
                    "unsupported DW_EH_PE encoding 0x{encoding:02x}"
                )))
            }
        };
        let value = match encoding & 0x0f {
            0 => i128::from(self.uint(8)?),
            1 => self.leb(false)?,
            2 => i128::from(self.uint(2)?),
            3 => i128::from(self.uint(4)?),
            4 => i128::from(self.uint(8)?),
            9 => self.leb(true)?,
            0x0a => i128::from(self.uint(2)? as i16),
            0x0b => i128::from(self.uint(4)? as i32),
            0x0c => i128::from(self.uint(8)? as i64),
            _ => {
                return Err(unrecorded(format!(
                    "unsupported DW_EH_PE encoding 0x{encoding:02x}"
                )))
            }
        };
        u64::try_from(i128::from(base) + value)
            .map_err(|_| unrecorded("encoded pointer overflow or negative range"))
    }
}
