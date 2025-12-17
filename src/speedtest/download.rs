use anyhow::Result;
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down";

pub struct DownloadTest {
    speed_samples: Vec<f64>,
    download_size: u64,
}

impl DownloadTest {
    pub fn new(download_size: u64) -> Self {
        Self {
            speed_samples: Vec::new(),
            download_size,
        }
    }

    pub async fn run(&mut self, progress_tx: mpsc::Sender<DownloadProgress>) -> Result<DownloadResult> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        let url = format!("{}?bytes={}", DOWNLOAD_URL, self.download_size);
        let response = client.get(&url).send().await?;
        let total_size = response.content_length().unwrap_or(self.download_size);
        let mut stream = response.bytes_stream();

        let start = Instant::now();
        let mut downloaded: u64 = 0;
        let mut last_update = Instant::now();
        let mut last_downloaded: u64 = 0;

        self.speed_samples.clear();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;

            let now = Instant::now();
            let interval = now.duration_since(last_update);

            if interval >= Duration::from_millis(100) {
                let bytes_delta = downloaded - last_downloaded;
                let mbps = (bytes_delta as f64 * 8.0) / interval.as_secs_f64() / 1_000_000.0;
                self.speed_samples.push(mbps);

                // Keep last 200 samples
                if self.speed_samples.len() > 200 {
                    self.speed_samples.remove(0);
                }

                let _ = progress_tx
                    .send(DownloadProgress {
                        downloaded_bytes: downloaded,
                        total_bytes: total_size,
                        speed_samples: self.speed_samples.clone(),
                    })
                    .await;

                last_update = now;
                last_downloaded = downloaded;
            }
        }

        let elapsed = start.elapsed();
        let avg_speed = (downloaded as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;

        Ok(DownloadResult { avg_speed_mbps: avg_speed })
    }
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_samples: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub avg_speed_mbps: f64,
}
