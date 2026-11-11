//! Canonical spellings for the initial axes and version lines (spec §3.1).
//!
//! Axes and lines stay open strings: a new axis needs no change here. These constants
//! exist so that producers and observers spell the shipped ones identically. A
//! requirement written as `GLIBC` would otherwise never compare with a host that
//! observed `glibc`; the line mismatch is refused as `unmet`, never guessed.

/// Axis identifiers for the rows of spec §3.1 that have an observer or an extractor.
pub mod axis {
    /// Architecture family, compared with `Equal` (for example `x86_64`).
    pub const ARCH: &str = "arch";
    /// Operating-system family, compared with `Equal` (for example `linux`).
    pub const OS: &str = "os";
    /// x86 instruction-set extensions, compared with `Subset` in psABI spellings.
    pub const ISA_X86: &str = "isa.x86";
    /// Minimum Linux kernel ABI, compared with `AtLeast` on the [`super::line::LINUX`] line.
    pub const KERNEL_LINUX: &str = "kernel.linux";
    /// C library symbol floor, compared with `AtLeast` on a named libc line.
    pub const ABI_LIBC: &str = "abi.libc";
    /// Program interpreter paths, compared with `Present`.
    pub const ABI_LOADER: &str = "abi.loader";
    /// Shared libraries the artifact needs, compared with `Present`.
    pub const LIB: &str = "lib";
}

/// Version lines for `AtLeast` comparisons. Versions on different lines never compare.
pub mod line {
    /// GNU C library symbol versions.
    pub const GLIBC: &str = "glibc";
    /// musl C library versions.
    pub const MUSL: &str = "musl";
    /// Linux kernel ABI versions.
    pub const LINUX: &str = "linux";
}

#[cfg(test)]
mod tests {
    use super::{axis, line};

    #[test]
    fn shipped_bundles_use_the_canonical_isa_axis() {
        assert_eq!(crate::X86_64_V2.axis, axis::ISA_X86);
        assert_eq!(crate::X86_64_V3.axis, axis::ISA_X86);
    }

    #[test]
    fn spellings_are_lowercase_and_distinct() {
        let all = [
            axis::ARCH,
            axis::OS,
            axis::ISA_X86,
            axis::KERNEL_LINUX,
            axis::ABI_LIBC,
            axis::ABI_LOADER,
            axis::LIB,
            line::GLIBC,
            line::MUSL,
            line::LINUX,
        ];
        for name in all {
            assert_eq!(name, name.to_ascii_lowercase(), "{name} must be lowercase");
        }
        let unique: std::collections::BTreeSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "spellings must be distinct");
    }
}
