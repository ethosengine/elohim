//! Device-local host observation, beside the driver's enforcement Fingerprint.
//! Rebuild on envelope start or host change; never persist this inventory.

use host_contract::{
    names::{axis, line},
    AxisId, AxisValue, Provides, Version,
};

#[cfg(target_os = "linux")]
#[path = "host_libraries.rs"]
mod libraries;

const AXES: &[&str] = &[
    axis::ARCH,
    axis::OS,
    axis::ISA_X86,
    axis::ABI_LIBC,
    axis::ABI_LOADER,
    axis::KERNEL_LINUX,
    axis::LIB,
];

/// Observe the current process's host without reading any declaration or label.
/// Unsupported or failed probes remain explicitly unobserved.
pub fn observe_host() -> Provides {
    let mut provides = Provides::default();
    for name in AXES {
        provides.mark_unobserved(AxisId((*name).into()));
    }
    #[cfg(target_os = "linux")]
    {
        observe_linux(&mut provides);
        let arch = match provides.observed(&AxisId(axis::ARCH.into())) {
            Some(AxisValue::Scalar(arch)) => arch.as_str(),
            _ => "",
        };
        if let Some(names) = libraries::observe(
            libraries::default_dirs(arch),
            std::path::Path::new("/etc/ld.so.conf"),
        ) {
            provides.observe(AxisId(axis::LIB.into()), AxisValue::Names(names));
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    provides.observe(
        AxisId(axis::ISA_X86.into()),
        AxisValue::Names(super::x86::observe()),
    );
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // SAFETY: glibc returns a static, NUL-terminated string for the loaded libc.
        let version = unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) };
        if let Ok(text) = version.to_str() {
            if let Some(version) = parse_version(line::GLIBC, text) {
                provides.observe(AxisId(axis::ABI_LIBC.into()), AxisValue::Version(version));
            }
        }
    }
    provides
}

/// Local operator diagnostic only. No Serialize implementation or record writer.
pub fn print_provides(provides: &Provides) -> String {
    let mut text = String::new();
    for name in AXES {
        let value = match provides.observed(&AxisId((*name).into())) {
            Some(AxisValue::Scalar(value)) => value.clone(),
            Some(AxisValue::Names(names)) => names.iter().cloned().collect::<Vec<_>>().join(", "),
            Some(AxisValue::Version(version)) => format!(
                "{} {}",
                version.line,
                version
                    .components
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(".")
            ),
            None => "unobserved".into(),
        };
        text.push_str(&format!("{name}: {value}\n"));
    }
    text
}

fn parse_version(line: &str, release: &str) -> Option<Version> {
    let numeric = release
        .split_once('-')
        .map_or(release, |(numeric, _)| numeric);
    let components = numeric
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u32>, _>>()
        .ok()?;
    if components.len() < 2 {
        return None;
    }
    Some(Version {
        line: line.into(),
        components,
    })
}

#[cfg(target_os = "linux")]
fn observe_linux(provides: &mut Provides) {
    if let Ok(uts) = nix::sys::utsname::uname() {
        if let Some(machine) = uts.machine().to_str() {
            provides.observe(AxisId(axis::ARCH.into()), AxisValue::Scalar(machine.into()));
        }
        if uts.sysname() == "Linux" {
            provides.observe(AxisId(axis::OS.into()), AxisValue::Scalar("linux".into()));
        }
        if let Some(version) = uts
            .release()
            .to_str()
            .and_then(|v| parse_version(line::LINUX, v))
        {
            provides.observe(
                AxisId(axis::KERNEL_LINUX.into()),
                AxisValue::Version(version),
            );
        }
    }
    let mut loaders = std::collections::BTreeSet::new();
    for path in [
        "/lib64/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
        "/lib/ld-linux.so.2",
        "/lib32/ld-linux.so.2",
        "/lib/i386-linux-gnu/ld-linux.so.2",
        "/libx32/ld-linux-x32.so.2",
        "/lib/x86_64-linux-gnux32/ld-linux-x32.so.2",
        "/lib/ld-linux-aarch64.so.1",
        "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1",
        "/lib/ld-linux-aarch64_be.so.1",
        "/lib/ld-linux-riscv64-lp64.so.1",
        "/lib/ld-linux-riscv64-lp64f.so.1",
        "/lib/ld-linux-riscv64-lp64d.so.1",
        "/lib/riscv64-linux-gnu/ld-linux-riscv64-lp64d.so.1",
        "/lib/ld-musl-i386.so.1",
        "/lib/ld-musl-x32.so.1",
        "/lib/ld-musl-aarch64.so.1",
        "/lib/ld-musl-aarch64_be.so.1",
        "/lib/ld-musl-riscv64.so.1",
        "/lib/ld-musl-riscv64-sf.so.1",
        "/lib/ld-musl-riscv64-sp.so.1",
        "/lib/ld-musl-x86_64.so.1",
    ] {
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_file() => {
                loaders.insert(path.into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return, // Permission or I/O failure is not evidence of absence.
        }
    }
    provides.observe(AxisId(axis::ABI_LOADER.into()), AxisValue::Names(loaders));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_numeric_and_fail_closed() {
        assert_eq!(
            parse_version("linux", "6.12.9-200.fc41.x86_64")
                .unwrap()
                .components,
            [6, 12, 9]
        );
        assert_eq!(parse_version("glibc", "2.34").unwrap().components, [2, 34]);
        for invalid in ["", "unknown", "6", "6.x.3", "6.12."] {
            assert!(parse_version("linux", invalid).is_none());
        }
    }

    #[test]
    fn every_supported_axis_is_observed_or_explicitly_unobserved() {
        let host = observe_host();
        for axis in AXES {
            let id = AxisId((*axis).into());
            assert!(host.observed(&id).is_some() || host.is_unobserved(&id));
        }
    }
}
