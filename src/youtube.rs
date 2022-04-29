use color_eyre::{eyre::eyre, Report};
use tap::Pipe;
use url::Url;

const YT_CHANNEL_ID: &str = "UCs4fQRyl1TJvoeOdekW6lYA";

#[tracing::instrument]
pub(crate) async fn fetch_video_id(client: &reqwest::Client) -> Result<String, Report> {
    Ok(client
        .get({
            let mut url = Url::parse("https://www.youtube.com/feeds/videos.xml")?;
            url.query_pairs_mut()
                .append_pair("channel_id", YT_CHANNEL_ID);
            url
        })
        .header("user-agent", "cool/bear")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .pipe(|bytes| feed_rs::parser::parse(&bytes[..]))?
        .pipe_ref(|feed| feed.entries.get(0))
        .ok_or(eyre!("no entries in video feed"))?
        .pipe_ref(|entry| entry.id.strip_prefix("yt:video:"))
        .ok_or(eyre!("first video feed item wasn't a video"))?
        .to_string())
}
