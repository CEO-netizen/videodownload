use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use regex::Regex;
use std::process::Command;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let url = "https://m.youtube.com/watch?v=DEa5hfcZyWo&pp=ugUHEgVlbi1VUw%3D%3D";
    let client = Client::new();

    // 1. Fetch HTML
    let html = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html);

    // 2. Extract video links (regex or <video> tags)
    let re = Regex::new(r#"https?://[^\s"']+\.(mp4|mkv|webm)"#)?;
    for cap in re.captures_iter(&html) {
        println!("Found video link: {}", &cap[0]);
    }

    // 3. Use yt-dlp to get metadata + download
    let output = Command::new("yt-dlp")
        .arg("-j") // JSON output
        .arg(url)
        .output()?;

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    println!("Title: {}", metadata["title"]);

    // Download video (example: best format)
    Command::new("yt-dlp")
        .args(&["-f", "best", "-o", "%(title)s.%(ext)s", url])
        .status()?;

    // 4. Generate thumbnail via ffmpeg
    let title = metadata["title"].as_str().unwrap_or("video");
    let safe_name = sanitize_filename::sanitize(title);
    let video_file = PathBuf::from(format!("{}.mp4", safe_name));
    let thumb_file = format!("{}_thumb.jpg", safe_name);

    Command::new("ffmpeg")
        .args(&["-i", video_file.to_str().unwrap(), "-ss", "00:00:01", "-vframes", "1", &thumb_file])
        .status()?;

    println!("Thumbnail saved as {}", thumb_file);

    Ok(())
}
