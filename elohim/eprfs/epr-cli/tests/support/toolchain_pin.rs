//! The Rust toolchain the local gate pins, resolved for tests that must run a real compiler.
//!
//! The pin is read from `genesis/agentic/pool-policy.json` (`cargo_env_overrides["*"]`), never
//! spelled in a test, so a toolchain bump moves one file. A device has rustup and installs the
//! pin there; CI runs in a nix shell whose PATH `rustc`/`cargo` ARE the pin and which has no
//! rustup at all. Resolution: rustup present, the pinned toolchain via rustup; rustup absent,
//! PATH `rustc`/`cargo` only when its `-vV` release equals the pin. Anything else is a `SKIP`.
use std::path::PathBuf;
use std::process::Command;

/// A real compiler at the pinned release.
#[allow(
    dead_code,
    reason = "shared by several test binaries; each reads only some fields"
)]
pub struct RealToolchain {
    /// The pinned release, e.g. `1.100.0`.
    pub pin: String,
    /// Output of `rustc -vV` for the pinned compiler.
    pub identity: String,
    /// Real (non-proxy) `cargo` of the pinned toolchain.
    pub cargo: PathBuf,
    /// Whether it was resolved through rustup (a device) or PATH (a nix shell).
    pub via_rustup: bool,
}

/// The pinned release declared by the repository containing this crate.
pub fn pin() -> Option<String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let policy = manifest
        .ancestors()
        .map(|dir| dir.join("genesis/agentic/pool-policy.json"))
        .find(|path| path.is_file())?;
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(policy).ok()?).ok()?;
    value["cargo_env_overrides"]["*"]["RUSTUP_TOOLCHAIN"]
        .as_str()
        .map(str::to_owned)
}

/// Whether a `rustup` executable is on PATH.
pub fn rustup_present() -> bool {
    Command::new("rustup")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn run(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program.as_ref())
        .args(args)
        .output()
        .map_err(|e| format!("{} unavailable: {e}", program.as_ref().to_string_lossy()))?;
    if !out.status.success() {
        return Err(format!(
            "{} {args:?} failed: {}",
            program.as_ref().to_string_lossy(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Resolve the pinned compiler, or say why it is unavailable (the caller prints `SKIP: <reason>`).
pub fn real_toolchain() -> Result<RealToolchain, String> {
    let pin = pin().ok_or("no toolchain pin declared in genesis/agentic/pool-policy.json")?;
    if rustup_present() {
        let identity = run("rustup", &["run", &pin, "rustc", "-vV"])
            .map_err(|e| format!("toolchain {pin} is not installed: {e}"))?;
        let cargo = run("rustup", &["which", "--toolchain", &pin, "cargo"])?;
        return Ok(RealToolchain {
            pin,
            identity,
            cargo: PathBuf::from(cargo.trim()),
            via_rustup: true,
        });
    }
    let identity = run("rustc", &["-vV"])?;
    let release = identity
        .lines()
        .find_map(|line| line.strip_prefix("release: "))
        .unwrap_or_default();
    if release != pin {
        return Err(format!(
            "no rustup, and PATH rustc is {release:?}, not the pinned {pin}"
        ));
    }
    let cargo = on_path("cargo").ok_or("no rustup, and no cargo on PATH")?;
    Ok(RealToolchain {
        pin,
        identity,
        cargo,
        via_rustup: false,
    })
}

/// [`real_toolchain`], printing `SKIP: <reason>` when it is unavailable.
pub fn real_toolchain_or_skip() -> Option<RealToolchain> {
    match real_toolchain() {
        Ok(toolchain) => Some(toolchain),
        Err(reason) => {
            eprintln!("SKIP: {reason}");
            None
        }
    }
}
