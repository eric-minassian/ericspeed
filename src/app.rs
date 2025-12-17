use crate::settings::{Settings, SettingsField};
use crate::speedtest::{
    download::{DownloadProgress, DownloadTest},
    ping::{PingProgress, PingTest},
    upload::{UploadProgress, UploadTest},
    SpeedTestResult, TestPhase,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Main,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Download,
    Upload,
    Ping,
}

impl Panel {
    pub fn next(self) -> Self {
        match self {
            Panel::Download => Panel::Upload,
            Panel::Upload => Panel::Ping,
            Panel::Ping => Panel::Download,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Panel::Download => Panel::Ping,
            Panel::Upload => Panel::Download,
            Panel::Ping => Panel::Upload,
        }
    }
}

pub struct App {
    pub phase: TestPhase,
    pub result: SpeedTestResult,
    pub should_quit: bool,

    // UI state
    pub view: AppView,
    pub selected_panel: Panel,
    pub expanded: bool,

    // Settings
    pub settings: Settings,
    pub selected_setting: SettingsField,

    // Progress tracking
    pub download_progress: f64,
    pub upload_progress: f64,

    // Speed samples for charts
    pub download_samples: Vec<f64>,
    pub upload_samples: Vec<f64>,
    pub ping_samples: Vec<f64>,

    cancel_tx: Option<mpsc::Sender<()>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            phase: TestPhase::Idle,
            result: SpeedTestResult::default(),
            should_quit: false,
            view: AppView::Main,
            selected_panel: Panel::Download,
            expanded: false,
            settings: Settings::default(),
            selected_setting: SettingsField::PingCount,
            download_progress: 0.0,
            upload_progress: 0.0,
            download_samples: Vec::new(),
            upload_samples: Vec::new(),
            ping_samples: Vec::new(),
            cancel_tx: None,
        }
    }

    pub fn handle_key_event(&mut self, key: event::KeyEvent) -> Option<AppAction> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        match self.view {
            AppView::Main => self.handle_main_key(key),
            AppView::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_main_key(&mut self, key: event::KeyEvent) -> Option<AppAction> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                Some(AppAction::Quit)
            }
            KeyCode::Char('s') => {
                if self.phase == TestPhase::Idle || self.phase == TestPhase::Complete {
                    self.view = AppView::Settings;
                }
                None
            }
            KeyCode::Enter => {
                if self.expanded {
                    self.expanded = false;
                    None
                } else if self.phase == TestPhase::Idle || self.phase == TestPhase::Complete {
                    Some(AppAction::StartTest)
                } else {
                    // Expand current panel during test
                    self.expanded = true;
                    None
                }
            }
            KeyCode::Esc => {
                if self.expanded {
                    self.expanded = false;
                    None
                } else if self.phase != TestPhase::Idle && self.phase != TestPhase::Complete {
                    Some(AppAction::CancelTest)
                } else {
                    None
                }
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('j') => {
                if !self.expanded {
                    self.selected_panel = self.selected_panel.next();
                }
                None
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('k') => {
                if !self.expanded {
                    self.selected_panel = self.selected_panel.prev();
                }
                None
            }
            KeyCode::Char(' ') => {
                self.expanded = !self.expanded;
                None
            }
            _ => None,
        }
    }

    fn handle_settings_key(&mut self, key: event::KeyEvent) -> Option<AppAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.view = AppView::Main;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_setting = self.selected_setting.prev();
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.selected_setting = self.selected_setting.next();
                None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.decrease_setting();
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.increase_setting();
                None
            }
            KeyCode::Enter => {
                self.view = AppView::Main;
                None
            }
            _ => None,
        }
    }

    fn increase_setting(&mut self) {
        match self.selected_setting {
            SettingsField::PingCount => {
                self.settings.ping_count = (self.settings.ping_count + 5).min(100);
            }
            SettingsField::DownloadSize => {
                self.settings.download_size_mb = (self.settings.download_size_mb + 25).min(500);
            }
            SettingsField::UploadSize => {
                self.settings.upload_size_mb = (self.settings.upload_size_mb + 25).min(250);
            }
        }
    }

    fn decrease_setting(&mut self) {
        match self.selected_setting {
            SettingsField::PingCount => {
                self.settings.ping_count = self.settings.ping_count.saturating_sub(5).max(5);
            }
            SettingsField::DownloadSize => {
                self.settings.download_size_mb = self.settings.download_size_mb.saturating_sub(25).max(25);
            }
            SettingsField::UploadSize => {
                self.settings.upload_size_mb = self.settings.upload_size_mb.saturating_sub(25).max(25);
            }
        }
    }

    pub fn reset_for_new_test(&mut self) {
        self.phase = TestPhase::Idle;
        self.result = SpeedTestResult::default();
        self.download_progress = 0.0;
        self.upload_progress = 0.0;
        self.download_samples.clear();
        self.upload_samples.clear();
        self.ping_samples.clear();
        self.expanded = false;
    }

    pub fn update_ping_progress(&mut self, progress: PingProgress) {
        if let Some(ping) = progress.latest_ping {
            self.ping_samples.push(ping);
            // Keep last 100 samples
            if self.ping_samples.len() > 100 {
                self.ping_samples.remove(0);
            }
        }
    }

    pub fn update_download_progress(&mut self, progress: DownloadProgress) {
        self.download_progress = progress.downloaded_bytes as f64 / progress.total_bytes as f64;
        self.download_samples = progress.speed_samples;
    }

    pub fn update_upload_progress(&mut self, progress: UploadProgress) {
        self.upload_progress = progress.uploaded_bytes as f64 / progress.total_bytes as f64;
        self.upload_samples = progress.speed_samples;
    }

    pub fn complete_test(&mut self) {
        self.phase = TestPhase::Complete;
    }

    pub fn set_cancel_tx(&mut self, tx: mpsc::Sender<()>) {
        self.cancel_tx = Some(tx);
    }

    pub fn cancel_test(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.try_send(());
        }
        self.phase = TestPhase::Idle;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppAction {
    Quit,
    StartTest,
    CancelTest,
}

pub enum TestUpdate {
    PingProgress(PingProgress),
    PingComplete { avg_ms: f64, jitter_ms: f64 },
    DownloadProgress(DownloadProgress),
    DownloadComplete { speed_mbps: f64 },
    UploadProgress(UploadProgress),
    UploadComplete { speed_mbps: f64 },
}

pub async fn run_speed_test(
    update_tx: mpsc::Sender<TestUpdate>,
    mut cancel_rx: mpsc::Receiver<()>,
    settings: Settings,
) -> Result<()> {
    // Ping test
    let ping_count = settings.ping_count;
    let (ping_tx, mut ping_rx) = mpsc::channel::<PingProgress>(32);
    let ping_handle = tokio::spawn(async move {
        let mut test = PingTest::new(ping_count);
        test.run(ping_tx).await
    });

    while let Some(progress) = ping_rx.recv().await {
        if cancel_rx.try_recv().is_ok() {
            ping_handle.abort();
            return Ok(());
        }
        let _ = update_tx.send(TestUpdate::PingProgress(progress)).await;
    }

    let ping_result = ping_handle.await??;
    let _ = update_tx
        .send(TestUpdate::PingComplete {
            avg_ms: ping_result.avg_ms,
            jitter_ms: ping_result.jitter_ms,
        })
        .await;

    // Download test
    let download_size = settings.download_size_bytes();
    let (download_tx, mut download_rx) = mpsc::channel::<DownloadProgress>(32);
    let download_handle = tokio::spawn(async move {
        let mut test = DownloadTest::new(download_size);
        test.run(download_tx).await
    });

    while let Some(progress) = download_rx.recv().await {
        if cancel_rx.try_recv().is_ok() {
            download_handle.abort();
            return Ok(());
        }
        let _ = update_tx.send(TestUpdate::DownloadProgress(progress)).await;
    }

    let download_result = download_handle.await??;
    let _ = update_tx
        .send(TestUpdate::DownloadComplete {
            speed_mbps: download_result.avg_speed_mbps,
        })
        .await;

    // Upload test
    let upload_size = settings.upload_size_bytes();
    let (upload_tx, mut upload_rx) = mpsc::channel::<UploadProgress>(32);
    let upload_handle = tokio::spawn(async move {
        let mut test = UploadTest::new(upload_size);
        test.run(upload_tx).await
    });

    while let Some(progress) = upload_rx.recv().await {
        if cancel_rx.try_recv().is_ok() {
            upload_handle.abort();
            return Ok(());
        }
        let _ = update_tx.send(TestUpdate::UploadProgress(progress)).await;
    }

    let upload_result = upload_handle.await??;
    let _ = update_tx
        .send(TestUpdate::UploadComplete {
            speed_mbps: upload_result.avg_speed_mbps,
        })
        .await;

    Ok(())
}

pub fn poll_event(timeout: Duration) -> Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
