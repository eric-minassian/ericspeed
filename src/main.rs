mod app;
mod settings;
mod speedtest;
mod ui;

use anyhow::Result;
use app::{poll_event, run_speed_test, App, AppAction, TestUpdate};
use crossterm::event::Event;
use ratatui::DefaultTerminal;
use speedtest::TestPhase;
use std::time::Duration;
use tokio::sync::mpsc;
use ui::draw_ui;

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;

    let result = run_app(&mut terminal).await;

    ratatui::restore();
    result
}

async fn run_app(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    let mut test_rx: Option<mpsc::Receiver<TestUpdate>> = None;

    loop {
        terminal.draw(|frame| draw_ui(frame, &app))?;

        // Handle test updates
        if let Some(rx) = test_rx.as_mut() {
            match rx.try_recv() {
                Ok(update) => handle_update(&mut app, update),
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    if app.phase != TestPhase::Idle {
                        app.complete_test();
                    }
                    test_rx = None;
                }
            }
        }

        // Handle input
        if let Some(Event::Key(key)) = poll_event(Duration::from_millis(30))? {
            if let Some(action) = app.handle_key_event(key) {
                match action {
                    AppAction::Quit => break,
                    AppAction::StartTest => {
                        app.reset_for_new_test();
                        app.phase = TestPhase::Ping;

                        let (tx, rx) = mpsc::channel(32);
                        let (cancel_tx, cancel_rx) = mpsc::channel(1);

                        app.set_cancel_tx(cancel_tx);
                        test_rx = Some(rx);

                        let settings = app.settings.clone();
                        tokio::spawn(async move {
                            let _ = run_speed_test(tx, cancel_rx, settings).await;
                        });
                    }
                    AppAction::CancelTest => {
                        app.cancel_test();
                        test_rx = None;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_update(app: &mut App, update: TestUpdate) {
    match update {
        TestUpdate::PingProgress(p) => app.update_ping_progress(p),
        TestUpdate::PingComplete { avg_ms, jitter_ms } => {
            app.result.ping_ms = avg_ms;
            app.result.jitter_ms = jitter_ms;
            app.phase = TestPhase::Download;
        }
        TestUpdate::DownloadProgress(p) => app.update_download_progress(p),
        TestUpdate::DownloadComplete { speed_mbps } => {
            app.result.download_mbps = speed_mbps;
            app.phase = TestPhase::Upload;
        }
        TestUpdate::UploadProgress(p) => app.update_upload_progress(p),
        TestUpdate::UploadComplete { speed_mbps } => {
            app.result.upload_mbps = speed_mbps;
            app.complete_test();
        }
    }
}
