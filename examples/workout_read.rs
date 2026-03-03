use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use csafe::{Client, Command, CommandResponse};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};
use std::{collections::VecDeque, io::stdout, sync::Mutex, time::Duration};
use tokio::sync::{mpsc, watch};
use tokio_serial::SerialPortBuilderExt;

const PORT_NAME: &str = "/dev/ttyUSB0";
const BAUD_RATE: u32 = 9600;
const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;
const MAX_LOG_LINES: usize = 200;

// ── In-memory logger ──────────────────────────────────────────────────────────

static LOG_BUFFER: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

struct TuiLogger;

impl log::Log for TuiLogger {
    fn enabled(&self, _meta: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if let Ok(mut buf) = LOG_BUFFER.lock() {
            buf.push_back(format!("[{}] {}", record.level(), record.args()));
            while buf.len() > MAX_LOG_LINES {
                buf.pop_front();
            }
        }
    }

    fn flush(&self) {}
}

fn drain_logs() -> Vec<String> {
    LOG_BUFFER
        .lock()
        .map(|buf| buf.iter().cloned().collect())
        .unwrap_or_default()
}

fn clear_logs() {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.clear();
    }
}

// ── CLI args ──────────────────────────────────────────────────────────────────

fn parse_poll_interval() -> Duration {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--interval" || args[i] == "-i" {
            if let Some(val) = args.get(i + 1) {
                if let Ok(ms) = val.parse::<u64>() {
                    return Duration::from_millis(ms);
                }
                eprintln!("Invalid interval value '{}', using default", val);
            }
        }
    }
    Duration::from_millis(DEFAULT_POLL_INTERVAL_MS)
}

// ── App State ─────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct WorkoutStats {
    speed_raw: i16,
    speed_unit: String,
    speed_value: f64,

    grade_raw: i16,
    grade_unit: String,
    grade_value: f64,

    hours: u8,
    minutes: u8,
    seconds: u8,
}

// ── Commands from TUI → Poller ────────────────────────────────────────────────

enum PollerCmd {
    ForcePoll,
    Shutdown,
}

// ── Polling task ──────────────────────────────────────────────────────────────

async fn poll_device<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    client: &mut Client<T>,
    stats: &mut WorkoutStats,
) -> anyhow::Result<()> {
    if let Some(CommandResponse::GetSpeed { unit, speed }) = client
        .send_command_async(Command::GetSpeed)
        .await?
        .into_iter()
        .next()
    {
        stats.speed_value = speed as f64;
        stats.speed_unit = unit.to_string();
        stats.speed_raw = speed;
    } else {
        log::debug!("GetSpeed: status-only response (no data)");
    }

    // if let Some(v) = client.send_command_async(Command::GetGrade).await? {
    //     let (val, unit) = v.display();
    //     stats.grade_value = val;
    //     stats.grade_unit = unit.to_string();
    //     stats.grade_raw = v.raw;
    // } else {
    //     log::debug!("GetGrade: status-only response (no data)");
    // }

    // if let Some(t) = client.send_command_async(Command::GetTWork).await? {
    //     stats.hours = t.hours;
    //     stats.minutes = t.minutes;
    //     stats.seconds = t.seconds;
    // } else {
    //     log::debug!("GetTWork: status-only response (no data)");
    // }

    Ok(())
}

async fn poller_task<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    mut client: Client<T>,
    stats_tx: watch::Sender<WorkoutStats>,
    mut cmd_rx: mpsc::Receiver<PollerCmd>,
    poll_interval: Duration,
) {
    let mut stats = WorkoutStats::default();
    let mut interval = tokio::time::interval(poll_interval);
    // First tick completes immediately, giving us an instant initial poll
    interval.tick().await;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = poll_device(&mut client, &mut stats).await {
                    log::error!("Poll failed: {e}");
                }
                let _ = stats_tx.send(stats.clone());
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(PollerCmd::ForcePoll) => {
                        if let Err(e) = poll_device(&mut client, &mut stats).await {
                            log::error!("Poll failed: {e}");
                        }
                        let _ = stats_tx.send(stats.clone());
                        interval.reset();
                    }
                    Some(PollerCmd::Shutdown) | None => break,
                }
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(frame: &mut Frame, stats: &WorkoutStats, poll_interval: &Duration) {
    let area = frame.area();

    // Outer background
    let bg = Block::default().style(Style::default().bg(Color::Rgb(10, 12, 20)));
    frame.render_widget(bg, area);

    // Split into top (stats card) and bottom (log panel)
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(16), // stats card
            Constraint::Min(6),     // log panel
        ])
        .split(area);

    // ── Stats card (centred horizontally) ─────────────────────────────────
    let card_width = 52u16;
    let card_area_full = main_layout[0];
    let card_x = card_area_full.width.saturating_sub(card_width) / 2;
    let card_area = Rect::new(
        card_x,
        card_area_full.y,
        card_width.min(card_area_full.width),
        card_area_full.height,
    );

    let title_text = format!("CSAFE Workout Monitor ({}ms)", poll_interval.as_millis());
    let card = Block::default()
        .title(Line::from(vec![
            Span::raw(" 🏃 "),
            Span::styled(
                title_text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(70, 130, 180)))
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(Color::Rgb(16, 20, 36)));
    let inner = card.inner(card_area);
    frame.render_widget(card, card_area);

    // Split inner into rows
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // speed
            Constraint::Length(3), // grade
            Constraint::Length(3), // duration
            Constraint::Min(0),    // spacer
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // ── Speed ──────────────────────────────────────────────────────────────
    let speed_label = Paragraph::new(vec![
        Line::from(Span::styled(
            "  SPEED",
            Style::default()
                .fg(Color::Rgb(150, 150, 180))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("  {:.2}", stats.speed_value),
                Style::default()
                    .fg(Color::Rgb(80, 220, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", stats.speed_unit),
                Style::default().fg(Color::Rgb(100, 180, 220)),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(35, 40, 70))),
    );
    frame.render_widget(speed_label, rows[0]);

    // ── Incline ────────────────────────────────────────────────────────────
    let grade_label = Paragraph::new(vec![
        Line::from(Span::styled(
            "  INCLINE",
            Style::default()
                .fg(Color::Rgb(150, 150, 180))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("  {:.2}", stats.grade_value),
                Style::default()
                    .fg(Color::Rgb(80, 255, 160))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", stats.grade_unit),
                Style::default().fg(Color::Rgb(80, 200, 130)),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(35, 40, 70))),
    );
    frame.render_widget(grade_label, rows[1]);

    // ── Duration ───────────────────────────────────────────────────────────
    let duration_label = Paragraph::new(vec![
        Line::from(Span::styled(
            "  DURATION",
            Style::default()
                .fg(Color::Rgb(150, 150, 180))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "  {:02}:{:02}:{:02}",
                stats.hours, stats.minutes, stats.seconds
            ),
            Style::default()
                .fg(Color::Rgb(255, 180, 80))
                .add_modifier(Modifier::BOLD),
        )),
    ]);
    frame.render_widget(duration_label, rows[2]);

    // ── Hint ───────────────────────────────────────────────────────────────
    let dim = Style::default().fg(Color::Rgb(100, 100, 130));
    let key = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("q", key),
        Span::styled("/", dim),
        Span::styled("Esc", key),
        Span::styled(" quit  ", dim),
        Span::styled("Space", key),
        Span::styled(" poll  ", dim),
        Span::styled("c", key),
        Span::styled(" clear log", dim),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hint, rows[4]);

    // ── Log panel ──────────────────────────────────────────────────────────
    let log_area = main_layout[1];
    let log_block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                "Protocol Log",
                Style::default()
                    .fg(Color::Rgb(180, 140, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 50, 100)))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(Color::Rgb(12, 14, 24)));

    let log_inner = log_block.inner(log_area);
    frame.render_widget(log_block, log_area);

    let logs = drain_logs();
    let visible_height = log_inner.height as usize;
    let skip = logs.len().saturating_sub(visible_height);
    let log_lines: Vec<Line> = logs
        .iter()
        .skip(skip)
        .map(|entry| {
            let color = if entry.starts_with("[ERROR") {
                Color::Rgb(255, 80, 80)
            } else if entry.contains("TX:") {
                Color::Rgb(100, 200, 255)
            } else if entry.contains("RX:") {
                Color::Rgb(120, 255, 160)
            } else {
                Color::Rgb(130, 130, 150)
            };
            Line::from(Span::styled(entry.as_str(), Style::default().fg(color)))
        })
        .collect();

    let log_paragraph = Paragraph::new(log_lines);
    frame.render_widget(log_paragraph, log_inner);
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let poll_interval = parse_poll_interval();

    // Install in-memory logger
    log::set_logger(&TuiLogger).ok();
    log::set_max_level(log::LevelFilter::Debug);

    let device = tokio_serial::new(PORT_NAME, BAUD_RATE)
        .timeout(Duration::from_secs(2))
        .open_native_async()
        .context("Failed to open serial port. Is the device connected?")?;

    let client = Client::new(device);

    // Channels between TUI ↔ poller
    let (stats_tx, stats_rx) = watch::channel(WorkoutStats::default());
    let (cmd_tx, cmd_rx) = mpsc::channel::<PollerCmd>(8);

    // Spawn background poller
    tokio::spawn(poller_task(client, stats_tx, cmd_rx, poll_interval));

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    'mainloop: loop {
        // ── Read latest stats (non-blocking) ──────────────────────────────
        let stats = stats_rx.borrow().clone();

        // ── Render ────────────────────────────────────────────────────────
        terminal.draw(|frame| render(frame, &stats, &poll_interval))?;

        // ── Input (non-blocking, 100ms timeout) ───────────────────────────
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            let _ = cmd_tx.send(PollerCmd::Shutdown).await;
                            break 'mainloop;
                        }
                        KeyCode::Char(' ') => {
                            let _ = cmd_tx.send(PollerCmd::ForcePoll).await;
                        }
                        KeyCode::Char('c') => clear_logs(),
                        _ => {}
                    }
                }
            }
        }
    }

    // ── Cleanup ───────────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
