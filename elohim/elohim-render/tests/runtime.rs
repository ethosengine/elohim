use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn evaluates_simple_expression() {
    let mut rt = JsRuntime::new();
    let result = rt.eval_string("1 + 1").await.expect("eval ok");
    assert_eq!(result, "2");
}

#[tokio::test]
async fn surfaces_js_exception_message() {
    let mut rt = JsRuntime::new();
    let err = rt
        .eval_string("throw new Error('boom from js')")
        .await
        .expect_err("should surface exception");
    let msg = format!("{err}");
    assert!(
        msg.contains("boom from js"),
        "exception message lost — got: {msg}"
    );
}

#[tokio::test]
async fn surfaces_syntax_error_message() {
    let mut rt = JsRuntime::new();
    let err = rt
        .eval_string("this is not ; valid js !!!")
        .await
        .expect_err("should surface syntax error");
    let msg = format!("{err}");
    assert!(
        !msg.contains("compile failed"),
        "compile failed is the fallback — V8 should have given us a real syntax-error message; got: {msg}"
    );
}
