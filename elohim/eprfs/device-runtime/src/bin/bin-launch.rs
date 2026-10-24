//! Native argv[0] launcher. The exec target, not this launcher, becomes current_exe().
fn main() {
    let mut args = std::env::args_os();
    let invoked = args.next().unwrap_or_default();
    let result = elohim_device_runtime::bin_store::Registry::discover().and_then(|registry| {
        let args: Vec<_> = args.collect();
        elohim_device_runtime::bin_store::launch_os(
            &registry,
            std::path::Path::new(&invoked),
            &args,
        )
    });
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(127);
    }
}
