use std::{
    collections::BTreeMap,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::unbounded;
use indicatif::{ProgressBar, ProgressStyle};
use modkei_core::{FileResult, IgnoreOptions, Language, ScanOptions};

#[derive(Debug, Parser)]
#[command(version, about = "Explore code statistics and file dependencies.")]
struct Cli {
    /// Directory to analyze.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output HTML report path.
    #[arg(long, default_value = "modkei-report.html")]
    output: PathBuf,

    /// Skip opening the browser after scan.
    #[arg(long)]
    no_open: bool,

    /// Do not respect ignore files.
    #[arg(long)]
    no_ignore: bool,

    /// Do not respect ignore files in parent directories.
    #[arg(long)]
    no_ignore_parent: bool,

    /// Do not respect .ignore files.
    #[arg(long)]
    no_ignore_dot: bool,

    /// Do not respect VCS ignore files such as .gitignore.
    #[arg(long)]
    no_ignore_vcs: bool,
}

#[derive(Debug, Default)]
struct Totals {
    files: usize,
    lines: u64,
    code: u64,
    comments: u64,
    blanks: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli
        .path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", cli.path.display()))?;

    let options = ScanOptions {
        ignore: IgnoreOptions {
            no_ignore: cli.no_ignore,
            no_ignore_parent: cli.no_ignore_parent,
            no_ignore_dot: cli.no_ignore_dot,
            no_ignore_vcs: cli.no_ignore_vcs,
        },
    };

    let (tx, rx) = unbounded::<FileResult>();
    let progress = progress_bar();
    let started = Instant::now();
    let progress_thread = thread::spawn(move || {
        let mut files = 0usize;
        while rx.recv().is_ok() {
            files += 1;
            progress.set_message(format!("scanned {files} files"));
            progress.tick();
        }
        progress.finish_with_message(format!(
            "complete in {}",
            format_duration(started.elapsed())
        ));
        files
    });

    let scan = modkei_core::scan(&root, options, tx)?;
    let scanned_files = progress_thread
        .join()
        .map_err(|_| anyhow::anyhow!("progress thread panicked"))?;

    println!();
    print_stats(&scan.files);
    println!();
    println!(
        "Graph: {} nodes, {} edges",
        scan.graph.nodes.len(),
        scan.graph.edges.len()
    );

    modkei_report::generate(&scan.graph, &cli.output)?;
    println!("Report: {}", cli.output.display());

    if !cli.no_open {
        modkei_report::open_in_browser(&cli.output)?;
        println!("Opened report in browser.");
    }

    if scanned_files != scan.files.len() {
        eprintln!(
            "warning: progress saw {scanned_files} files, scan returned {} files",
            scan.files.len()
        );
    }

    Ok(())
}

fn progress_bar() -> ProgressBar {
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["-", "\\", "|", "/"]),
    );
    progress.enable_steady_tick(Duration::from_millis(90));
    progress.set_message("scanning files");
    progress
}

fn print_stats(files: &[FileResult]) {
    let stats = aggregate(files);
    println!(
        "{:<14} {:>8} {:>12} {:>12} {:>12} {:>12}",
        "Language", "Files", "Lines", "Code", "Comments", "Blanks"
    );
    println!("{}", "-".repeat(76));

    let mut rows = stats.iter().collect::<Vec<_>>();
    rows.sort_by(|(_, left), (_, right)| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| right.files.cmp(&left.files))
    });

    let mut total = Totals::default();
    for (language, row) in rows {
        print_row(language.name(), row);
        total.files += row.files;
        total.lines += row.lines;
        total.code += row.code;
        total.comments += row.comments;
        total.blanks += row.blanks;
    }
    println!("{}", "-".repeat(76));
    print_row("Total", &total);
}

fn aggregate(files: &[FileResult]) -> BTreeMap<Language, Totals> {
    let mut stats = BTreeMap::new();
    for file in files {
        let row: &mut Totals = stats.entry(file.language).or_default();
        row.files += 1;
        row.lines += file.lines;
        row.code += file.code;
        row.comments += file.comments;
        row.blanks += file.blanks;
    }
    stats
}

fn print_row(label: &str, row: &Totals) {
    println!(
        "{:<14} {:>8} {:>12} {:>12} {:>12} {:>12}",
        label, row.files, row.lines, row.code, row.comments, row.blanks
    );
}

fn format_duration(duration: Duration) -> String {
    format!("{:.2}s", duration.as_secs_f64())
}
