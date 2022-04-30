use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use color_eyre::{eyre::eyre, Report};
use std::future::Future;
use std::pin::Pin;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tracing::debug;

pub type BoxFut<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

#[derive(Debug, Clone, thiserror::Error)]
#[error("stringified error: {inner}")]
pub struct CachedError {
    inner: String,
}

pub struct ReportError(Report);

impl CachedError {
    pub fn new<E: std::fmt::Display>(e: E) -> Self {
        Self {
            inner: e.to_string(),
        }
    }
}

impl From<Report> for CachedError {
    fn from(e: Report) -> Self {
        CachedError::new(e)
    }
}

impl From<broadcast::error::RecvError> for CachedError {
    fn from(e: broadcast::error::RecvError) -> Self {
        CachedError::new(e)
    }
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

#[derive(Clone)]
pub struct Cached<T>
where
    T: Clone + Send + Sync + 'static,
{
    inner: Arc<Mutex<CachedLastVideoInner<T>>>,
    refresh_interval: Duration,
}

struct CachedLastVideoInner<T>
where
    T: Clone + Send + Sync + 'static,
{
    last_fetched: Option<(Instant, T)>,
    inflight: Option<broadcast::Sender<Result<T, CachedError>>>,
}

impl<T> Default for CachedLastVideoInner<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            last_fetched: None,
            inflight: None,
        }
    }
}

impl<T> Cached<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(refresh_interval: Duration) -> Self {
        Self {
            inner: Default::default(),
            refresh_interval,
        }
    }

    pub async fn get_cached<F, E>(&self, f: F) -> Result<T, CachedError>
    where
        F: FnOnce() -> BoxFut<'static, Result<T, E>> + Send + 'static,
        E: std::fmt::Display + 'static,
    {
        let mut rx = {
            // only sync code in this block
            let mut inner = self.inner.lock().unwrap();

            if let Some((fetched_at, value)) = inner.last_fetched.as_ref() {
                if fetched_at.elapsed() < std::time::Duration::from_secs(5) {
                    return Ok(value.clone());
                } else {
                    // was stale, let's refresh
                    debug!("stale, let's refresh");
                }
            }

            if let Some(inflight) = inner.inflight.as_ref() {
                inflight.subscribe()
            } else {
                // there isn't, let's fetch
                let (tx, rx) = broadcast::channel::<Result<T, CachedError>>(1);
                inner.inflight = Some(tx.clone());
                let inner = self.inner.clone();

                let fut = f();

                tokio::spawn(async move {
                    let res = fut.await;

                    {
                        // only sync code in this block
                        let mut inner = inner.lock().unwrap();
                        inner.inflight = None;

                        match res {
                            Ok(value) => {
                                inner.last_fetched.replace((Instant::now(), value.clone()));
                                let _ = tx.send(Ok(value));
                            }
                            Err(e) => {
                                let _ = tx.send(Err(CachedError {
                                    inner: e.to_string(),
                                }));
                            }
                        };
                    }
                });
                rx
            }
        };

        // if we reached here, we're waiting for an in-flight request (we weren't
        // able to serve from cache)
        Ok(rx
            .recv()
            .await
            .map_err(|_| eyre!("in-flight request died"))??)
    }
}
