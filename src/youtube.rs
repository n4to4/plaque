use std::error::Error;
use url::Url;

const YT_CHANNEL_ID: &str = "UCs4fQRyl1TJvoeOdekW6lYA";

pub(crate) async fn fetch_video_id() -> Result<String, Box<dyn Error>> {
    let mut api_url = Url::parse("https://www.youtube.com/feeds/videos.xml")?;
    {
        let mut q = api_url.query_pairs_mut();
        q.append_pair("channel_id", YT_CHANNEL_ID);
    }
    todo!("fetch from {api_url}");
}
