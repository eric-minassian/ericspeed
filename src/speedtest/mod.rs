pub mod download;
pub mod ping;
pub mod upload;

#[derive(Debug, Clone, Default)]
pub struct SpeedTestResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPhase {
    Idle,
    Ping,
    Download,
    Upload,
    Complete,
}
