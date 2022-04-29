use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router, Server,
};
use color_eyre::Report;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{error::Error, net::SocketAddr};
use tower_http::trace::TraceLayer;
use tracing::{debug, info};

mod tracing_stuff;
mod youtube;

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
        .layer(Extension(CachedLatestVideo::default()));
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument(skip(client, cached))]
async fn root(
    client: Extension<reqwest::Client>,
    cached: Extension<CachedLatestVideo>,
) -> Result<impl IntoResponse, ReportError> {
    #[derive(Serialize)]
    struct Response {
        video_id: String,
    }

    {
        if let Some((cached_at, video_id)) = cached.value.lock().unwrap().as_ref() {
            if cached_at.elapsed() < std::time::Duration::from_secs(5) {
                return Ok(Json(Response {
                    video_id: video_id.clone(),
                }));
            } else {
                // was stale, let's refresh
                debug!("stale video, let's refresh");
            }
        } else {
            debug!("not cached, let's fetch");
        }
    }

    let video_id = youtube::fetch_video_id(&client).await?;
    cached
        .value
        .lock()
        .unwrap()
        .replace((Instant::now(), video_id.clone()));

    Ok(Json(Response { video_id }))
}

#[derive(Clone, Default)]
struct CachedLatestVideo {
    value: Arc<Mutex<Option<(Instant, String)>>>,
}

pub struct ReportError(Report);

impl From<Report> for ReportError {
    fn from(err: Report) -> Self {
        ReportError(err)
    }
}

impl IntoResponse for ReportError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("youtube error: {:?}", self.0),
        )
            .into_response()
    }
}
