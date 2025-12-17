use crate::app::{App, AppView, Panel};
use crate::settings::SettingsField;
use crate::speedtest::TestPhase;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

// Color Palette - Elegant & Minimal
const ACCENT: Color = Color::Rgb(100, 149, 237);      // Cornflower blue
const SUCCESS: Color = Color::Rgb(134, 194, 156);     // Soft green
const SUCCESS_DIM: Color = Color::Rgb(80, 120, 90);
const INFO: Color = Color::Rgb(147, 180, 220);        // Soft blue
const INFO_DIM: Color = Color::Rgb(90, 110, 140);
const WARN: Color = Color::Rgb(220, 180, 130);        // Soft amber
const TEXT_PRIMARY: Color = Color::Rgb(230, 230, 230);
const TEXT_SECONDARY: Color = Color::Rgb(160, 160, 160);
const TEXT_MUTED: Color = Color::Rgb(100, 100, 100);
const BORDER: Color = Color::Rgb(60, 60, 65);
const BORDER_ACTIVE: Color = Color::Rgb(100, 100, 110);

pub fn draw_ui(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match app.view {
        AppView::Main => {
            if app.expanded {
                draw_expanded_view(frame, area, app);
            } else {
                draw_normal_view(frame, area, app);
            }
        }
        AppView::Settings => {
            draw_settings_view(frame, area, app);
        }
    }
}

fn draw_normal_view(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, chunks[0], app);

    let panels = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(chunks[1]);

    draw_download_panel(frame, panels[0], app, app.selected_panel == Panel::Download);
    draw_upload_panel(frame, panels[1], app, app.selected_panel == Panel::Upload);
    draw_ping_panel(frame, panels[2], app, app.selected_panel == Panel::Ping);

    draw_help(frame, chunks[2], app);
}

fn draw_expanded_view(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, chunks[0], app);

    match app.selected_panel {
        Panel::Download => draw_download_expanded(frame, chunks[1], app),
        Panel::Upload => draw_upload_expanded(frame, chunks[1], app),
        Panel::Ping => draw_ping_expanded(frame, chunks[1], app),
    }

    draw_help(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::horizontal([
        Constraint::Length(12),
        Constraint::Min(10),
        Constraint::Length(20),
    ])
    .split(inner);

    // Title
    let title = Paragraph::new("ericspeed")
        .style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    // Status
    let (status, color) = match app.phase {
        TestPhase::Idle => ("Ready", TEXT_MUTED),
        TestPhase::Ping => ("Measuring latency...", WARN),
        TestPhase::Download => ("Testing download...", SUCCESS),
        TestPhase::Upload => ("Testing upload...", INFO),
        TestPhase::Complete => ("Complete", ACCENT),
    };

    let status_text = Paragraph::new(status)
        .style(Style::default().fg(color))
        .alignment(Alignment::Center);
    frame.render_widget(status_text, chunks[1]);

    // Phase indicator
    let phase_text = create_phase_text(app.phase);
    frame.render_widget(
        Paragraph::new(phase_text).alignment(Alignment::Right),
        chunks[2],
    );
}

fn create_phase_text(phase: TestPhase) -> Line<'static> {
    let phases = [
        (TestPhase::Ping, "ping"),
        (TestPhase::Download, "down"),
        (TestPhase::Upload, "up"),
    ];

    let mut spans = Vec::new();

    for (i, (p, label)) in phases.iter().enumerate() {
        let is_active = phase == *p;
        let is_complete = match phase {
            TestPhase::Download => *p == TestPhase::Ping,
            TestPhase::Upload => *p == TestPhase::Ping || *p == TestPhase::Download,
            TestPhase::Complete => true,
            _ => false,
        };

        let style = if is_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else if is_complete {
            Style::default().fg(TEXT_SECONDARY)
        } else {
            Style::default().fg(TEXT_MUTED)
        };

        spans.push(Span::styled(*label, style));

        if i < phases.len() - 1 {
            spans.push(Span::styled(" / ", Style::default().fg(TEXT_MUTED)));
        }
    }

    Line::from(spans)
}

// Panels
fn draw_download_panel(frame: &mut Frame, area: Rect, app: &App, selected: bool) {
    draw_metric_panel(
        frame,
        area,
        "Download",
        SUCCESS,
        SUCCESS_DIM,
        selected,
        get_current_download_speed(app),
        calculate_download_progress(app),
        &app.download_samples,
    );
}

fn draw_upload_panel(frame: &mut Frame, area: Rect, app: &App, selected: bool) {
    draw_metric_panel(
        frame,
        area,
        "Upload",
        INFO,
        INFO_DIM,
        selected,
        get_current_upload_speed(app),
        calculate_upload_progress(app),
        &app.upload_samples,
    );
}

fn draw_ping_panel(frame: &mut Frame, area: Rect, app: &App, selected: bool) {
    let border_color = if selected { BORDER_ACTIVE } else { BORDER };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Latency ",
            Style::default().fg(if selected { WARN } else { TEXT_SECONDARY }),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(inner);

    // Value
    let ping = get_current_ping(app);
    let value = if ping > 0.0 {
        format!("{:.0} ms", ping)
    } else {
        "—".to_string()
    };

    frame.render_widget(
        Paragraph::new(value)
            .style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Jitter
    let jitter = if app.result.jitter_ms > 0.0 {
        format!("jitter {:.1} ms", app.result.jitter_ms)
    } else {
        "jitter —".to_string()
    };
    frame.render_widget(
        Paragraph::new(jitter)
            .style(Style::default().fg(TEXT_MUTED))
            .alignment(Alignment::Center),
        chunks[1],
    );

    // Chart
    if !app.ping_samples.is_empty() {
        draw_sparkline(frame, chunks[2], &app.ping_samples, WARN);
    }
}

fn draw_metric_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    color: Color,
    dim_color: Color,
    selected: bool,
    speed: f64,
    progress: f64,
    samples: &[f64],
) {
    let border_color = if selected { BORDER_ACTIVE } else { BORDER };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(if selected { color } else { TEXT_SECONDARY }),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(inner);

    // Speed value
    let speed_text = format_speed(speed);
    frame.render_widget(
        Paragraph::new(speed_text)
            .style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Progress bar
    draw_progress_bar(frame, chunks[1], progress, color, dim_color);

    // Chart
    if !samples.is_empty() {
        draw_sparkline(frame, chunks[2], samples, color);
    }
}

fn draw_progress_bar(frame: &mut Frame, area: Rect, ratio: f64, color: Color, dim_color: Color) {
    if area.width < 4 {
        return;
    }

    let width = (area.width - 2) as usize;
    let filled = ((ratio * width as f64) as usize).min(width);
    let empty = width.saturating_sub(filled);

    let bar = Line::from(vec![
        Span::raw(" "),
        Span::styled("━".repeat(filled), Style::default().fg(color)),
        Span::styled("━".repeat(empty), Style::default().fg(dim_color)),
        Span::raw(" "),
    ]);

    frame.render_widget(Paragraph::new(bar), area);
}

fn draw_sparkline(frame: &mut Frame, area: Rect, data: &[f64], color: Color) {
    if data.is_empty() || area.width < 4 || area.height < 2 {
        return;
    }

    let (min_val, max_val) = get_data_range(data);
    let range = (max_val - min_val).max(1.0);

    let points: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(&points);

    let chart = Chart::new(vec![dataset])
        .x_axis(Axis::default().bounds([0.0, data.len() as f64]))
        .y_axis(Axis::default().bounds([min_val - range * 0.1, max_val + range * 0.1]));

    frame.render_widget(chart, area);
}

// Expanded views
fn draw_download_expanded(frame: &mut Frame, area: Rect, app: &App) {
    draw_expanded_metric(
        frame,
        area,
        "Download",
        SUCCESS,
        SUCCESS_DIM,
        get_current_download_speed(app),
        calculate_download_progress(app),
        &app.download_samples,
        "Mbps",
    );
}

fn draw_upload_expanded(frame: &mut Frame, area: Rect, app: &App) {
    draw_expanded_metric(
        frame,
        area,
        "Upload",
        INFO,
        INFO_DIM,
        get_current_upload_speed(app),
        calculate_upload_progress(app),
        &app.upload_samples,
        "Mbps",
    );
}

fn draw_ping_expanded(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .title(Span::styled(" Latency ", Style::default().fg(WARN)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(4),
    ])
    .split(inner);

    // Stats
    let ping = get_current_ping(app);
    let (avg, max, min) = get_stats(&app.ping_samples);
    let jitter = if app.result.jitter_ms > 0.0 {
        format!("{:.1}", app.result.jitter_ms)
    } else {
        "—".to_string()
    };

    let stats = Line::from(vec![
        Span::styled(format!("{:.0} ms", ping), Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("jitter {} ms", jitter), Style::default().fg(TEXT_SECONDARY)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("avg {:.0}", avg), Style::default().fg(TEXT_MUTED)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("max {:.0}", max), Style::default().fg(TEXT_MUTED)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("min {:.0}", min), Style::default().fg(TEXT_MUTED)),
    ]);
    frame.render_widget(Paragraph::new(stats).alignment(Alignment::Center), chunks[0]);

    draw_detailed_chart(frame, chunks[1], &app.ping_samples, WARN, "ms");
}

fn draw_expanded_metric(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    color: Color,
    dim_color: Color,
    speed: f64,
    progress: f64,
    samples: &[f64],
    unit: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(color)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(4),
    ])
    .split(inner);

    // Stats line
    let (avg, max, min) = get_stats(samples);
    let stats = Line::from(vec![
        Span::styled(format_speed(speed), Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("avg {}", format_speed(avg)), Style::default().fg(TEXT_MUTED)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("max {}", format_speed(max)), Style::default().fg(TEXT_MUTED)),
        Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("min {}", format_speed(min)), Style::default().fg(TEXT_MUTED)),
    ]);
    frame.render_widget(Paragraph::new(stats).alignment(Alignment::Center), chunks[0]);

    // Progress
    draw_progress_bar(frame, chunks[1], progress, color, dim_color);

    // Chart
    draw_detailed_chart(frame, chunks[2], samples, color, unit);
}

fn draw_detailed_chart(frame: &mut Frame, area: Rect, data: &[f64], color: Color, unit: &str) {
    if data.is_empty() || area.width < 10 || area.height < 3 {
        return;
    }

    let (min_val, max_val) = get_data_range(data);
    let range = (max_val - min_val).max(0.1);
    let y_min = (min_val - range * 0.1).max(0.0);
    let y_max = max_val + range * 0.1;

    let points: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    let avg = if !data.is_empty() { data.iter().sum::<f64>() / data.len() as f64 } else { 0.0 };
    let avg_line: Vec<(f64, f64)> = vec![(0.0, avg), (data.len() as f64, avg)];

    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color))
            .data(&points),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(TEXT_MUTED))
            .data(&avg_line),
    ];

    let y_labels = vec![
        Span::styled(format!("{:.0}", y_min), Style::default().fg(TEXT_MUTED)),
        Span::styled(format!("{:.0} {}", y_max, unit), Style::default().fg(TEXT_MUTED)),
    ];

    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(BORDER))
                .bounds([0.0, data.len() as f64]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(BORDER))
                .bounds([y_min, y_max])
                .labels(y_labels),
        );

    frame.render_widget(chart, area);
}

// Settings
fn draw_settings_view(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(1),
    ])
    .split(area);

    // Header
    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER));
    let header_inner = header_block.inner(chunks[0]);
    frame.render_widget(header_block, chunks[0]);

    frame.render_widget(
        Paragraph::new("Settings")
            .style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        header_inner,
    );

    // Settings content
    let content_area = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Min(30),
        Constraint::Length(2),
    ])
    .split(chunks[1])[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));
    let inner = block.inner(content_area);
    frame.render_widget(block, content_area);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    draw_setting_row(
        frame,
        rows[0],
        "Ping samples",
        &format!("{}", app.settings.ping_count),
        app.selected_setting == SettingsField::PingCount,
    );

    draw_setting_row(
        frame,
        rows[1],
        "Download size",
        &format!("{} MB", app.settings.download_size_mb),
        app.selected_setting == SettingsField::DownloadSize,
    );

    draw_setting_row(
        frame,
        rows[2],
        "Upload size",
        &format!("{} MB", app.settings.upload_size_mb),
        app.selected_setting == SettingsField::UploadSize,
    );

    // Help
    let help = "↑↓ select · ←→ adjust · enter done";
    frame.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(TEXT_MUTED))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_setting_row(frame: &mut Frame, area: Rect, label: &str, value: &str, selected: bool) {
    let chunks = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Min(10),
    ])
    .split(area);

    let label_style = if selected {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(TEXT_SECONDARY)
    };

    frame.render_widget(
        Paragraph::new(format!(" {}", label)).style(label_style),
        chunks[0],
    );

    let value_text = if selected {
        format!("< {} >", value)
    } else {
        value.to_string()
    };

    let value_style = if selected {
        Style::default().fg(TEXT_PRIMARY)
    } else {
        Style::default().fg(TEXT_MUTED)
    };

    frame.render_widget(Paragraph::new(value_text).style(value_style), chunks[1]);
}

fn draw_help(frame: &mut Frame, area: Rect, app: &App) {
    let help = if app.expanded {
        "esc close · q quit"
    } else {
        match app.phase {
            TestPhase::Idle | TestPhase::Complete => "enter start · s settings · tab select · space expand · q quit",
            _ => "tab select · space expand · esc cancel · q quit",
        }
    };

    frame.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(TEXT_MUTED))
            .alignment(Alignment::Center),
        area,
    );
}

// Helpers
fn get_current_download_speed(app: &App) -> f64 {
    if app.result.download_mbps > 0.0 {
        app.result.download_mbps
    } else {
        app.download_samples.last().copied().unwrap_or(0.0)
    }
}

fn get_current_upload_speed(app: &App) -> f64 {
    if app.result.upload_mbps > 0.0 {
        app.result.upload_mbps
    } else {
        app.upload_samples.last().copied().unwrap_or(0.0)
    }
}

fn get_current_ping(app: &App) -> f64 {
    if app.result.ping_ms > 0.0 {
        app.result.ping_ms
    } else {
        app.ping_samples.last().copied().unwrap_or(0.0)
    }
}

fn get_data_range(data: &[f64]) -> (f64, f64) {
    let min = data.iter().cloned().fold(f64::MAX, f64::min);
    let max = data.iter().cloned().fold(f64::MIN, f64::max);
    (if min == f64::MAX { 0.0 } else { min }, if max == f64::MIN { 0.0 } else { max })
}

fn get_stats(data: &[f64]) -> (f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let avg = data.iter().sum::<f64>() / data.len() as f64;
    let max = data.iter().cloned().fold(f64::MIN, f64::max);
    let min = data.iter().cloned().fold(f64::MAX, f64::min);
    (avg, max, if min == f64::MAX { 0.0 } else { min })
}

fn calculate_download_progress(app: &App) -> f64 {
    match app.phase {
        TestPhase::Download => app.download_progress,
        TestPhase::Upload | TestPhase::Complete => 1.0,
        _ => 0.0,
    }
}

fn calculate_upload_progress(app: &App) -> f64 {
    match app.phase {
        TestPhase::Upload => app.upload_progress,
        TestPhase::Complete => 1.0,
        _ => 0.0,
    }
}

fn format_speed(mbps: f64) -> String {
    if mbps >= 1000.0 {
        format!("{:.1} Gbps", mbps / 1000.0)
    } else if mbps >= 1.0 {
        format!("{:.1} Mbps", mbps)
    } else if mbps > 0.0 {
        format!("{:.0} Kbps", mbps * 1000.0)
    } else {
        "—".to_string()
    }
}
