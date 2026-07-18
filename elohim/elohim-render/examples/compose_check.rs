//! Run `compose_ssr_with_shell` on two HTML files and report exactly which
//! stage failed — the compose-side twin of `render_url`.
//!
//! ```bash
//! RUSTFLAGS="" cargo run --example compose_check -- <rendered.html> <shell.html>
//! ```

fn main() {
    let mut args = std::env::args().skip(1);
    let rendered_path = args
        .next()
        .expect("usage: compose_check <rendered> <shell>");
    let shell_path = args
        .next()
        .expect("usage: compose_check <rendered> <shell>");

    let rendered = std::fs::read_to_string(&rendered_path).expect("read rendered");
    let shell = std::fs::read_to_string(&shell_path).expect("read shell");

    eprintln!(
        "rendered: {} bytes, <app-root>={}, </app-root>={}",
        rendered.len(),
        rendered.matches("<app-root").count(),
        rendered.matches("</app-root>").count()
    );
    eprintln!(
        "shell:    {} bytes, <app-root>={}, </app-root>={}",
        shell.len(),
        shell.matches("<app-root").count(),
        shell.matches("</app-root>").count()
    );

    match elohim_render::compose_ssr_with_shell(&rendered, &shell) {
        Ok(out) => {
            eprintln!("COMPOSE OK: {} bytes", out.len());
            eprintln!("composed ng-state={}", out.matches("ng-state").count());
            println!("{out}");
        }
        Err(e) => {
            eprintln!("COMPOSE REFUSED [{}]: {e}", e.reason_str());
            std::process::exit(2);
        }
    }
}
