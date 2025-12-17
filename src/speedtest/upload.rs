use anyhow::Result;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const UPLOAD_URL: &str = "https://speed.cloudflare.com/__up";
const CHUNK_SIZE: usize = 1_000_000; // 1MB chunks

pub struct UploadTest {
    data: Vec<u8>,
    speed_samples: Vec<f64>,
    upload_size: usize,
}

impl UploadTest {
    pub fn new(upload_size: usize) -> Self {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let data: Vec<u8> = (0..upload_size).map(|_| rng.gen()).collect();
        Self {
            data,
            speed_samples: Vec::new(),
            upload_size,
        }
    }

    pub async fn run(&mut self, progress_tx: mpsc::Sender<UploadProgress>) -> Result<UploadResult> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        let start = Instant::now();
        let mut uploaded: usize = 0;
        let mut last_update = Instant::now();
        let mut last_uploaded: usize = 0;

        self.speed_samples.clear();

        // Upload in chunks
        for chunk in self.data.chunks(CHUNK_SIZE) {
            let _ = client.post(UPLOAD_URL).body(chunk.to_vec()).send().await;
            uploaded += chunk.len();

            let now = Instant::now();
            let interval = now.duration_since(last_update);

            if interval >= Duration::from_millis(100) {
                let bytes_delta = uploaded - last_uploaded;
                let mbps = (bytes_delta as f64 * 8.0) / interval.as_secs_f64() / 1_000_000.0;
                self.speed_samples.push(mbps);

                // Keep last 200 samples
                if self.speed_samples.len() > 200 {
                    self.speed_samples.remove(0);
                }

                let _ = progress_tx
                    .send(UploadProgress {
                        uploaded_bytes: uploaded as u64,
                        total_bytes: self.upload_size as u64,
                        speed_samples: self.speed_samples.clone(),
                    })
                    .await;

                last_update = now;
                last_uploaded = uploaded;
            }
        }

        let elapsed = start.elapsed();
        let avg_speed = (self.upload_size as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;

        Ok(UploadResult { avg_speed_mbps: avg_speed })
    }
}

#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_samples: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct UploadResult {
    pub avg_speed_mbps: f64,
}
