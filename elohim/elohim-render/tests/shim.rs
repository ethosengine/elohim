use elohim_render::runtime::JsRuntime;

#[test]
fn shim_js_files_are_pure_ascii() {
    let shim_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shim");
    let mut violations = Vec::new();
    for entry in std::fs::read_dir(&shim_dir).expect("read shim dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        let content = std::fs::read(&path).expect("read .js file");
        if !content.is_ascii() {
            violations.push(path);
        }
    }
    assert!(
        violations.is_empty(),
        "non-ASCII bytes in shim .js files (deno_core ascii_str_include! will panic): {:?}",
        violations
    );
}

#[tokio::test]
async fn console_log_does_not_throw() {
    let mut rt = JsRuntime::with_shims();
    let v = rt.eval_string("console.log('hello'); 42").await.unwrap();
    assert_eq!(v, "42");
}

#[tokio::test]
async fn url_constructor_works() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new URL('/foo', 'https://example.com').href")
        .await
        .unwrap();
    assert_eq!(v, "https://example.com/foo");
}

#[tokio::test]
async fn text_encoder_round_trips() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new TextDecoder().decode(new TextEncoder().encode('hi'))")
        .await
        .unwrap();
    assert_eq!(v, "hi");
}
