use crate::cached::{Cached, ReportError};
use axum::{response::IntoResponse, routing::get, Extension, Json, Router, Server};
use color_eyre::Report;
use serde::Serialize;
use std::time::Duration;
use std::{error::Error, net::SocketAddr};
use tower_http::trace::TraceLayer;
use tracing::info;

mod cached;
mod tracing_stuff;
mod youtube;

#[derive(Clone)]
struct LatestVideo(String);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;

    tracing_stuff::setup()?;
    run_server().await?;
    tracing_stuff::teardown();
    Ok(())
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:3779".parse()?;
    info!("Listening on http://{}", addr);

    let app = Router::new()
        .route("/", get(root))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(reqwest::Client::new()))
        .layer(Extension(Cached::<LatestVideo>::new(Duration::from_secs(
            5,
        ))));
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument(skip(client, cached))]
async fn root(
    client: Extension<reqwest::Client>,
    cached: Extension<Cached<LatestVideo>>,
) -> Result<impl IntoResponse, ReportError> {
    #[derive(Serialize)]
    struct Response {
        video_id: String,
    }

    let LatestVideo(video_id) = cached
        .get_cached(|| {
            Box::pin(async move {
                let video_id = youtube::fetch_video_id(&client).await?;
                Ok::<_, Report>(LatestVideo(video_id))
            })
        })
        .await?;

    Ok(Json(Response { video_id }))
}
