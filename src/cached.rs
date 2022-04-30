use color_eyre::{eyre::eyre, Report};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tracing::debug;

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

#[derive(Clone, Default)]
struct CachedLatestVideo {
    inner: Arc<Mutex<CachedLastVideoInner>>,
}

#[derive(Default)]
struct CachedLastVideoInner {
    last_fetched: Option<(Instant, String)>,
    inflight: Option<broadcast::Sender<Result<String, CachedError>>>,
}
