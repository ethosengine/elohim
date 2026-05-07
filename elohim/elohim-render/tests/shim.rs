use elohim_render::runtime::JsRuntime;

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
