use std::{
    collections::BTreeMap,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossbeam_channel::{Receiver, TryRecvError};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use modkei_core::{FileResult, GraphData, Language, ScanOutput};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
};

#[derive(Debug, Clone, Default)]
pub struct LangStats {
    pub files: usize,
    pub lines: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
}

#[derive(Default)]
pub struct App {
    pub stats: BTreeMap<Language, LangStats>,
    pub total_files: usize,
    pub scanned_files: usize,
    pub done: bool,
    graph: Option<GraphData>,
    output: Option<PathBuf>,
    opened: bool,
    error: Option<String>,
    start: Option<Instant>,
    completed_elapsed: Option<Duration>,
}

pub fn run(
    rx: Receiver<FileResult>,
    done_rx: Receiver<Result<ScanOutput>>,
    output: PathBuf,
    no_open: bool,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_inner(&mut terminal, rx, done_rx, output, no_open);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: Receiver<FileResult>,
    done_rx: Receiver<Result<ScanOutput>>,
    output: PathBuf,
    no_open: bool,
) -> Result<()> {
    let mut app = App {
        start: Some(Instant::now()),
        ..App::default()
    };

    loop {
        receive_files(&mut app, &rx);
        receive_done(&mut app, &done_rx, &output, no_open);
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('o') | KeyCode::Char('g') if app.done => {
                        if let Some(path) = &app.output {
                            if let Err(err) = modkei_report::open_in_browser(path) {
                                app.error = Some(err.to_string());
                            } else {
                                app.opened = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn receive_files(app: &mut App, rx: &Receiver<FileResult>) {
    loop {
        match rx.try_recv() {
            Ok(file) => {
                app.scanned_files += 1;
                let stats = app.stats.entry(file.language).or_default();
                stats.files += 1;
                stats.lines += file.lines;
                stats.code += file.code;
                stats.comments += file.comments;
                stats.blanks += file.blanks;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
}

fn receive_done(
    app: &mut App,
    done_rx: &Receiver<Result<ScanOutput>>,
    output: &PathBuf,
    no_open: bool,
) {
    if app.done {
        return;
    }
    if let Ok(result) = done_rx.try_recv() {
        app.done = true;
        app.completed_elapsed = app.start.map(|start| start.elapsed());
        match result {
            Ok(scan) => {
                app.total_files = scan.files.len();
                app.graph = Some(scan.graph.clone());
                match modkei_report::generate(&scan.graph, output) {
                    Ok(()) => {
                        app.output = Some(output.clone());
                        if !no_open {
                            match modkei_report::open_in_browser(output) {
                                Ok(()) => app.opened = true,
                                Err(err) => app.error = Some(err.to_string()),
                            }
                        }
                    }
                    Err(err) => app.error = Some(err.to_string()),
                }
            }
            Err(err) => app.error = Some(err.to_string()),
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let [header, table_area, footer] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(6),
    ])
    .areas(frame.area());

    let elapsed = app
        .completed_elapsed
        .or_else(|| app.start.map(|start| start.elapsed()))
        .unwrap_or_default();
    let elapsed_label = format_duration(elapsed);
    let label = if app.done {
        format!("complete in {elapsed_label}")
    } else {
        format!("{} files scanned, {elapsed_label}", app.scanned_files)
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" modkei "))
        .gauge_style(Style::default().fg(if app.done { Color::Green } else { Color::Cyan }))
        .ratio(if app.done { 1.0 } else { 0.35 })
        .label(label);
    frame.render_widget(gauge, header);

    let rows = app.stats.iter().map(|(lang, stats)| {
        Row::new([
            lang.name().to_string(),
            stats.files.to_string(),
            stats.lines.to_string(),
            stats.code.to_string(),
            stats.comments.to_string(),
            stats.blanks.to_string(),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["Language", "Files", "Lines", "Code", "Comments", "Blanks"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title(" Statistics "))
    .column_spacing(2);
    frame.render_widget(table, table_area);

    let total = app
        .stats
        .values()
        .fold(LangStats::default(), |mut acc, stats| {
            acc.files += stats.files;
            acc.lines += stats.lines;
            acc.code += stats.code;
            acc.comments += stats.comments;
            acc.blanks += stats.blanks;
            acc
        });
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Total ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} files, {} lines, {} code, {} comments, {} blanks",
                total.files, total.lines, total.code, total.comments, total.blanks
            )),
        ]),
        Line::from(if app.done {
            "Press o/g to open graph, q/Esc to quit."
        } else {
            "Scanning in parallel. Press q/Esc to quit."
        }),
    ];
    if let Some(output) = &app.output {
        lines.push(Line::from(format!(
            "HTML graph: {}{}",
            output.display(),
            if app.opened { " (opened)" } else { "" }
        )));
    }
    if let Some(error) = &app.error {
        lines.push(Line::from(Span::styled(
            format!("Error: {error}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Status ")),
        footer,
    );
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() < 10 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}s", duration.as_secs())
    }
}
