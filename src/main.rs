use anyhow::{Result, bail};
use clap::Parser;
use regex::Regex;
use reqwest::Client;
use scraper::Html;
use std::path::PathBuf;
use std::process::Command;

/// Simple video downloader + thumbnail generator
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Video page URL
    #[arg(long)]
    url: String,

    /// Output directory
    #[arg(long, default_value = ".")]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = Client::new();

    // 1. Fetch HTML
    let html = client.get(&args.url).send().await?.text().await?;
    let document = Html::parse_document(&html);

    // 2. Extract video links
    let re = Regex::new(r#"https?://[^\s"']+\.(mp4|mkv|webm)"#)?;
    for cap in re.captures_iter(&html) {
        println!("Found video link: {}", &cap[0]);
    }

    // 3. yt-dlp metadata
    let output = Command::new("yt-dlp")
        .arg("-j")
        .arg(&args.url)
        .output()?;
    if !output.status.success() {
        bail!("yt-dlp metadata fetch failed");
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let (title, ext) = if metadata.is_array() {
        let item = &metadata[0];
        (
            item["title"].as_str().unwrap_or("video"),
            item["ext"].as_str().unwrap_or("mp4"),
        )
    } else {
        (
            metadata["title"].as_str().unwrap_or("video"),
            metadata["ext"].as_str().unwrap_or("mp4"),
        )
    };
    println!("Title: {}", title);

    // 4. Download video
    let status = Command::new("yt-dlp")
        .args(&[
            "-f",
            "best",
            "-o",
            &format!("{}/%(title)s.%(ext)s", args.output.display()),
            &args.url,
        ])
        .status()?;
    if !status.success() {
        bail!("yt-dlp download failed");
    }

    // 5. Thumbnail
    let safe_name = sanitize_filename::sanitize(title);
    let video_file = args.output.join(format!("{}.{}", safe_name, ext));
    let thumb_file = args.output.join(format!("{}_thumb.jpg", safe_name));

    let status = Command::new("ffmpeg")
        .args(&[
            "-i",
            video_file.to_str().unwrap(),
            "-ss",
            "00:00:01",
            "-vframes",
            "1",
            thumb_file.to_str().unwrap(),
        ])
        .status()?;
    if !status.success() {
        bail!("ffmpeg thumbnail generation failed");
    }

    println!("Thumbnail saved as {}", thumb_file.display());
    Ok(())
}
