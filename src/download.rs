use crate::modrinth::ModrinthProject;
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Url;
use reqwest::header::USER_AGENT;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

/// Download up to 8 files at a time
pub async fn download_files(
    urls: Vec<ModrinthProject>,
    dest_dir: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    const CONCURRENCY: usize = 8;

    let dest_dir = dest_dir.as_ref().to_path_buf();
    fs::create_dir_all(&dest_dir).await?;

    let client = Arc::new(reqwest::Client::new());
    let mp = Arc::new(MultiProgress::new());
    let sem = Arc::new(Semaphore::new(CONCURRENCY));

    let mut tasks = FuturesUnordered::new();

    for url in urls {
        let client = Arc::clone(&client);
        let mp = Arc::clone(&mp);
        let sem = Arc::clone(&sem);
        let dest_dir = dest_dir.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let url_parsed = Url::parse(&url.download_link)?;
            let resp = client
                .get(url_parsed.clone())
                .header(
                    USER_AGENT,
                    "kay-xr/modrinth_collection_downloader/0.1.0 (archangel@angelware.net)",
                )
                .send()
                .await?
                .error_for_status()?;

            let filename = filename_from_response(&url_parsed, &resp);
            let filepath = dest_dir.join(&filename);

            // indicatif bar
            let total = resp.content_length();
            let pb = mp.add(match total {
                Some(n) => ProgressBar::new(n),
                None => ProgressBar::new_spinner(),
            });

            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} {msg:.dim} {bytes:>10}/{total_bytes:10} ({eta})\n{bar:40.cyan/blue}",
                )?
            );
            pb.set_message(filename.clone());

            let mut file = File::create(&filepath).await?;
            let mut stream = resp.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file.write_all(&chunk).await?;
                pb.inc(chunk.len() as u64);
            }

            pb.finish_with_message(format!("{} ✓", filename));
            Ok::<PathBuf, anyhow::Error>(filepath)
        }));
    }

    // run all
    let mut saved = Vec::new();
    while let Some(res) = tasks.next().await {
        match res {
            Ok(Ok(path)) => saved.push(path),
            Ok(Err(e)) => eprintln!("Download failed: {e}"),
            Err(join_err) => eprintln!("Task join error: {join_err}"),
        }
    }

    Ok(saved)
}

/// Trying to extract the filename from header or fallback to the URL
fn filename_from_response(url: &Url, resp: &reqwest::Response) -> String {
    if let Some(disposition) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
        if let Ok(s) = disposition.to_str() {
            if let Some(idx) = s.to_ascii_lowercase().find("filename=") {
                let mut v = s[idx + "filename=".len()..].trim().trim_matches(';').trim();
                v = v.trim_matches('"');
                if !v.is_empty() {
                    return v.to_string();
                }
            }
        }
    }

    // fallback, use url
    let raw = url
        .path_segments()
        .and_then(|segment| segment.last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "download".to_string());

    // deciding sequences
    urlencoding::decode(&raw)
        .unwrap_or_else(|_| raw.clone().into())
        .into_owned()
}
