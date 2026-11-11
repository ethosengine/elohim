/// Three section headers: null, section names and .comment. No binary fixture is stored.
pub fn artifact_elf(comment: &[u8]) -> Vec<u8> {
    artifact_elf_format(comment, true, true)
}

pub fn artifact_elf_format(comment: &[u8], is64: bool, little: bool) -> Vec<u8> {
    let names = b"\0.shstrtab\0.comment\0";
    let header_size = if is64 { 64 } else { 52 };
    let section_size = if is64 { 64 } else { 40 };
    let data = header_size + 3 * section_size;
    let mut bytes = vec![0u8; data];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[4] = if is64 { 2 } else { 1 };
    bytes[5] = if little { 1 } else { 2 };
    let mut write = |offset: usize, width: usize, value: u64| {
        let encoded = if little {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        let start = if little { 0 } else { 8 - width };
        bytes[offset..offset + width].copy_from_slice(&encoded[start..start + width]);
    };
    write(16, 2, 2);
    write(18, 2, if is64 { 62 } else { 3 });
    write(20, 4, 1);
    write(
        if is64 { 40 } else { 32 },
        if is64 { 8 } else { 4 },
        header_size as u64,
    );
    let sizes = if is64 { 52 } else { 40 };
    write(sizes, 2, header_size as u64);
    write(sizes + 6, 2, section_size as u64);
    write(sizes + 8, 2, 3);
    write(sizes + 10, 2, 1);
    for (header, name, kind, offset, size) in [
        (
            header_size + section_size,
            1,
            3,
            data as u64,
            names.len() as u64,
        ),
        (
            header_size + 2 * section_size,
            11,
            1,
            (data + names.len()) as u64,
            comment.len() as u64,
        ),
    ] {
        write(header, 4, name);
        write(header + 4, 4, kind);
        write(
            header + if is64 { 24 } else { 16 },
            if is64 { 8 } else { 4 },
            offset,
        );
        write(
            header + if is64 { 32 } else { 20 },
            if is64 { 8 } else { 4 },
            size,
        );
    }
    bytes.extend_from_slice(names);
    bytes.extend_from_slice(comment);
    bytes
}
