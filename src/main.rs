use crate::cached::{CachedError, ReportError};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router, Server,
};
use color_eyre::{eyre::eyre, Report};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{error::Error, net::SocketAddr};
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing::{debug, info};

mod cached;
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

    let mut rx = {
        let mut inner = cached.inner.lock().unwrap();
        if let Some((cached_at, video_id)) = inner.last_fetched.as_ref() {
            if cached_at.elapsed() < std::time::Duration::from_secs(5) {
                return Ok(Json(Response {
                    video_id: video_id.clone(),
                }));
            } else {
                // was stale, let's refresh
                debug!("stale video, let's refresh");
            }
        }

        // is there an in-flight request?
        if let Some(inflight) = inner.inflight.as_ref() {
            inflight.subscribe()
        } else {
            // there isn't, let's fetch
            let (tx, rx) = broadcast::channel::<Result<String, CachedError>>(1);
            inner.inflight = Some(tx.clone());
            let cached = cached.clone();
            tokio::spawn(async move {
                let video_id = youtube::fetch_video_id(&client).await;
                {
                    // only sync code in this block
                    let mut inner = cached.inner.lock().unwrap();
                    inner.inflight = None;

                    match video_id {
                        Ok(video_id) => {
                            inner
                                .last_fetched
                                .replace((Instant::now(), video_id.clone()));
                            let _ = tx.send(Ok(video_id));
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e.into()));
                        }
                    };
                }
            });
            rx
        }
    };

    // if we reached here, we're waiting for an in-flight request ( we weren't
    // able to serve from cache
    Ok(Json(Response {
        video_id: rx
            .recv()
            .await
            .map_err(|_| eyre!("in-flight request died"))??,
    }))
}

impl From<CachedError> for ReportError {
    fn from(err: CachedError) -> Self {
        ReportError(err.into())
    }
}

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
