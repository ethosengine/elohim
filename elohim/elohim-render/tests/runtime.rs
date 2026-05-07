use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn evaluates_simple_expression() {
    let mut rt = JsRuntime::new();
    let result = rt.eval_string("1 + 1").await.expect("eval ok");
    assert_eq!(result, "2");
}
