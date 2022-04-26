use axum::{response::IntoResponse, routing::get, Router, Server};
use std::{error::Error, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

mod tracing_stuff;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_stuff::setup()?;
    run_server().await?;
    tracing_stuff::teardown();
    Ok(())
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:3779".parse()?;
    info!("Listening on http://{}", addr);

    let app = Router::new().route("/", get(root)).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .into_inner(),
    );
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument]
async fn root() -> impl IntoResponse {
    "Hello from plaque!\n"
}
