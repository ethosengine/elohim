//! Proxy refusal is observed in child processes, without mutating the test runner's environment.

use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};

struct ProxyFixture {
    repo: TempDir,
    scratch: TempDir,
    descriptor: MethodDescriptor,
    method: PathBuf,
    proxy_marker: PathBuf,
    compiler_marker: PathBuf,
    home: PathBuf,
    compiler: PathBuf,
}

impl ProxyFixture {
    fn new(hard_link: bool) -> Self {
        let repo = checkout();
        let scratch = TempDir::new().unwrap();
        let tools = scratch.path().join("bin");
        fake_cc(&tools, "cc fixture");
        let proxy_marker = scratch.path().join("proxy-ran");
        // Do not use fake_tool: its readiness probe would execute the proxy under test.
        let proxy = tools.join("rustup");
        std::fs::write(
            &proxy,
            format!(
                "#!/bin/sh\necho ran > '{}'\nexit 99\n",
                proxy_marker.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&proxy, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_file(tools.join("rustc")).unwrap();
        if hard_link {
            std::fs::hard_link(&proxy, tools.join("rustc")).unwrap();
        } else {
            symlink(&proxy, tools.join("rustc")).unwrap();
        }
        let home = scratch.path().join("home/.rustup");
        let bin = home.join("toolchains/1.96.1-x86_64-unknown-linux-gnu/bin");
        let compiler_marker = scratch.path().join("compiler-ran");
        fake_tool(&bin, "rustc", &format!(
            "[ \"$1\" = --version ] && exit 0\n[ \"$TMPDIR\" = \"$PWD\" ] || exit 98\necho \"$*\" >> '{}'\nif [ \"$1\" = -vV ]; then printf '%s' '{}'; else echo 'note: native-static-libs: -lc' >&2; fi",
            compiler_marker.display(), fixture::RUSTC_IDENTITY
        ));
        let path = format!("{}:/bin", tools.display());
        let (method, descriptor) =
            method_file_with(scratch.path(), &["crate/src"], &[("PATH", &path)]);
        Self {
            repo,
            scratch,
            descriptor,
            method,
            proxy_marker,
            compiler_marker,
            home,
            compiler: bin.join("rustc"),
        }
    }

    fn key(&self, home: Option<&Path>) -> Output {
        std::fs::write(&self.method, serde_json::to_vec(&self.descriptor).unwrap()).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .env_clear()
            .envs(&self.descriptor.non_path_inputs)
            .env("HOME", self.scratch.path().join("home"))
            .env("XDG_CONFIG_HOME", self.scratch.path().join("config"));
        if let Some(home) = home {
            command.env("RUSTUP_HOME", home);
        }
        // Match the caller's temp location; probe itself must use its private child directory.
        if let Some(tmp) = std::env::var_os("TMPDIR") {
            command.env("TMPDIR", tmp);
        }
        let out = command
            .args(["artifact", "key", "--json", "--repo"])
            .arg(self.repo.path())
            .arg("--method")
            .arg(&self.method)
            .output()
            .unwrap();
        assert!(
            !self.proxy_marker.exists(),
            "the rustup proxy must never execute"
        );
        out
    }
}

#[test]
fn artifact_proxy_nonexistent_rustup_home_refuses_without_creating_it() {
    let fixture = ProxyFixture::new(false);
    let absent = fixture.scratch.path().join("absent-rustup-home");
    let out = fixture.key(Some(&absent));
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("must be an existing directory"),
        "{}",
        text(&out)
    );
    assert!(text(&out).contains(absent.to_str().unwrap()));
    assert!(
        !absent.exists(),
        "a refusal must not initialize RUSTUP_HOME"
    );
    assert!(!fixture.compiler_marker.exists());
}

#[test]
fn artifact_proxy_uses_installed_compiler_without_executing_symlink_or_hardlink() {
    for hard_link in [false, true] {
        let mut fixture = ProxyFixture::new(hard_link);
        // Toolchain location must use host, not this cross-compilation target.
        fixture
            .descriptor
            .flags
            .push("--target=aarch64-unknown-linux-gnu".into());
        for home in [Some(fixture.home.as_path()), None] {
            let out = fixture.key(home);
            assert!(out.status.success(), "{}", text(&out));
        }
        // Also accept an explicitly host-qualified recorded selector.
        fixture
            .descriptor
            .tools
            .insert("toolchain".into(), "1.96.1-x86_64-unknown-linux-gnu".into());
        let out = fixture.key(Some(&fixture.home));
        assert!(out.status.success(), "{}", text(&out));
        let calls = std::fs::read_to_string(&fixture.compiler_marker).unwrap();
        assert_eq!(calls.lines().filter(|line| *line == "-vV").count(), 3);
        assert_eq!(
            calls
                .lines()
                .filter(|line| line.starts_with("--print native-static-libs "))
                .count(),
            3
        );
    }
}

#[test]
fn artifact_proxy_without_recorded_toolchain_refuses() {
    let mut fixture = ProxyFixture::new(false);
    fixture.descriptor.tools.remove("toolchain");
    let out = fixture.key(Some(&fixture.home));
    assert!(!out.status.success());
    assert!(
        text(&out).contains("needs a recorded rustc toolchain"),
        "{}",
        text(&out)
    );
    assert!(!fixture.compiler_marker.exists());
}

#[test]
fn artifact_proxy_rejects_unusable_or_proxy_toolchain_binary() {
    let fixture = ProxyFixture::new(false);
    std::fs::set_permissions(&fixture.compiler, std::fs::Permissions::from_mode(0o644)).unwrap();
    for directory in [false, true] {
        if directory {
            std::fs::remove_file(&fixture.compiler).unwrap();
            std::fs::create_dir(&fixture.compiler).unwrap();
        }
        let out = fixture.key(Some(&fixture.home));
        assert!(!out.status.success());
        assert!(
            text(&out).contains("regular executable file"),
            "{}",
            text(&out)
        );
    }
    std::fs::remove_dir(&fixture.compiler).unwrap();
    symlink(fixture.scratch.path().join("bin/rustup"), &fixture.compiler).unwrap();
    let out = fixture.key(Some(&fixture.home));
    assert!(!out.status.success());
    assert!(
        text(&out).contains("itself a rustup proxy"),
        "{}",
        text(&out)
    );
    std::fs::remove_file(&fixture.compiler).unwrap();
    std::fs::hard_link(fixture.scratch.path().join("bin/rustup"), &fixture.compiler).unwrap();
    let out = fixture.key(Some(&fixture.home));
    assert!(!out.status.success());
    assert!(
        text(&out).contains("itself a rustup proxy"),
        "{}",
        text(&out)
    );
    assert!(!fixture.compiler_marker.exists());
}
