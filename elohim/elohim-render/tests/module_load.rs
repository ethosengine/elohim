use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn loads_esm_module_and_calls_export() {
    let mut rt = JsRuntime::with_fs_loader();
    let module_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/echo-bundle.mjs");
    let html = rt
        .render_via_module(&module_path, "/foo")
        .await
        .expect("module render ok");
    assert!(html.contains("/foo"), "html: {}", html);
}

#[tokio::test]
async fn render_via_module_errors_on_runtime_without_fs_loader() {
    let mut rt = JsRuntime::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/echo-bundle.mjs");
    let err = rt
        .render_via_module(&path, "/foo")
        .await
        .expect_err("should fail without FS loader");
    let msg = format!("{err}");
    assert!(
        msg.contains("FS loader") || msg.contains("fs loader"),
        "msg: {msg}"
    );
}
