use doorway_client::Route;

#[test]
fn render_field_round_trips_through_serde() {
    let route = Route::get("/lamad/concept/{id}")
        .handler("get_concept")
        .render("angular-ssr")
        .build();
    let json = serde_json::to_string(&route).unwrap();
    assert!(json.contains(r#""render":"angular-ssr""#), "json: {}", json);
    let back: Route = serde_json::from_str(&json).unwrap();
    assert_eq!(back.render.as_deref(), Some("angular-ssr"));
}

#[test]
fn render_field_omitted_when_none() {
    let route = Route::get("/blob/{hash}").handler("get_blob").build();
    let json = serde_json::to_string(&route).unwrap();
    assert!(
        !json.contains("render"),
        "json should omit render when None: {}",
        json
    );
}
