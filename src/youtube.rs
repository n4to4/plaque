use color_eyre::{eyre::eyre, Report};
use url::Url;

const YT_CHANNEL_ID: &str = "UCs4fQRyl1TJvoeOdekW6lYA";

#[tracing::instrument]
pub(crate) async fn fetch_video_id() -> Result<String, Report> {
    let mut api_url = Url::parse("https://www.youtube.com/feeds/videos.xml")?;
    {
        let mut q = api_url.query_pairs_mut();
        q.append_pair("channel_id", YT_CHANNEL_ID);
    }
    Err(eyre!("TODO: fetch from {api_url}"))
}
