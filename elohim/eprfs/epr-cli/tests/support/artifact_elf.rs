/// Full compiler identity shared by the synthetic descriptor and its executable stub.
pub const RUSTC_IDENTITY: &str = "rustc fixture\nbinary: rustc\ncommit-hash: fixture-hash\ncommit-date: 2026-01-01\nhost: x86_64-unknown-linux-gnu\nrelease: 1.96.1\nLLVM version: 22.1.2\n";

/// Static Linux ELF: section names, .comment, baseline .text (ret), and a sized function.
pub fn artifact_elf(comment: &[u8]) -> Vec<u8> {
    artifact_elf_format(comment, true, true)
}

pub fn artifact_elf_format(comment: &[u8], is64: bool, little: bool) -> Vec<u8> {
    let names = b"\0.shstrtab\0.comment\0.text\0.symtab\0.strtab\0";
    let header_size = if is64 { 64 } else { 52 };
    let section_size = if is64 { 64 } else { 40 };
    let data = header_size + 6 * section_size;
    let mut bytes = vec![0u8; data];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[7] = 3; // ELFOSABI_LINUX; no ABI-tag version floor.
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
    write(sizes + 8, 2, 6);
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
        (
            header_size + 3 * section_size,
            20,
            1,
            (data + names.len() + comment.len()) as u64,
            1,
        ),
        (
            header_size + 4 * section_size,
            26,
            2,
            (data + names.len() + comment.len() + 1) as u64,
            if is64 { 48 } else { 32 },
        ),
        (
            header_size + 5 * section_size,
            34,
            3,
            (data + names.len() + comment.len() + 1 + if is64 { 48 } else { 32 }) as u64,
            9,
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
    write(
        header_size + 3 * section_size + 8,
        if is64 { 8 } else { 4 },
        6,
    );
    let sym_header = header_size + 4 * section_size;
    write(sym_header + if is64 { 40 } else { 24 }, 4, 5);
    write(sym_header + if is64 { 44 } else { 28 }, 4, 1);
    write(
        sym_header + if is64 { 56 } else { 36 },
        if is64 { 8 } else { 4 },
        if is64 { 24 } else { 16 },
    );
    bytes.extend_from_slice(names);
    bytes.extend_from_slice(comment);
    bytes.push(0xc3);
    let symbol_size = if is64 { 24 } else { 16 };
    bytes.resize(bytes.len() + symbol_size, 0);
    let mut symbol = vec![0; symbol_size];
    let mut field = |offset: usize, width: usize, value: u64| {
        let encoded = if little {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        let start = if little { 0 } else { 8 - width };
        symbol[offset..offset + width].copy_from_slice(&encoded[start..start + width]);
    };
    field(0, 4, 1);
    field(if is64 { 16 } else { 8 }, if is64 { 8 } else { 4 }, 1);
    field(if is64 { 6 } else { 14 }, 2, 3);
    symbol[if is64 { 4 } else { 12 }] = 0x12; // STB_GLOBAL | STT_FUNC
    bytes.extend(symbol);
    bytes.extend(b"\0fixture\0");
    if is64 && little {
        let phoff = bytes.len() as u64;
        bytes[32..40].copy_from_slice(&phoff.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        let mut load = [0u8; 56];
        load[..4].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
        load[4..8].copy_from_slice(&5u32.to_le_bytes()); // PF_R | PF_X
        load[8..16].copy_from_slice(&((data + names.len() + comment.len()) as u64).to_le_bytes());
        // .text and its sized symbol both start at virtual address zero.
        load[32..40].copy_from_slice(&1u64.to_le_bytes());
        load[40..48].copy_from_slice(&1u64.to_le_bytes());
        load[48..56].copy_from_slice(&1u64.to_le_bytes());
        bytes.extend(load);
    }
    bytes
}

/// Newly written executable fixtures can briefly return ETXTBSY on overlay-backed /tmp.
/// Wait only for that transient publication condition; other errors remain immediate failures.
pub fn wait_for_stub(path: &std::path::Path) {
    for _ in 0..100 {
        match std::process::Command::new(path).arg("--version").output() {
            Ok(output) => {
                assert!(output.status.success());
                return;
            }
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("{}: {error}", path.display()),
        }
    }
    panic!("{} stayed ETXTBSY", path.display());
}
