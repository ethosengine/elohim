//! Verify that routes with `render` set in the manifest carry their render_spec
//! through the registry's match_request output.
//!
//! Task 13: manifest-driven SSR dispatch.

use doorway::services::route_registry::RouteRegistry;
use doorway_client::{DoorwayRoutesBuilder, Route};

#[tokio::test]
async fn registry_marks_render_routes_for_ssr_dispatch() {
    let routes = DoorwayRoutesBuilder::new()
        .route(
            Route::get("/lamad/concept/{id}")
                .handler("get_content")
                .render("angular-ssr")
                .build(),
        )
        .route(Route::get("/blob/{hash}").handler("get_blob").build())
        .build();

    let registry = RouteRegistry::with_routes("http://storage:8090", routes);

    let matches = registry
        .match_request(doorway_client::HttpMethod::Get, "/lamad/concept/abc")
        .await;
    let m1 = matches.first().expect("matched concept route");
    assert_eq!(
        m1.render_spec.as_deref(),
        Some("angular-ssr"),
        "SSR-eligible route must carry render_spec through registry"
    );

    let matches2 = registry
        .match_request(doorway_client::HttpMethod::Get, "/blob/sha256-x")
        .await;
    let m2 = matches2.first().expect("matched blob route");
    assert_eq!(
        m2.render_spec.as_deref(),
        None,
        "Non-SSR route must have render_spec = None"
    );
}
