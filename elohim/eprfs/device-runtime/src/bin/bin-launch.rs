//! Native argv[0] launcher. The exec target, not this launcher, becomes current_exe().
fn main() {
    let mut args = std::env::args_os();
    let invoked = args.next().unwrap_or_default();
    let result = elohim_device_runtime::bin_store::Registry::discover().and_then(|registry| {
        let args: Vec<_> = args.collect();
        if std::path::Path::new(&invoked)
            .file_name()
            .is_some_and(|n| n == "bin-launch")
            && args.first().is_some_and(|a| a == "resolve")
        {
            return resolve_current(&registry, &args);
        }
        elohim_device_runtime::bin_store::launch_os(
            &registry,
            std::path::Path::new(&invoked),
            &args,
        )
    });
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(if error.kind == "bootstrapping" {
            75
        } else {
            127
        });
    }
}

// Inspection deliberately never enters launch_os: even a query must not publish a pin.
fn resolve_current(
    registry: &elohim_device_runtime::bin_store::Registry,
    args: &[std::ffi::OsString],
) -> Result<(), elohim_device_runtime::bin_store::Refusal> {
    use elohim_device_runtime::bin_store::{Refusal, Store};
    if args.len() != 5 || args[1] != "--tool" || args[3] != "--current" || args[4] != "--json" {
        return Err(Refusal {
            kind: "launcher-usage".into(),
            detail: "bin-launch resolve --tool NAME --current --json".into(),
        });
    }
    let name = args[2].to_str().ok_or_else(|| Refusal {
        kind: "launcher-usage".into(),
        detail: "tool name must be UTF-8".into(),
    })?;
    let resolution = Store::from_environment(name, registry)?.resolve_current(None)?;
    println!("{}", serde_json::to_string(&resolution)?);
    Ok(())
}
