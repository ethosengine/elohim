/// Three ELF64 section headers: null, section names and .comment. No binary fixture is stored.
pub fn artifact_elf(comment: &[u8]) -> Vec<u8> {
    let names = b"\0.shstrtab\0.comment\0";
    let mut bytes = vec![0u8; 256];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[40..48].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[58..60].copy_from_slice(&64u16.to_le_bytes());
    bytes[60..62].copy_from_slice(&3u16.to_le_bytes());
    bytes[62..64].copy_from_slice(&1u16.to_le_bytes());
    for (header, name, kind, offset, size) in [
        (128, 1u32, 3u32, 256u64, names.len() as u64),
        (
            192,
            11u32,
            1u32,
            256 + names.len() as u64,
            comment.len() as u64,
        ),
    ] {
        bytes[header..header + 4].copy_from_slice(&name.to_le_bytes());
        bytes[header + 4..header + 8].copy_from_slice(&kind.to_le_bytes());
        bytes[header + 24..header + 32].copy_from_slice(&offset.to_le_bytes());
        bytes[header + 32..header + 40].copy_from_slice(&size.to_le_bytes());
    }
    bytes.extend_from_slice(names);
    bytes.extend_from_slice(comment);
    bytes
}
