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

#[tokio::test]
async fn url_searchparams_basic_ops() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('https://example.com/?a=1&b=2');
            const p = u.searchParams;
            `${p.get('a')}|${p.get('b')}|${p.has('c')}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "1|2|false");
}

#[tokio::test]
async fn url_ipv6_hostname_port() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('http://[::1]:8080/path');
            `${u.hostname}|${u.port}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "[::1]|8080");
}

#[tokio::test]
async fn url_default_port_elided() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('https://example.com:443/');
            `${u.host}|${u.origin}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "example.com|https://example.com");
}
