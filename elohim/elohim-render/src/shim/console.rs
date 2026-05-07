use deno_core::op2;

#[op2(fast)]
fn op_console_log(#[string] msg: String) {
    tracing::info!(target: "elohim_render::js_console", "{}", msg);
}

#[op2(fast)]
fn op_console_warn(#[string] msg: String) {
    tracing::warn!(target: "elohim_render::js_console", "{}", msg);
}

#[op2(fast)]
fn op_console_error(#[string] msg: String) {
    tracing::error!(target: "elohim_render::js_console", "{}", msg);
}

deno_core::extension!(
    console_ext,
    ops = [op_console_log, op_console_warn, op_console_error],
    js = [dir "src/shim", "console.js"],
);
