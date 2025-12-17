use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const PING_URL: &str = "https://speed.cloudflare.com/__down?bytes=0";

pub struct PingTest {
    samples: Vec<f64>,
    ping_count: usize,
}

impl PingTest {
    pub fn new(ping_count: usize) -> Self {
        Self {
            samples: Vec::new(),
            ping_count,
        }
    }

    pub async fn run(&mut self, progress_tx: mpsc::Sender<PingProgress>) -> Result<PingResult> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        self.samples.clear();

        for _ in 0..self.ping_count {
            let start = Instant::now();
            if client.get(PING_URL).send().await.is_ok() {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                self.samples.push(elapsed);
            }

            let _ = progress_tx
                .send(PingProgress {
                    latest_ping: self.samples.last().copied(),
                })
                .await;

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Ok(self.calculate_result())
    }

    fn calculate_result(&self) -> PingResult {
        if self.samples.is_empty() {
            return PingResult { avg_ms: 0.0, jitter_ms: 0.0 };
        }

        let avg = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        let jitter = if self.samples.len() > 1 {
            let variance: f64 = self.samples.iter().map(|&x| (x - avg).powi(2)).sum::<f64>()
                / (self.samples.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        PingResult { avg_ms: avg, jitter_ms: jitter }
    }
}

#[derive(Debug, Clone)]
pub struct PingProgress {
    pub latest_ping: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub avg_ms: f64,
    pub jitter_ms: f64,
}
